/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2023. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use anyhow::{anyhow, Context, Result};
use log::{debug, info};

use crate::{
    api::{ImageHandler, UpgradeRequest},
    sys_mgmt::{IMAGE_PERMISSION, NEED_BYTES},
    utils::*,
};

pub struct CtrImageHandler<T: CommandExecutor> {
    pub paths: PreparePath,
    pub executor: T,
}

const DEFAULT_NAMESPACE: &str = "k8s.io";

impl<T: CommandExecutor> ImageHandler<T> for CtrImageHandler<T> {
    fn download_image(&self, req: &UpgradeRequest) -> Result<UpgradeImageManager<T>> {
        perpare_env(&self.paths, NEED_BYTES, IMAGE_PERMISSION)?;
        self.get_image(req)?;
        self.get_rootfs_archive(req, IMAGE_PERMISSION)?;

        let (_, next_partition_info) = get_partition_info(&self.executor)?;
        let img_manager = UpgradeImageManager::new(self.paths.clone(), next_partition_info, self.executor.clone());
        img_manager.create_os_image(IMAGE_PERMISSION)
    }
}

impl Default for CtrImageHandler<RealCommandExecutor> {
    fn default() -> Self {
        Self { paths: PreparePath::default(), executor: RealCommandExecutor {} }
    }
}

impl<T: CommandExecutor> CtrImageHandler<T> {
    #[cfg(test)]
    pub fn new(paths: PreparePath, executor: T) -> Self {
        Self { paths, executor }
    }

    fn get_image(&self, req: &UpgradeRequest) -> Result<()> {
        let image_name = &req.container_image;
        is_valid_image_name(image_name)?;
        let cli: String =
            if is_command_available("crictl", &self.executor) { "crictl".to_string() } else { "ctr".to_string() };
        remove_image_if_exist(&cli, image_name, &self.executor)?;
        info!("Start pulling image {}", image_name);
        pull_image(&cli, image_name, &self.executor)?;
        info!("Start checking image digest");
        check_oci_image_digest(&cli, image_name, &req.check_sum, &self.executor)?;
        Ok(())
    }

    fn get_rootfs_archive(&self, req: &UpgradeRequest, permission: u32) -> Result<()> {
        let image_name = &req.container_image;
        let mount_path = &self
            .paths
            .mount_path
            .to_str()
            .ok_or_else(|| anyhow!("Failed to get mount path: {}", self.paths.mount_path.display()))?;
        info!("Start getting rootfs {}", image_name);
        self.check_and_unmount(mount_path).with_context(|| "Failed to clean containerd environment".to_string())?;
        self.executor
            .run_command("ctr", &["-n", DEFAULT_NAMESPACE, "images", "mount", "--rw", image_name, mount_path])?;
        // copy os.tar from mount_path to its partent dir
        self.copy_file(self.paths.mount_path.join(&self.paths.rootfs_file), &self.paths.tar_path, permission)?;
        self.check_and_unmount(mount_path).with_context(|| "Failed to clean containerd environment".to_string())?;
        Ok(())
    }

    fn check_and_unmount(&self, mount_path: &str) -> Result<()> {
        let ctr_snapshot_cmd =
            format!("ctr -n={} snapshots ls | grep {} | awk '{{print $1}}'", DEFAULT_NAMESPACE, mount_path);
        let exist_snapshot = self.executor.run_command_with_output("bash", &["-c", &ctr_snapshot_cmd])?;
        if !exist_snapshot.is_empty() {
            self.executor.run_command("ctr", &["-n", DEFAULT_NAMESPACE, "images", "unmount", mount_path])?;
            self.executor.run_command("ctr", &["-n", DEFAULT_NAMESPACE, "snapshots", "remove", mount_path])?;
        }
        Ok(())
    }

    fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(&self, src: P, dst: Q, permission: u32) -> Result<()> {
        let copied_bytes = fs::copy(src.as_ref(), dst.as_ref())?;
        debug!("Copy {} to {}, total bytes: {}", src.as_ref().display(), dst.as_ref().display(), copied_bytes);
        fs::set_permissions(dst, fs::Permissions::from_mode(permission))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, path::PathBuf};

    use mockall::mock;
    use tempfile::NamedTempFile;

    use super::*;
    use crate::api::CertsInfo;

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

    fn init() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_get_image() {
        init();
        let mut mock_executor = MockCommandExec::new();
        let image_name = "docker.io/library/busybox:latest";
        let req = UpgradeRequest {
            version: "KubeOS v2".to_string(),
            image_type: "containerd".to_string(),
            container_image: image_name.to_string(),
            check_sum: "22222".to_string(),
            image_url: "".to_string(),
            flag_safe: false,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        // mock is_command_available
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "/bin/sh" && args.contains(&"command -v crictl")) // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(()));
        // mock remove_image_if_exist
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "crictl" && args.contains(&"inspecti")) // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(()));
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "crictl" && args.contains(&"rmi")) // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(()));
        // mock pull_image
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| {
                cmd == "crictl" && args.contains(&"pull") && args.contains(&"docker.io/library/busybox:latest")
            })
            .times(1)
            .returning(|_, _| Ok(()));
        // mock get_oci_image_digest
        let command_output2 = "[docker.io/library/busybox:latest@sha256:22222]";
        mock_executor
            .expect_run_command_with_output()
            .withf(|cmd, args| {
                cmd == "crictl" && args.contains(&"inspecti") && args.contains(&"{{.status.repoDigests}}")
            })
            .times(1)
            .returning(|_, _| Ok(command_output2.to_string()));
        let ctr = CtrImageHandler::new(PreparePath::default(), mock_executor);
        let result = ctr.get_image(&req);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_rootfs_archive() {
        init();
        let mut mock_executor = MockCommandExec::new();
        let image_name = "docker.io/library/busybox:latest";
        let req = UpgradeRequest {
            version: "KubeOS v2".to_string(),
            image_type: "containerd".to_string(),
            container_image: image_name.to_string(),
            check_sum: "22222".to_string(),
            image_url: "".to_string(),
            flag_safe: false,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };

        // mock check_and_unmount
        mock_executor
            .expect_run_command_with_output()
            .withf(|cmd, args| cmd == "bash" && args.len() == 2 && args[0] == "-c") // simplified with a closure
            .times(1)
            .returning(|_, _| Ok("".to_string()));

        // mock ctr mount rw
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "ctr" && args.len() == 7 && args[4] == "--rw") // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(()));

        // create temp file for copy
        let mut tmp_file = NamedTempFile::new().expect("Failed to create temporary file.");
        writeln!(tmp_file, "Hello, world!").expect("Failed to write to temporary file.");

        // Get the path of the temporary file and the path where it should be copied.
        let src_dir = tmp_file.path().parent().unwrap();
        let src_file_name = tmp_file.path().file_name().unwrap().to_str().unwrap().to_string();
        let dst_file = NamedTempFile::new().expect("Failed to create destination temporary file.");
        let dst_path = dst_file.path().to_path_buf();

        let paths = PreparePath {
            persist_path: "/tmp".into(),
            update_path: PathBuf::new(),
            image_path: PathBuf::new(),
            mount_path: src_dir.to_path_buf(),
            rootfs_file: src_file_name.clone(),
            tar_path: dst_path.clone(),
        };

        // mock check_and_unmount
        mock_executor
            .expect_run_command_with_output()
            .withf(|cmd, args| cmd == "bash" && args.len() == 2 && args[0] == "-c") // simplified with a closure
            .times(1)
            .returning(|_, _| Ok("".to_string()));

        let ctr = CtrImageHandler::new(paths, mock_executor);
        let result = ctr.get_rootfs_archive(&req, IMAGE_PERMISSION);
        assert!(result.is_ok());
    }

    #[test]
    fn test_copy_file() {
        // Setup: Create a temporary file and write some data to it.
        let mut tmp_file = NamedTempFile::new().expect("Failed to create temporary file.");
        writeln!(tmp_file, "Hello, world!").expect("Failed to write to temporary file.");

        // Get the path of the temporary file and the path where it should be copied.
        let src_path = tmp_file.path().to_str().unwrap().to_string();
        let dst_file = NamedTempFile::new().expect("Failed to create destination temporary file.");
        let dst_path = dst_file.path().to_str().unwrap().to_string();

        let ctr = CtrImageHandler::default();
        let result = ctr.copy_file(&src_path, &dst_path, IMAGE_PERMISSION);

        assert!(result.is_ok());

        let expected_content = "Hello, world!\n";
        let actual_content = fs::read_to_string(&dst_path).expect("Failed to read destination file.");
        assert_eq!(expected_content, actual_content);

        // Assert the file permission
        let metadata = fs::metadata(&dst_path).expect("Failed to read destination file.");
        let expected_permission = 0o100600;
        assert_eq!(metadata.permissions().mode(), expected_permission);
    }

    #[test]
    fn test_check_and_unmount() {
        let mut mock_executor = MockCommandExec::new();

        // When `run_command_with_output` is called with "bash" and the specific args, it will return Ok("snapshot_exists").
        mock_executor
            .expect_run_command_with_output()
            .withf(|cmd, args| cmd == "bash" && args.len() == 2 && args[0] == "-c")
            .times(1)
            .returning(|_, _| Ok("snapshot_exists".to_string()));

        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "ctr" && args.contains(&"images"))
            .times(1)
            .returning(|_, _| Ok(()));

        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "ctr" && args.contains(&"snapshots"))
            .times(1)
            .returning(|_, _| Ok(()));

        let result = CtrImageHandler::new(PreparePath::default(), mock_executor).check_and_unmount("test_mount_path");

        assert!(result.is_ok());
    }
}
