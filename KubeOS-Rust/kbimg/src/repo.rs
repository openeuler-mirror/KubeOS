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
    env,
    fs::{create_dir_all, File},
    path::PathBuf,
};

use anyhow::bail;
use sysinfo::{System, SystemExt, DiskExt};

use crate::{
    commands::RepoInfo,
    scripts_gen::*,
    utils::{self, check_pxe_conf_valid, set_permissions},
    values::*,
    Config, CreateImage,
};

impl CreateImage for RepoInfo {
    fn prepare(&self, config: &mut Config) -> anyhow::Result<()> {
        verify_repo_input(&self)?;
        check_disk_space(&self.image_type)?;
        check_repo_file_valid(&self.repo_path)?;
        check_agent_file_valid(&self.agent_path)?;
        if self.image_type == "pxe-repo" {
            if let Some(pxe_config) = &config.pxe_config {
                check_pxe_conf_valid(&pxe_config)?;
            } else {
                bail!("pxe config not found!")
            }
        }
        Ok(())
    }

    fn generate_scripts(&self, config: &Config) -> anyhow::Result<PathBuf> {
        // rpmlist
        let rpmlist_path = format!("{}/{}", SCRIPTS_DIR, RPMLIST);
        let mut rpmlist = File::create(&rpmlist_path)?;
        gen_rpm_list(&mut rpmlist, &self.rpmlist)?;
        set_permissions(&rpmlist_path, CONFIG_PERMISSION)?;
        // 00bootup
        match create_dir_all(BOOTUP_DIR) {
            Ok(_) => {
                if let Some(pxe_config) = &config.pxe_config {
                    let global_cfg_path = format!("{}/{}", BOOTUP_DIR, BOOTUP_GLOBAL_CFG);
                    let mut global_cfg = File::create(&global_cfg_path)?;
                    gen_global_cfg(&mut global_cfg, &pxe_config)?;
                    set_permissions(&global_cfg_path, CONFIG_PERMISSION)?;
                }
                let module_setup_path = format!("{}/{}", BOOTUP_DIR, BOOTUP_MODULE_SETUP_SH);
                let mut module_setup = File::create(&module_setup_path)?;
                gen_module_setup(&mut module_setup)?;
                set_permissions(&module_setup_path, EXEC_PERMISSION)?;
                let mount_path = format!("{}/{}", BOOTUP_DIR, BOOTUP_MOUNT_SH);
                let mut mount = File::create(&mount_path)?;
                gen_mount(&mut mount)?;
                set_permissions(&mount_path, EXEC_PERMISSION)?;
            },
            Err(e) => {
                bail!(e);
            },
        }
        // misc-files
        match create_dir_all(MISC_FILES_DIR) {
            Ok(_) => {
                let boot_efi_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_BOOT_EFI_MOUNT);
                let mut boot_efi_mount = File::create(&boot_efi_mount_path)?;
                gen_boot_efi_mount(&mut boot_efi_mount)?;
                set_permissions(&boot_efi_mount_path, CONFIG_PERMISSION)?;
                let boot_grub2_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_BOOT_GRUB2_MOUNT);
                let mut boot_grub2_mount = File::create(&boot_grub2_mount_path)?;
                gen_boot_grub2_mount(&mut boot_grub2_mount)?;
                set_permissions(&boot_grub2_mount_path, CONFIG_PERMISSION)?;
                let etc_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_ETC_MOUNT);
                let mut etc_mount = File::create(&etc_mount_path)?;
                gen_etc_mount(&mut etc_mount)?;
                set_permissions(&etc_mount_path, CONFIG_PERMISSION)?;
                let os_agent_service_path = format!("{}/{}", MISC_FILES_DIR, MISC_OS_AGENT_SERVICE);
                let mut os_agent_service = File::create(&os_agent_service_path)?;
                gen_os_agent_service(&mut os_agent_service)?;
                set_permissions(&os_agent_service_path, CONFIG_PERMISSION)?;
                let os_release_path = format!("{}/{}", MISC_FILES_DIR, MISC_OS_RELEASE);
                let mut os_release = File::create(&os_release_path)?;
                gen_os_release(&mut os_release)?;
                set_permissions(&os_release_path, CONFIG_PERMISSION)?;
                let persist_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_PERSIST_MOUNT);
                let mut persist_mount = File::create(&persist_mount_path)?;
                gen_persist_mount(&mut persist_mount)?;
                set_permissions(&persist_mount_path, CONFIG_PERMISSION)?;
                let var_mount_path = format!("{}/{}", MISC_FILES_DIR, MISC_VAR_MOUNT);
                let mut var_mount = File::create(&var_mount_path)?;
                gen_var_mount(&mut var_mount)?;
                set_permissions(&var_mount_path, CONFIG_PERMISSION)?;
            },
            Err(e) => {
                bail!(e);
            },
        }
        // grub.cfg
        let grub_cfg_path = format!("{}/{}", SCRIPTS_DIR, GRUB_CFG);
        let mut grub_cfg = File::create(&grub_cfg_path)?;
        gen_grub_cfg(&mut grub_cfg)?;
        set_permissions(&grub_cfg_path, CONFIG_PERMISSION)?;
        // set_in_chroot.sh
        let set_in_chroot_path = format!("{}/{}", SCRIPTS_DIR, SET_IN_CHROOT_SH);
        let mut set_in_chroot = File::create(&set_in_chroot_path)?;
        gen_set_in_chroot(&mut set_in_chroot, self.legacy_bios, &config)?;
        set_permissions(&set_in_chroot_path, EXEC_PERMISSION)?;
        // kbimg.sh
        let kbimg_path = format!("{}/{}", SCRIPTS_DIR, KBIMG_SH);
        let mut kbimg = File::create(&kbimg_path)?;
        gen_global_vars(&mut kbimg)?;
        gen_repo_vars(&mut kbimg, &self)?;
        gen_global_func(&mut kbimg)?;
        gen_mount_proc_dev_sys(&mut kbimg)?;
        gen_unmount_dir(&mut kbimg)?;
        gen_create_os_tar_from_repo(&mut kbimg, &self, &config)?;
        if self.image_type == "vm-repo" {
            // bootloader.sh
            let bootloader_path = format!("{}/{}", SCRIPTS_DIR, BOOTLOADER_SH);
            let mut bootloader = File::create(&bootloader_path)?;
            gen_bootloader(&mut bootloader, self.arch.as_ref().unwrap(), self.legacy_bios)?;
            set_permissions(&bootloader_path, EXEC_PERMISSION)?;
            // kbimg.sh
            gen_init_part(&mut kbimg)?;
            gen_create_img(&mut kbimg, self.legacy_bios, &config)?;
            gen_create_vm_repo_img(&mut kbimg)?;
        } else if self.image_type == "pxe-repo" {
            // kbimg.sh
            gen_create_pxe_repo_img(&mut kbimg)?;
        } else {
            // Dockerfile
            let dockerfile_path = format!("{}/{}", SCRIPTS_DIR, DOCKERFILE);
            let mut dockerfile = File::create(&dockerfile_path)?;
            gen_dockerfile(&mut dockerfile)?;
            set_permissions(&dockerfile_path, CONFIG_PERMISSION)?;
            // kbimg.sh
            gen_create_docker_img(&mut kbimg)?;
        }
        set_permissions(&kbimg_path, EXEC_PERMISSION)?;

        Ok(PathBuf::from(&format!("{}/{}", SCRIPTS_DIR, KBIMG_SH)))
    }
}

fn verify_repo_input(info: &RepoInfo) -> anyhow::Result<()> {
    if !utils::is_valid_param(info.repo_path.to_str().unwrap()) {
        bail!("params {} is invalid, please check input", info.repo_path.to_str().unwrap());
    }
    if !utils::is_valid_param(&info.version) {
        bail!("params {} is invalid, please check input", info.version);
    }
    if !utils::is_valid_param(info.agent_path.to_str().unwrap()) {
        bail!("params {} is invalid, please check input", info.agent_path.to_str().unwrap());
    }
    if let Some(docker_img) = &info.docker_img {
        if !utils::is_valid_param(docker_img) {
            bail!("params {} is invalid, please check input", docker_img);
        }
    }
    Ok(())
}

fn check_disk_space(image_type: &str) -> anyhow::Result<()> {
    let max_size: u64 = match image_type {
        "upgrade" => 6,
        "vm-repo" => 25,
        "pxe-repo" => 5,
        _ => bail!("Invalid image type: {}", image_type),
    };

    let current_dir = env::current_dir().expect("Failed to get current directory");
    let root_dir = current_dir.ancestors().last().expect("Failed to get current directory").to_path_buf();
    let mut sys = System::new_all();
    sys.refresh_all();
    for d in sys.disks() {
        if d.mount_point() == root_dir {
            if d.available_space() < max_size * 1024 * 1024 {
                bail!("The available disk space is not enough, at least {}GiB.", max_size);
            }
        }
    }
    Ok(())
}

fn check_repo_file_valid(repo_path: &PathBuf) -> anyhow::Result<()> {
    utils::is_file_valid("REPO file", repo_path)
}

fn check_agent_file_valid(agent_path: &PathBuf) -> anyhow::Result<()> {
    utils::is_file_valid("os-agent binary", agent_path)
}
