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
    fs,
    os::{linux::fs::MetadataExt, unix::fs::DirBuilderExt},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use log::{debug, info, trace};
use nix::{mount, mount::MntFlags};

use crate::{
    sys_mgmt::{MOUNT_DIR, OS_IMAGE_NAME, PERSIST_DIR, ROOTFS_ARCHIVE, UPDATE_DIR},
    utils::CommandExecutor,
};

/// * persist_path: /persist
///
/// * update_path: /persist/KubeOS-Update
///
/// * mount_path: /persist/KubeOS-Update/kubeos-update
///
/// * tar_path: /persist/KubeOS-Update/os.tar
///
/// * image_path: /persist/update.img
///
/// * rootfs_file: os.tar
#[derive(Clone)]
pub struct PreparePath {
    pub persist_path: PathBuf,
    pub update_path: PathBuf,
    pub mount_path: PathBuf,
    pub tar_path: PathBuf,
    pub image_path: PathBuf,
    pub rootfs_file: String,
}

impl Default for PreparePath {
    fn default() -> Self {
        let persist_dir = Path::new(PERSIST_DIR);
        let update_pathbuf = persist_dir.join(UPDATE_DIR);
        Self {
            persist_path: persist_dir.to_path_buf(),
            update_path: update_pathbuf.clone(),
            mount_path: update_pathbuf.join(MOUNT_DIR),
            tar_path: update_pathbuf.join(ROOTFS_ARCHIVE),
            image_path: persist_dir.join(OS_IMAGE_NAME),
            rootfs_file: ROOTFS_ARCHIVE.to_string(),
        }
    }
}

pub fn is_file_exist<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().exists()
}

pub fn perpare_env(prepare_path: &PreparePath, need_bytes: i64, permission: u32) -> Result<()> {
    info!("Prepare environment to upgrade");
    check_disk_size(need_bytes, &prepare_path.persist_path)?;
    clean_env(&prepare_path.update_path, &prepare_path.mount_path, &prepare_path.image_path)?;
    fs::DirBuilder::new().recursive(true).mode(permission).create(&prepare_path.mount_path)?;
    Ok(())
}

pub fn check_disk_size<P: AsRef<Path>>(need_bytes: i64, path: P) -> Result<()> {
    trace!("Check if there is enough disk space to upgrade");
    let fs_stat = nix::sys::statfs::statfs(path.as_ref())?;
    let available_blocks = i64::try_from(fs_stat.blocks_available())?;
    let available_space = available_blocks * fs_stat.block_size();
    if available_space < need_bytes {
        bail!("Space is not enough for downloading");
    }
    Ok(())
}

/// clean_env will umount the mount path and delete directory /persist/KubeOS-Update and /persist/update.img
pub fn clean_env<P>(update_path: P, mount_path: P, image_path: P) -> Result<()>
where
    P: AsRef<Path> + std::fmt::Debug,
{
    if is_mounted(&mount_path)? {
        debug!("Umount \"{}\"", mount_path.as_ref().display());
        if let Err(errno) = mount::umount2(mount_path.as_ref(), MntFlags::MNT_FORCE) {
            bail!("Failed to umount {} in clean_env: {}", mount_path.as_ref().display(), errno);
        }
    }
    // losetup -D?
    delete_file_or_dir(&update_path).with_context(|| format!("Failed to delete {:?}", update_path))?;
    delete_file_or_dir(&image_path).with_context(|| format!("Failed to delete {:?}", image_path))?;
    Ok(())
}

pub fn delete_file_or_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    if is_file_exist(&path) {
        if fs::metadata(&path)?.is_file() {
            info!("Delete file \"{}\"", path.as_ref().display());
            fs::remove_file(&path)?;
        } else {
            info!("Delete directory \"{}\"", path.as_ref().display());
            fs::remove_dir_all(&path)?;
        }
    }
    Ok(())
}

pub fn is_command_available<T: CommandExecutor>(command: &str, command_executor: &T) -> bool {
    match command_executor.run_command("/bin/sh", &["-c", format!("command -v {}", command).as_str()]) {
        Ok(_) => {
            debug!("command {} is available", command);
            true
        },
        Err(_) => {
            debug!("command {} is not available", command);
            false
        },
    }
}

pub fn is_mounted<P: AsRef<Path>>(mount_path: P) -> Result<bool> {
    if !is_file_exist(&mount_path) {
        return Ok(false);
    }
    // Get device ID of mountPath
    let mount_meta = fs::symlink_metadata(&mount_path)?;
    let dev = mount_meta.st_dev();

    // Get device ID of mountPath's parent directory
    let parent = mount_path
        .as_ref()
        .parent()
        .ok_or_else(|| anyhow!("Failed to get parent directory of {}", mount_path.as_ref().display()))?;
    let parent_meta = fs::symlink_metadata(parent)?;
    let dev_parent = parent_meta.st_dev();
    Ok(dev != dev_parent)
}

pub fn switch_boot_menuentry<T: CommandExecutor>(
    command_executor: &T,
    grub_env_path: &str,
    next_menuentry: &str,
) -> Result<()> {
    if get_boot_mode() == "uefi" {
        command_executor.run_command(
            "grub2-editenv",
            &[grub_env_path, "set", format!("saved_entry={}", next_menuentry).as_str()],
        )?;
    } else {
        command_executor.run_command("grub2-set-default", &[next_menuentry])?;
    }
    Ok(())
}

pub fn get_boot_mode() -> String {
    if is_file_exist("/sys/firmware/efi") {
        "uefi".into()
    } else {
        "bios".into()
    }
}

pub fn is_dmv_mode<T: CommandExecutor>(c: &T) -> bool {
    c.run_command("veritysetup", &["status", "kubeos-root"]).is_ok()
}

#[cfg(test)]
mod tests {
    use mockall::{mock, predicate::*};
    use tempfile::{NamedTempFile, TempDir};

    use super::*;
    use crate::utils::RealCommandExecutor;

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
    fn test_is_file_exist() {
        init();
        let path = "/tmp/test_is_file_exist";
        assert_eq!(is_file_exist(path), false);

        let file = NamedTempFile::new().unwrap();
        assert_eq!(is_file_exist(file.path().to_str().unwrap()), true);

        let tmp_dir = TempDir::new().unwrap();
        assert_eq!(is_file_exist(tmp_dir.path().to_str().unwrap()), true);
    }

    #[test]
    fn test_prepare_env() {
        init();
        let paths = PreparePath {
            persist_path: PathBuf::from("/tmp"),
            update_path: PathBuf::from("/tmp/test_prepare_env"),
            mount_path: PathBuf::from("/tmp/test_prepare_env/kubeos-update"),
            tar_path: PathBuf::from("/tmp/test_prepare_env/os.tar"),
            image_path: PathBuf::from("/tmp/test_prepare_env/update.img"),
            rootfs_file: "os.tar".to_string(),
        };
        perpare_env(&paths, 1 * 1024 * 1024 * 1024, 0o700).unwrap();
    }

    #[test]
    fn test_check_disk_size() {
        init();
        let path = "/home";
        let gb: i64 = 1 * 1024 * 1024 * 1024;
        let need_gb = 1 * gb;
        let result = check_disk_size(need_gb, path);
        assert!(result.is_ok());
        let need_gb = 10000 * gb;
        let result = check_disk_size(need_gb, path);
        assert!(result.is_err());
    }

    #[test]
    fn test_clean_env() {
        init();
        let update_path = "/tmp/test_clean_env";
        let mount_path = "/tmp/test_clean_env/kubeos-update";
        let image_path = "/tmp/test_clean_env/update.img";
        clean_env(&update_path.to_string(), &mount_path.to_string(), &image_path.to_string()).unwrap();
    }

    #[test]
    fn test_delete_file_or_dir() {
        init();
        let path = "/tmp/test_delete_file";
        fs::File::create(path).unwrap();
        assert_eq!(Path::new(path).exists(), true);
        delete_file_or_dir(&path.to_string()).unwrap();
        assert_eq!(Path::new(path).exists(), false);

        let path = "/tmp/test_dir";
        fs::create_dir(path).unwrap();
        assert_eq!(Path::new(path).exists(), true);
        delete_file_or_dir(&path.to_string()).unwrap();
        assert_eq!(Path::new(path).exists(), false);

        let path = "/tmp/nonexist";
        delete_file_or_dir(path).unwrap();

        let path = PathBuf::new();
        delete_file_or_dir(path).unwrap();
    }

    #[test]
    fn test_switch_boot_menuentry() {
        init();
        let grubenv_path = "/boot/efi/EFI/openEuler/grubenv";
        let next_menuentry = "B";
        let mut mock = MockCommandExec::new();
        if get_boot_mode() == "uefi" {
            mock.expect_run_command()
                .withf(move |name, args| {
                    name == "grub2-editenv"
                        && args[0] == grubenv_path
                        && args[2] == format!("saved_entry={}", next_menuentry).as_str()
                })
                .times(1) // Expect it to be called once
                .returning(move |_, _| Ok(()));
        } else {
            mock.expect_run_command()
                .withf(move |name, args| name == "grub2-set-default" && args[0] == next_menuentry)
                .times(1) // Expect it to be called once
                .returning(move |_, _| Ok(()));
        }

        switch_boot_menuentry(&mock, grubenv_path, next_menuentry).unwrap()
    }

    #[test]
    fn test_get_boot_mode() {
        init();
        let boot_mode = get_boot_mode();
        let executor = RealCommandExecutor {};
        let res = executor.run_command("ls", &["/sys/firmware/efi"]);
        if res.is_ok() {
            assert!(boot_mode == "uefi");
        } else {
            assert!(boot_mode == "bios");
        }
    }

    #[test]
    fn test_is_command_available() {
        init();
        let executor = RealCommandExecutor {};
        assert_eq!(is_command_available("ls", &executor), true);
        assert_eq!(is_command_available("aaaabb", &executor), false);
    }

    #[test]
    fn test_is_dmv_mode() {
        init();
        let executor = RealCommandExecutor {};
        assert_eq!(is_dmv_mode(&executor), false);
    }
}
