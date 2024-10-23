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

use std::{
    fs::{self, Permissions},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

use anyhow::{Context, Result};
use log::{debug, info};

use super::{
    clean_env,
    common::{delete_file_or_dir, PreparePath},
    executor::CommandExecutor,
    partition::PartitionInfo,
};

pub struct UpgradeImageManager<T: CommandExecutor> {
    pub paths: PreparePath,
    pub next_partition: PartitionInfo,
    pub executor: T,
}

impl<T: CommandExecutor> UpgradeImageManager<T> {
    pub fn new(paths: PreparePath, next_partition: PartitionInfo, executor: T) -> Self {
        Self { paths, next_partition, executor }
    }

    fn image_path_str(&self) -> Result<&str> {
        self.paths.image_path.to_str().context("Failed to convert image path to string")
    }

    fn mount_path_str(&self) -> Result<&str> {
        self.paths.mount_path.to_str().context("Failed to convert mount path to string")
    }

    fn tar_path_str(&self) -> Result<&str> {
        self.paths.tar_path.to_str().context("Failed to convert tar path to string")
    }

    pub fn create_image_file(&self, permission: u32) -> Result<()> {
        let image_str = self.image_path_str()?;

        // convert bytes to the count of 2MB block
        let count = self.next_partition.size / ( 2 << 20 );

        debug!("Create image {}, count {}", image_str, count);

        self.executor.run_command("dd", &["if=/dev/zero", &format!("of={}", image_str), "bs=2M", &format!("count={}", count)])?;
        fs::set_permissions(&self.paths.image_path, Permissions::from_mode(permission))?;
        Ok(())
    }

    pub fn format_image(&self) -> Result<()> {
        let image_str = self.image_path_str()?;
        debug!("Format image {}", image_str);
        self.executor.run_command(
            format!("mkfs.{}", self.next_partition.fs_type).as_str(),
            &["-L", format!("ROOT-{}", self.next_partition.menuentry).as_str(), image_str],
        )?;
        Ok(())
    }

    pub fn mount_image(&self) -> Result<()> {
        let image_str = self.image_path_str()?;
        let mount_str = self.mount_path_str()?;
        debug!("Mount {} to {}", image_str, mount_str);
        self.executor.run_command("mount", &["-o", "loop", image_str, mount_str])?;
        Ok(())
    }

    pub fn extract_tar_to_image(&self) -> Result<()> {
        let tar_str = self.tar_path_str()?;
        let mount_str = self.mount_path_str()?;
        debug!("Extract {} to mounted path {}", tar_str, mount_str);
        self.executor.run_command("tar", &["-xvf", tar_str, "-C", mount_str])?;
        Ok(())
    }

    pub fn create_os_image(self, permission: u32) -> Result<Self> {
        self.create_image_file(permission)?;
        self.format_image()?;
        self.mount_image()?;
        self.extract_tar_to_image()?;
        // Pass empty image_path to clean_env but avoid deleting the upgrade image
        clean_env(&self.paths.update_path, &self.paths.mount_path, &PathBuf::new())?;
        Ok(self)
    }

    pub fn install(&self) -> Result<()> {
        let image_str = self.image_path_str()?;
        let device = self.next_partition.device.as_str();
        self.executor
            .run_command("dd", &[format!("if={}", image_str).as_str(), format!("of={}", device).as_str(), "bs=8M"])?;
        debug!("Install image {} to {} done", image_str, device);
        info!(
            "Device {} is overwritten and unable to rollback to the previous version anymore if the eviction of node fails",
            device
        );
        delete_file_or_dir(image_str)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, io::Write, path::Path};

    use mockall::{mock, predicate::*};
    use tempfile::NamedTempFile;

    use super::*;

    // Mock the CommandExecutor trait
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
    fn test_update_image_manager() {
        init();
        // create a dir in tmp dir
        let tmp_dir = "/tmp/test_update_image_manager";
        let img_path = format!("{}/test_image", tmp_dir);
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "test content").unwrap(); // Writing s
        fs::create_dir(tmp_dir).unwrap();
        let clone_img_path = img_path.clone();

        let mut mock = MockCommandExec::new();
        //mock create_image_file
        mock.expect_run_command()
            .withf(|name, args| name == "dd" && args[0] == "if=/dev/zero")
            .times(1) // Expect it to be called once
            .returning(move |_, _| {
                // simulate 'dd' by copying the contents of the temporary file
                std::fs::copy(temp_file.path(), &clone_img_path).unwrap();
                Ok(())
            });

        //mock format_image
        mock.expect_run_command()
            .withf(|name, args| name == "mkfs.ext4" && args[1] == "ROOT-B")
            .times(1) // Expect it to be called once
            .returning(|_, _| Ok(()));

        //mock mount_image
        mock.expect_run_command()
            .withf(|name, _| name == "mount")
            .times(1) // Expect it to be called once
            .returning(|_, _| Ok(()));

        //mock extract_tar_to_image
        mock.expect_run_command()
            .withf(|name, args| name == "tar" && args[0] == "-xvf")
            .times(1) // Expect it to be called once
            .returning(|_, _| Ok(()));

        //mock install->dd
        mock.expect_run_command()
            .withf(|name, _| name == "dd")
            .times(1) // Expect it to be called once
            .returning(|_, _| Ok(()));

        let img_manager = UpgradeImageManager::new(
            PreparePath {
                persist_path: "/tmp".into(),
                update_path: tmp_dir.into(),
                image_path: img_path.into(),
                mount_path: "/tmp/update/mount".into(),
                tar_path: "/tmp/update/image.tar".into(),
                rootfs_file: "image.tar".into(),
            },
            PartitionInfo { device: "/dev/sda3".into(), fs_type: "ext4".into(), menuentry: "B".into(), size:13000245248},
            mock,
        );

        let img_manager = img_manager.create_os_image(0o755).unwrap();
        let result = img_manager.install();
        assert!(result.is_ok());

        assert_eq!(Path::new(&tmp_dir).exists(), false);
    }
}
