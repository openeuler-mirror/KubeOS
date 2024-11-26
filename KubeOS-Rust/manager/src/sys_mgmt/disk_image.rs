use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
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
    pub dmv: bool,
}

impl<T: CommandExecutor> ImageHandler<T> for DiskImageHandler<T> {
    fn download_image(&self, req: &UpgradeRequest) -> Result<UpgradeImageManager<T>> {
        if self.dmv {
            bail!("DM-Verity doesn't support disk image upgrade");
        }
        clean_env(&self.paths.update_path, &self.paths.mount_path, &self.paths.image_path)?;
        fs::DirBuilder::new().recursive(true).mode(IMAGE_PERMISSION).create(&self.paths.mount_path)?;
        self.download(req)?;
        self.checksum_match(self.paths.tar_path.to_str().unwrap_or_default(), &req.check_sum)?;
        let (_, next_partition_info) = get_partition_info(&self.executor)?;
        let img_manager =
            UpgradeImageManager::new(self.paths.clone(), next_partition_info, self.executor.clone(), false);
        img_manager.create_os_image(IMAGE_PERMISSION)
    }
}

impl Default for DiskImageHandler<RealCommandExecutor> {
    fn default() -> Self {
        Self {
            paths: PreparePath::default(),
            executor: RealCommandExecutor {},
            certs_path: CERTS_PATH.to_string(),
            dmv: false,
        }
    }
}

impl<T: CommandExecutor> DiskImageHandler<T> {
    #[cfg(test)]
    pub fn new(paths: PreparePath, executor: T, certs_path: String, dmv: bool) -> Self {
        Self { paths, executor, certs_path, dmv }
    }

    fn download(&self, req: &UpgradeRequest) -> Result<()> {
        let mut resp = self.send_download_request(req)?;
        if resp.status() != reqwest::StatusCode::OK {
            bail!("Failed to download upgrade tar from {}, status: {}", req.image_url, resp.status());
        }
        debug!("Received response body size: {:?}", resp.content_length().unwrap_or_default());
        let need_bytes = resp.content_length().unwrap_or_default() + BUFFER;

        check_disk_size(
            i64::try_from(need_bytes).with_context(|| "Failed to transform content length from u64 to i64")?,
            self.paths.tar_path.parent().unwrap_or_else(|| Path::new(PERSIST_DIR)),
        )?;

        let dst = &self.paths.tar_path;
        let mut out = fs::File::create(dst)?;
        trace!("Start to save upgrade tar to path {}", dst.display());
        out.set_permissions(fs::Permissions::from_mode(IMAGE_PERMISSION))?;
        let bytes = resp.copy_to(&mut out)?;
        info!("Download upgrade tar successfully, upgrade tar path: {}, write bytes: {}", dst.display(), bytes);
        Ok(())
    }

    fn checksum_match(&self, file_path: &str, check_sum: &str) -> Result<()> {
        info!("Start checking file checksum");
        let check_sum = check_sum.to_ascii_lowercase();
        let file = fs::read(file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(file);
        let hash = hasher.finalize();
        // sha256sum -b /persist/update.img
        let cal_sum = format!("{:X}", hash).to_ascii_lowercase();
        if cal_sum != check_sum {
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
                bail!("The upgrade tar url is not safe");
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
    use std::io::Write;

    use mockall::mock;
    use mockito;
    use tempfile::NamedTempFile;

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
    }
    mock! {
        pub CommandExec{}
        impl CommandExecutor for CommandExec {
            fn run_command<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<()>;
            fn run_command_with_output<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<String>;
        }
        impl Clone for CommandExec {
            fn clone(&self) -> Self;
        }
    }

    #[test]
    fn test_dmv_mode() {
        init();
        let handler = DiskImageHandler::new(PreparePath::default(), RealCommandExecutor {}, String::new(), true);
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "1327e27d600538354d93bd68cce86566dd089e240c126dc3019cafabdc65aa02".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: "https://localhost:8082/aaa.txt".to_string(),
            flag_safe: true,
            mtls: true,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        assert!(handler.download_image(&req).is_err());
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
        let handler = DiskImageHandler::<RealCommandExecutor>::new(
            PreparePath::default(),
            RealCommandExecutor {},
            String::new(),
            false,
        );
        let res = handler.cert_exist(tmp_file.path().to_str().unwrap());
        assert!(res.is_ok());

        assert!(handler.cert_exist("aaa.pem").is_err());
        assert!(handler.cert_exist("").is_err())
    }

    #[test]
    fn test_send_download_request() {
        init();
        // This is a tmp cert only for KubeOS unit testing.
        let tmp_cert = "-----BEGIN CERTIFICATE-----\n\
        MIIBdDCCARqgAwIBAgIVALnQ5XwM2En1P+xCpkXsO44f8SAUMAoGCCqGSM49BAMC\n\
        MCExHzAdBgNVBAMMFnJjZ2VuIHNlbGYgc2lnbmVkIGNlcnQwIBcNNzUwMTAxMDAw\n\
        MDAwWhgPNDA5NjAxMDEwMDAwMDBaMCExHzAdBgNVBAMMFnJjZ2VuIHNlbGYgc2ln\n\
        bmVkIGNlcnQwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQAi4bkPp5iI9F36HH2\n\
        Gn+/sC0Ss+DanYY/wEwCrTXDXzAsA0Fuwg0kX75y8qF5JOfWW4tvZwKbeRa5s8vp\n\
        HpJNoy0wKzApBgNVHREEIjAgghNoZWxsby53b3JsZC5leGFtcGxlgglsb2NhbGhv\n\
        c3QwCgYIKoZIzj0EAwIDSAAwRQIhALuS4MU94wJmOZLN+nO7UaTspMN9zbTTkDkG\n\
        vG+oLD1sAiBg9wpCw+MWJHWvU+H/72mIac9YsC48BYwA7E/LQUOrkw==\n\
        -----END CERTIFICATE-----\n";

        // This is a tmp private key only for KubeOS unit testing.
        let tmp_key = "-----BEGIN PRIVATE KEY-----\n\
        MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg9Puh/0yMP7S6jXvX\n\
        Q8K3/COzzyJj84bT8/MJaJ0qp7ihRANCAAQAi4bkPp5iI9F36HH2Gn+/sC0Ss+Da\n\
        nYY/wEwCrTXDXzAsA0Fuwg0kX75y8qF5JOfWW4tvZwKbeRa5s8vpHpJN\n\
        -----END PRIVATE KEY-----\n";

        // Create a temporary file to hold the certificate
        let mut cert_file = NamedTempFile::new().unwrap();
        cert_file.write_all(tmp_cert.as_bytes()).unwrap();
        println!("cert_file: {:?}", cert_file.path().to_str().unwrap());

        // Create a temporary file to hold the private key
        let mut key_file = NamedTempFile::new().unwrap();
        key_file.write_all(tmp_key.as_bytes()).unwrap();
        // http
        let handler = DiskImageHandler::<RealCommandExecutor>::default();
        let mut req = UpgradeRequest {
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
        assert!(res.is_err());
        req.flag_safe = false;
        let res = handler.send_download_request(&req);
        assert!(res.is_err());

        // https
        let mut handler = DiskImageHandler::<RealCommandExecutor>::default();
        handler.certs_path = "/tmp".to_string();
        let tmp_cert_filename = cert_file.path().file_name().unwrap().to_str().unwrap();
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "1327e27d600538354d93bd68cce86566dd089e240c126dc3019cafabdc65aa02".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: "https://localhost:8081/aaa.txt".to_string(),
            flag_safe: true,
            mtls: false,
            certs: CertsInfo {
                ca_cert: tmp_cert_filename.to_string(),
                client_cert: "".to_string(),
                client_key: "".to_string(),
            },
        };
        let res = handler.send_download_request(&req);
        assert!(res.is_err());

        // mtls
        let tmp_key = NamedTempFile::new().unwrap();
        let tmp_key_filename = tmp_key.path().file_name().unwrap().to_str().unwrap();
        let mut handler = DiskImageHandler::<RealCommandExecutor>::default();
        handler.certs_path = "/tmp".to_string();
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "1327e27d600538354d93bd68cce86566dd089e240c126dc3019cafabdc65aa02".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: "https://localhost:8082/aaa.txt".to_string(),
            flag_safe: true,
            mtls: true,
            certs: CertsInfo {
                ca_cert: tmp_cert_filename.to_string(),
                client_cert: tmp_cert_filename.to_string(),
                client_key: tmp_key_filename.to_string(),
            },
        };
        let res = handler.send_download_request(&req);
        assert!(res.is_err());
    }

    #[test]
    fn test_checksum_match() {
        init();
        let mut tmp_file = NamedTempFile::new().unwrap();
        tmp_file.write(b"This is a test txt file for KubeOS test.\n").unwrap();
        let mut handler = DiskImageHandler::<RealCommandExecutor>::default();
        handler.paths.image_path = tmp_file.path().to_path_buf();
        let mut req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "98Ea7aff44631D183e6df3488f1107357d7503e11e5f146effdbfd11810cd4a2".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: "http://localhost:8080/aaa.txt".to_string(),
            flag_safe: true,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        assert_eq!(handler.paths.image_path.exists(), true);
        handler.checksum_match(handler.paths.image_path.to_str().unwrap(), &req.check_sum).unwrap();

        req.check_sum = "1234567Abc".into();
        let res = handler.checksum_match(handler.paths.image_path.to_str().unwrap(), &req.check_sum);
        assert!(res.is_err());
    }

    #[test]
    fn test_load_certs() {
        init();
        // This is a tmp cert only for KubeOS unit testing.
        let tmp_cert = "-----BEGIN CERTIFICATE-----\n\
        MIIBdDCCARqgAwIBAgIVALnQ5XwM2En1P+xCpkXsO44f8SAUMAoGCCqGSM49BAMC\n\
        MCExHzAdBgNVBAMMFnJjZ2VuIHNlbGYgc2lnbmVkIGNlcnQwIBcNNzUwMTAxMDAw\n\
        MDAwWhgPNDA5NjAxMDEwMDAwMDBaMCExHzAdBgNVBAMMFnJjZ2VuIHNlbGYgc2ln\n\
        bmVkIGNlcnQwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQAi4bkPp5iI9F36HH2\n\
        Gn+/sC0Ss+DanYY/wEwCrTXDXzAsA0Fuwg0kX75y8qF5JOfWW4tvZwKbeRa5s8vp\n\
        HpJNoy0wKzApBgNVHREEIjAgghNoZWxsby53b3JsZC5leGFtcGxlgglsb2NhbGhv\n\
        c3QwCgYIKoZIzj0EAwIDSAAwRQIhALuS4MU94wJmOZLN+nO7UaTspMN9zbTTkDkG\n\
        vG+oLD1sAiBg9wpCw+MWJHWvU+H/72mIac9YsC48BYwA7E/LQUOrkw==\n\
        -----END CERTIFICATE-----\n";

        // This is a tmp private key only for KubeOS unit testing.
        let tmp_key = "-----BEGIN PRIVATE KEY-----\n\
        MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg9Puh/0yMP7S6jXvX\n\
        Q8K3/COzzyJj84bT8/MJaJ0qp7ihRANCAAQAi4bkPp5iI9F36HH2Gn+/sC0Ss+Da\n\
        nYY/wEwCrTXDXzAsA0Fuwg0kX75y8qF5JOfWW4tvZwKbeRa5s8vpHpJN\n\
        -----END PRIVATE KEY-----\n";

        // Create a temporary file to hold the certificate
        let mut cert_file = NamedTempFile::new().unwrap();
        cert_file.write_all(tmp_cert.as_bytes()).unwrap();

        // Create a temporary file to hold the private key
        let mut key_file = NamedTempFile::new().unwrap();
        key_file.write_all(tmp_key.as_bytes()).unwrap();

        let mut handler = DiskImageHandler::<RealCommandExecutor>::default();
        handler.certs_path = "".to_string();
        let certs = CertsInfo {
            ca_cert: cert_file.path().to_str().unwrap().to_string(),
            client_cert: cert_file.path().to_str().unwrap().to_string(),
            client_key: key_file.path().to_str().unwrap().to_string(),
        };

        let res = handler.load_ca_client_certs(&certs);
        assert!(res.is_ok());

        let res = handler.load_ca_certs(&certs.ca_cert);
        assert!(res.is_ok());
    }

    #[test]
    fn test_download() {
        init();
        let tmp_file = NamedTempFile::new().unwrap();

        let mock_executor = MockCommandExec::new();
        let mut handler = DiskImageHandler::new(PreparePath::default(), mock_executor, String::new(), false);
        handler.paths.update_path = tmp_file.path().parent().unwrap().to_path_buf();
        handler.paths.tar_path = tmp_file.path().to_path_buf();

        let url = mockito::server_url();
        let upgrade_request = UpgradeRequest {
            version: "v2".into(),
            check_sum: "98ea7aff44631d183e6df3488f1107357d7503e11e5f146effdbfd11810cd4a2".into(),
            image_type: "disk".into(),
            container_image: "".into(),
            image_url: format!("{}/test.txt", url),
            flag_safe: true,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        let _m = mockito::mock("GET", "/test.txt")
            .with_status(200)
            .with_body("This is a test txt file for KubeOS test.\n")
            .create();
        handler.download(&upgrade_request).unwrap();
        assert_eq!(true, handler.paths.tar_path.exists());
        assert_eq!(
            fs::read(handler.paths.tar_path.to_str().unwrap()).unwrap(),
            "This is a test txt file for KubeOS test.\n".as_bytes()
        );

        let _m = mockito::mock("GET", "/test.txt").with_status(404).with_body("Not found").create();
        let res = handler.download(&upgrade_request);
        assert!(res.is_err())
    }
}
