use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use log::{debug, info, trace};
use reqwest::{blocking::Client, Certificate};
use sha2::{Digest, Sha256};

use crate::{
    api::{CertsInfo, ImageHandler, UpgradeRequest},
    sys_mgmt::{CERTS_PATH, IMAGE_PERMISSION, PERSIST_DIR},
    utils::*,
};

const BUFFER: u64 = 1024 * 1024 * 10;

pub struct DiskImageHandler<T: CommandExecutor> {
    pub paths: PreparePath,
    pub executor: T,
    pub certs_path: String,
}

impl<T: CommandExecutor> ImageHandler<T> for DiskImageHandler<T> {
    fn download_image(&self, req: &UpgradeRequest) -> Result<UpgradeImageManager<T>> {
        self.download(req)?;
        self.checksum_match(self.paths.image_path.to_str().unwrap_or_default(), &req.check_sum)?;
        let (_, next_partition_info) = get_partition_info(&self.executor)?;
        let img_manager = UpgradeImageManager::new(self.paths.clone(), next_partition_info, self.executor.clone());
        Ok(img_manager)
    }
}

impl Default for DiskImageHandler<RealCommandExecutor> {
    fn default() -> Self {
        Self { paths: PreparePath::default(), executor: RealCommandExecutor {}, certs_path: CERTS_PATH.to_string() }
    }
}

impl<T: CommandExecutor> DiskImageHandler<T> {
    #[cfg(test)]
    fn new(paths: PreparePath, executor: T, certs_path: String) -> Self {
        Self { paths, executor, certs_path }
    }

    fn download(&self, req: &UpgradeRequest) -> Result<()> {
        let mut resp = self.send_download_request(req)?;
        if resp.status() != reqwest::StatusCode::OK {
            bail!("Failed to download image from {}, status: {}", req.image_url, resp.status());
        }
        debug!("Received response body size: {:?}", resp.content_length().unwrap_or_default());
        let need_bytes = resp.content_length().unwrap_or_default() + BUFFER;

        check_disk_size(
            i64::try_from(need_bytes).with_context(|| "Failed to transform content length from u64 to i64")?,
            self.paths.image_path.parent().unwrap_or_else(|| Path::new(PERSIST_DIR)),
        )?;

        let mut out = fs::File::create(&self.paths.image_path)?;
        trace!("Start to save upgrade image to path {}", &self.paths.image_path.display());
        out.set_permissions(fs::Permissions::from_mode(IMAGE_PERMISSION))?;
        let bytes = resp.copy_to(&mut out)?;
        info!(
            "Download image successfully, upgrade image path: {}, write bytes: {}",
            &self.paths.image_path.display(),
            bytes
        );
        Ok(())
    }

    fn checksum_match(&self, file_path: &str, check_sum: &str) -> Result<()> {
        trace!("Start to check checksum");
        let file = fs::read(file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(file);
        let hash = hasher.finalize();
        // sha256sum -b /persist/update.img
        let cal_sum = format!("{:X}", hash);
        if cal_sum.to_lowercase() != check_sum.to_lowercase() {
            delete_file_or_dir(file_path)?;
            bail!("Checksum {} mismatch to {}", cal_sum, check_sum);
        }
        debug!("Checksum match");
        Ok(())
    }

    fn send_download_request(&self, req: &UpgradeRequest) -> Result<reqwest::blocking::Response> {
        let client: Client;

        if !req.image_url.starts_with("https://") {
            // http request
            if !req.flag_safe {
                bail!("The upgrade image url is not safe");
            }
            info!("Discover http request to: {}", &req.image_url);
            client = Client::new();
        } else if req.mtls {
            // https mtls request
            client = self.load_ca_client_certs(&req.certs).with_context(|| "Failed to load client certificates")?;
            info!("Discover https mtls request to: {}", &req.image_url);
        } else {
            // https request
            client = self.load_ca_certs(&req.certs.ca_cert).with_context(|| "Failed to load CA certificates")?;
            info!("Discover https request to: {}", &req.image_url);
        }

        client.get(&req.image_url).send().with_context(|| format!("Failed to fetch from URL: {}", &req.image_url))
    }

    fn load_ca_certs(&self, ca_cert: &str) -> Result<Client> {
        trace!("Start to load CA certificates");
        self.cert_exist(ca_cert)?;
        let ca = Certificate::from_pem(&std::fs::read(self.get_certs_path(ca_cert))?)?;
        let client = Client::builder().add_root_certificate(ca).build()?;
        Ok(client)
    }

    fn load_ca_client_certs(&self, certs: &CertsInfo) -> Result<Client> {
        trace!("Start to load CA and client certificates");
        self.cert_exist(&certs.ca_cert)?;
        let ca = Certificate::from_pem(&std::fs::read(self.get_certs_path(&certs.ca_cert))?)?;

        self.cert_exist(&certs.client_cert)?;
        self.cert_exist(&certs.client_key)?;
        let client_cert = std::fs::read(self.get_certs_path(&certs.client_cert))?;
        let client_key = std::fs::read(self.get_certs_path(&certs.client_key))?;
        let mut client_identity = Vec::new();
        client_identity.extend_from_slice(&client_cert);
        client_identity.extend_from_slice(&client_key);
        let client_id = reqwest::Identity::from_pem(&client_identity)?;

        let client = Client::builder().use_rustls_tls().add_root_certificate(ca).identity(client_id).build()?;
        Ok(client)
    }

    fn cert_exist(&self, cert_file: &str) -> Result<()> {
        if cert_file.is_empty() {
            bail!("Please provide the certificate");
        }
        if !self.get_certs_path(cert_file).exists() {
            bail!("Certificate does not exist: {}", cert_file);
        }
        Ok(())
    }

    fn get_certs_path(&self, cert: &str) -> PathBuf {
        let cert_path = format!("{}{}", self.certs_path, cert);
        PathBuf::from(cert_path)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::NamedTempFile;

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_get_certs_path() {
        init();
        let handler = DiskImageHandler::<RealCommandExecutor>::default();
        let certs_path = handler.get_certs_path("ca.pem");
        assert_eq!(certs_path.to_str().unwrap(), "/etc/KubeOS/certs/ca.pem");
    }

    #[test]
    fn test_cert_exist() {
        init();
        // generate tmp file
        let tmp_file = NamedTempFile::new().unwrap();
        let handler =
            DiskImageHandler::<RealCommandExecutor>::new(PreparePath::default(), RealCommandExecutor {}, String::new());
        let res = handler.cert_exist(tmp_file.path().to_str().unwrap());
        assert!(res.is_ok());

        assert!(handler.cert_exist("aaa.pem").is_err())
    }

    #[test]
    #[ignore]
    fn test_send_download_request() {
        init();
        // http
        let handler = DiskImageHandler::<RealCommandExecutor>::default();
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "1327e27d600538354d93bd68cce86566dd089e240c126dc3019cafabdc65aa02".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: "http://localhost:8080/aaa.txt".to_string(),
            flag_safe: true,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        let res = handler.send_download_request(&req);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().text().unwrap(), "This is a test txt file generated by yuhang wei\n");

        // https
        let mut handler = DiskImageHandler::<RealCommandExecutor>::default();
        handler.certs_path = "/home/yuhang/Documents/data/https-nginx/nginx/certs/".to_string();
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "1327e27d600538354d93bd68cce86566dd089e240c126dc3019cafabdc65aa02".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: "https://7.250.142.47:8081/aaa.txt".to_string(),
            flag_safe: true,
            mtls: false,
            certs: CertsInfo {
                ca_cert: "nginx.crt".to_string(),
                client_cert: "".to_string(),
                client_key: "".to_string(),
            },
        };
        let res = handler.send_download_request(&req);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().text().unwrap(), "This is a test txt file generated by yuhang wei\n");

        // mtls
        let mut handler = DiskImageHandler::<RealCommandExecutor>::default();
        handler.certs_path = "/home/yuhang/Documents/data/cert/".to_string();
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "1327e27d600538354d93bd68cce86566dd089e240c126dc3019cafabdc65aa02".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: "https://7.250.142.47:8082/aaa.txt".to_string(),
            flag_safe: true,
            mtls: true,
            certs: CertsInfo {
                ca_cert: "nginx.crt".to_string(),
                client_cert: "client.crt".to_string(),
                client_key: "client.key".to_string(),
            },
        };
        let res = handler.send_download_request(&req);
        assert!(res.is_ok());
        assert_eq!(res.unwrap().text().unwrap(), "This is a test txt file generated by yuhang wei\n");
    }

    #[test]
    #[ignore]
    fn test_download() {
        init();
        let mut handler = DiskImageHandler::<RealCommandExecutor>::default();
        handler.paths.image_path = PathBuf::from("/home/yuhang/Documents/KubeOS/KubeOS-Rust/test_download_image");
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "90da5a14e9c06ddb276b06134e90d37098be2830beaa4357205bec7ff1aa1f7c".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: "http://localhost:8080/linux-firmware.rpm".to_string(),
            flag_safe: true,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        let res = handler.download(&req);
        assert!(res.is_ok());
        assert_eq!(true, handler.paths.image_path.exists())
    }

    #[test]
    #[ignore]
    fn test_checksum_match() {
        init();
        let mut handler = DiskImageHandler::<RealCommandExecutor>::default();
        handler.paths.image_path = PathBuf::from("/home/yuhang/Documents/KubeOS/KubeOS-Rust/test_download_image");
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "90da5a14e9c06ddb276b06134e90d37098be2830beaa4357205bec7ff1aa1f7c".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: "http://localhost:8080/aaa.txt".to_string(),
            flag_safe: true,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        assert_eq!(handler.paths.image_path.exists(), true);
        handler.checksum_match(handler.paths.image_path.to_str().unwrap(), &req.check_sum).unwrap();
    }

    #[test]
    #[ignore]
    fn test_load_ca_client_certs() {
        init();
        let mut handler = DiskImageHandler::<RealCommandExecutor>::default();
        handler.certs_path = "/home/yuhang/Documents/data/cert/".to_string();
        let certs = CertsInfo {
            ca_cert: "nginx.crt".to_string(),
            client_cert: "client.crt".to_string(),
            client_key: "client.key".to_string(),
        };
        let res = handler.load_ca_client_certs(&certs);
        assert!(res.is_ok());
    }
}
