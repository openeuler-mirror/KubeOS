/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2024. All rights reserved.
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
    env,
    fs::{self, File},
    path::PathBuf,
};

use anyhow::{anyhow, bail, Result};
use fs2::{available_space, free_space, total_space};
use log::{debug, warn};

use crate::{
    commands::{DiskPartition, ImageType, PxeConfig, RepoInfo},
    scripts_gen::*,
    utils::{self, set_permissions},
    values::*,
    Config, CreateImage,
};

impl CreateImage for RepoInfo {
    fn prepare(&self, config: &mut Config) -> Result<()> {
        self.check_legacy_bios()?;
        self.check_dm_verity(&config)?;
        self.verify_repo_input()?;
        check_disk_space(self.image_type.as_ref().unwrap(), &config.disk_partition)?;
        check_repo_file_valid(&self.repo_path)?;
        check_agent_file_valid(&self.agent_path)?;
        if self.image_type == Some(ImageType::PxeRepo) {
            self.check_pxe(config)?;
        }
        if self.image_type == Some(ImageType::UpgradeImage) {
            self.check_upgrade_image(config)?;
        }
        Ok(())
    }

    fn generate_scripts(&self, config: &Config) -> Result<PathBuf> {
        self.write_rpmlist(config)?;
        self.write_misc_files()?;
        self.write_grub_cfg(config.dm_verity.is_some())?;
        self.write_set_in_chroot(config)?;
        let kbimg_path = self.create_kbimg_script(config)?;
        set_permissions(&kbimg_path, EXEC_PERMISSION)?;
        Ok(kbimg_path)
    }
}

impl RepoInfo {
    fn create_kbimg_script(&self, config: &Config) -> Result<PathBuf> {
        let kbimg_path = format!("{}/{}", SCRIPTS_DIR, KBIMG_SH);
        let mut kbimg = File::create(&kbimg_path)?;
        base_gen(&mut kbimg, GLOBAL_VARS, true)?;
        gen_repo_vars(&mut kbimg, &self, &config.dm_verity, &config.grub)?;
        gen_global_func(&mut kbimg)?;
        gen_mount_proc_dev_sys(&mut kbimg)?;
        gen_unmount_dir(&mut kbimg)?;
        gen_create_os_tar_from_repo(&mut kbimg, &self, config)?;
        self.generate_image_specific_scripts(&mut kbimg, config)?;
        Ok(PathBuf::from(kbimg_path))
    }

    fn generate_image_specific_scripts(&self, kbimg: &mut File, config: &Config) -> Result<()> {
        match self.image_type.as_ref().unwrap() {
            ImageType::UpgradeImage => self.generate_upgrade_image_scripts(kbimg, config)?,
            ImageType::VMRepo => self.generate_vm_repo_scripts(kbimg, config)?,
            ImageType::PxeRepo => self.generate_pxe_repo_scripts(kbimg, config)?,
            _ => bail!("Invalid image type: {:?}", self.image_type),
        }
        Ok(())
    }

    fn generate_upgrade_image_scripts(&self, kbimg: &mut File, config: &Config) -> Result<()> {
        if config.dm_verity.is_none() {
            self.write_upgrade_dockerfile()?;
            gen_create_docker_img(kbimg)?;
        } else {
            self.generate_vm_repo_scripts(kbimg, config)?;
            config.dm_verity.as_ref().unwrap().write_dm_verity_upgrade(kbimg)?;
        }
        Ok(())
    }

    fn generate_vm_repo_scripts(&self, kbimg: &mut File, config: &Config) -> Result<()> {
        write_bootloader(self.arch.as_deref().unwrap(), self.legacy_bios)?;
        gen_init_partition(kbimg)?;
        gen_set_partuuid(kbimg, self.legacy_bios, config.dm_verity.is_some())?;
        gen_create_img(kbimg, self.legacy_bios, config)?;
        gen_create_vm_repo_img(kbimg)?;
        if let Some(dmv) = config.dm_verity.as_ref() {
            dmv.write_dm_verity_repo()?;
        }
        Ok(())
    }

    fn generate_pxe_repo_scripts(&self, kbimg: &mut File, config: &Config) -> Result<()> {
        gen_create_pxe_repo_img(kbimg)?;
        self.write_bootup(config)?;
        Ok(())
    }

    fn write_set_in_chroot(&self, config: &Config) -> Result<()> {
        let set_in_chroot_path = format!("{}/{}", SCRIPTS_DIR, SET_IN_CHROOT_SH);
        let mut set_in_chroot = File::create(&set_in_chroot_path)?;
        gen_set_in_chroot(
            &mut set_in_chroot,
            self.legacy_bios,
            self.arch.as_deref().unwrap_or(""),
            self.image_type.as_ref().unwrap(),
            config,
        )?;
        set_permissions(&set_in_chroot_path, EXEC_PERMISSION)?;
        Ok(())
    }

    fn write_bootup(&self, config: &Config) -> Result<()> {
        fs::create_dir_all(BOOTUP_DIR)?;
        utils::set_permissions(BOOTUP_DIR, DIR_PERMISSION)?;
        let mount_path = format!("{}/{}", BOOTUP_DIR, BOOTUP_MOUNT_SH);
        let mut mount = File::create(&mount_path)?;
        if let Some(pxe_config) = &config.pxe_config {
            gen_global_cfg(&mut mount, &pxe_config)?;
        }
        gen_mount(&mut mount, config)?;
        set_permissions(&mount_path, EXEC_PERMISSION)?;

        let module_setup_path = format!("{}/{}", BOOTUP_DIR, BOOTUP_MODULE_SETUP_SH);
        let mut module_setup = File::create(&module_setup_path)?;
        base_gen(&mut module_setup, MODULE_SETUP, true)?;
        set_permissions(&module_setup_path, EXEC_PERMISSION)?;
        Ok(())
    }

    fn write_rpmlist(&self, config: &Config) -> Result<()> {
        let rpmlist_path = format!("{}/{}", SCRIPTS_DIR, RPMLIST);
        let mut rpmlist = File::create(&rpmlist_path)?;
        gen_rpm_list(
            &mut rpmlist,
            &self.rpmlist,
            self.arch.as_deref().unwrap(),
            self.legacy_bios,
            config.dm_verity.is_some(),
        )?;
        set_permissions(&rpmlist_path, CONFIG_PERMISSION)?;
        Ok(())
    }

    fn write_misc_files(&self) -> Result<()> {
        fs::create_dir_all(MISC_FILES_DIR)?;
        utils::set_permissions(MISC_FILES_DIR, DIR_PERMISSION)?;

        if self.legacy_bios {
            let boot_grub2_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_BOOT_GRUB2_MOUNT);
            let mut boot_grub2_mount = File::create(&boot_grub2_mount_path)?;
            base_gen(&mut boot_grub2_mount, BOOT_GRUB2_MOUNT, false)?;
            set_permissions(&boot_grub2_mount_path, CONFIG_PERMISSION)?;
        } else {
            let boot_efi_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_BOOT_EFI_MOUNT);
            let mut boot_efi_mount = File::create(&boot_efi_mount_path)?;
            base_gen(&mut boot_efi_mount, BOOT_EFI_MOUNT, false)?;
            set_permissions(&boot_efi_mount_path, CONFIG_PERMISSION)?;
        }

        let etc_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_ETC_MOUNT);
        let mut etc_mount = File::create(&etc_mount_path)?;
        base_gen(&mut etc_mount, ETC_MOUNT, false)?;
        set_permissions(&etc_mount_path, CONFIG_PERMISSION)?;

        let opt_cni_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_OPT_CNI_MOUNT);
        let mut opt_cni_mount = File::create(&opt_cni_mount_path)?;
        base_gen(&mut opt_cni_mount, OPT_CNI_MOUNT, false)?;
        set_permissions(&opt_cni_mount_path, CONFIG_PERMISSION)?;

        let os_agent_service_path = format!("{}/{}", MISC_FILES_DIR, MISC_OS_AGENT_SERVICE);
        let mut os_agent_service = File::create(&os_agent_service_path)?;
        base_gen(&mut os_agent_service, OS_AGENT_SERVICE, false)?;
        set_permissions(&os_agent_service_path, CONFIG_PERMISSION)?;

        let os_release_path = format!("{}/{}", MISC_FILES_DIR, MISC_OS_RELEASE);
        let mut os_release = File::create(&os_release_path)?;
        gen_os_release(&mut os_release)?;
        set_permissions(&os_release_path, CONFIG_PERMISSION)?;

        let persist_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_PERSIST_MOUNT);
        let mut persist_mount = File::create(&persist_mount_path)?;
        base_gen(&mut persist_mount, PERSIST_MOUNT, false)?;
        set_permissions(&persist_mount_path, CONFIG_PERMISSION)?;

        let var_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_VAR_MOUNT);
        let mut var_mount = File::create(&var_mount_path)?;
        base_gen(&mut var_mount, VAR_MOUNT, false)?;
        set_permissions(&var_mount_path, CONFIG_PERMISSION)?;
        Ok(())
    }

    fn write_grub_cfg(&self, dmv: bool) -> Result<()> {
        let grub_cfg_path = format!("{}/{}", SCRIPTS_DIR, GRUB_CFG);
        let mut grub_cfg = File::create(&grub_cfg_path)?;
        if dmv {
            base_gen(&mut grub_cfg, DMV_MAIN_GRUB_CFG, false)?;
        } else {
            base_gen(&mut grub_cfg, GRUB_CFG_CONTENTS, false)?;
        }
        set_permissions(&grub_cfg_path, CONFIG_PERMISSION)?;
        Ok(())
    }

    fn write_upgrade_dockerfile(&self) -> Result<()> {
        let dockerfile_path = format!("{}/{}", SCRIPTS_DIR, DOCKERFILE);
        let mut dockerfile = File::create(&dockerfile_path)?;
        base_gen(&mut dockerfile, OS_TAR_DOCKERFILE, false)?;
        set_permissions(&dockerfile_path, CONFIG_PERMISSION)?;
        Ok(())
    }

    fn check_legacy_bios(&self) -> Result<()> {
        if self.legacy_bios && self.arch.as_deref() == Some("aarch64") {
            bail!("aarch64 does not support legacy bios");
        }
        Ok(())
    }

    fn check_dm_verity(&self, config: &Config) -> Result<()> {
        if config.dm_verity.is_some() {
            if self.legacy_bios {
                bail!("dm_verity does not support legacy bios");
            }
            if config.grub.is_none() {
                bail!("grub is required for dm_verity");
            }
            if self.image_type != Some(ImageType::VMRepo) && self.image_type != Some(ImageType::UpgradeImage) {
                bail!("dm_verity only supports VMRepo and UpgradeImage mode");
            }
        }
        Ok(())
    }

    fn verify_repo_input(&self) -> Result<()> {
        if !utils::is_valid_param(self.repo_path.to_str().unwrap()) {
            bail!("params {} is invalid, please check input", self.repo_path.to_str().unwrap());
        }
        if !utils::is_valid_param(&self.version) {
            bail!("params {} is invalid, please check input", self.version);
        }
        if !utils::is_valid_param(self.agent_path.to_str().unwrap()) {
            bail!("params {} is invalid, please check input", self.agent_path.to_str().unwrap());
        }
        if let Some(docker_img) = &self.upgrade_img {
            if !utils::is_valid_param(docker_img) {
                bail!("params {} is invalid, please check input", docker_img);
            }
        }
        Ok(())
    }

    fn check_upgrade_image(&self, config: &Config) -> Result<()> {
        if self.upgrade_img.is_none() {
            bail!("upgrade_img field is required for generating upgrade-img");
        }
        if config.pxe_config.is_some() && config.dm_verity.is_some() {
            bail!("dm_verity does NOT support PXE upgrade-img");
        }
        Ok(())
    }

    fn check_pxe(&self, config: &Config) -> Result<()> {
        if self.legacy_bios {
            warn!("KubeOS PXE image does NOT support legacy bios for x86_64 or aarch64");
        }
        let pxe_config =
            config.pxe_config.as_ref().ok_or_else(|| anyhow!("pxe_config is required for building pxe image"))?;
        check_pxe_conf_valid(pxe_config)?;
        Ok(())
    }
}

// Check pxe config
fn check_pxe_conf_valid(config: &PxeConfig) -> anyhow::Result<()> {
    if config.dhcp.unwrap_or(false) {
        if config.local_ip.is_some() || config.net_name.is_some() {
            bail!("dhcp and local_ip/net_name cannot be set at the same time");
        }
    } else {
        let local_ip = config.local_ip.as_ref().ok_or_else(|| anyhow!("local_ip not found!"))?;
        if !utils::is_addr_valid(local_ip) {
            bail!("address {} is invalid, please check input", local_ip)
        }
        let netmask = config.netmask.as_ref().ok_or_else(|| anyhow!("netmask not found!"))?;
        if !utils::is_addr_valid(netmask) {
            bail!("address {} is invalid, please check input", netmask)
        }
    }
    if !utils::is_addr_valid(&config.server_ip) {
        bail!("address {} is invalid, please check input", &config.server_ip)
    }
    if !utils::is_addr_valid(&config.route_ip) {
        bail!("address {} is invalid, please check input", &config.route_ip)
    }
    Ok(())
}

fn check_disk_space(image_type: &ImageType, disk: &Option<DiskPartition>) -> Result<()> {
    let max_size: u64 = match image_type {
        ImageType::UpgradeImage => 6,
        ImageType::VMRepo => disk.as_ref().and_then(|d| d.img_size).unwrap_or(20) as u64 + 5,
        ImageType::PxeRepo => 5,
        _ => bail!("Invalid image type: {:?}", image_type),
    };

    let current_dir = env::current_dir().expect("Failed to get current directory");
    debug!("Current Directory: {}", current_dir.display());
    let total_space = total_space(&current_dir).expect("Failed to get total space");
    let available_space = available_space(&current_dir).expect("Failed to get available space");
    let free_space = free_space(&current_dir).expect("Failed to get free space");
    debug!("Total space: {} bytes", total_space);
    debug!("Available space: {} bytes", available_space);
    debug!("Free space: {} bytes", free_space);

    if available_space < max_size * 1024 * 1024 * 1024 {
        bail!(
            "Not enough space to create image, available space: {} GiB, required space: {} GiB",
            available_space / 1024 / 1024 / 1024,
            max_size
        );
    }

    Ok(())
}

fn check_repo_file_valid(repo_path: &PathBuf) -> Result<()> {
    utils::is_file_valid("REPO file", repo_path)
}

fn check_agent_file_valid(agent_path: &PathBuf) -> Result<()> {
    utils::is_file_valid("os-agent binary", agent_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_check_disk_space_vm_repo() {
        init();
        let image_type = "vm-repo".into();
        let result = check_disk_space(&image_type, &None);
        assert!(result.is_ok());
    }
}
