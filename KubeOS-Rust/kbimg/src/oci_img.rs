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

use std::{env, fs::File, path::PathBuf};

use anyhow::{anyhow, bail, Result};
use fs2::available_space;
use log::debug;

use crate::{
    commands::OciImgInfo,
    scripts_gen::*,
    utils::{self, set_permissions},
    values::*,
    Config, CreateImage,
};

impl CreateImage for OciImgInfo {
    fn prepare(&self, config: &mut Config) -> Result<()> {
        let repo_info = config
            .from_repo
            .as_ref()
            .ok_or_else(|| anyhow!("from_repo is required in config file for creating oci image"))?;

        let arch = repo_info.arch.as_deref().unwrap_or("");
        if arch != "x86_64" && arch != "aarch64" {
            bail!("Unsupported architecture for oci image: {:?}", repo_info.arch);
        }
        if repo_info.legacy_bios && arch == "aarch64" {
            bail!("aarch64 does not support legacy bios");
        }
        if config.dm_verity.is_some() {
            bail!("dm-verity is not supported for oci image");
        }

        verify_oci_input(&self.image_name)?;
        utils::is_file_valid("os-agent binary", &repo_info.agent_path)?;
        check_repo_file_valid(&repo_info.repo_path)?;

        check_oci_disk_space()?;
        Ok(())
    }

    fn generate_scripts(&self, config: &Config) -> Result<PathBuf> {
        let repo_info = config
            .from_repo
            .as_ref()
            .ok_or_else(|| anyhow!("from_repo is required in config file for creating oci image"))?;

        // Reuse the repo-based rootfs preparation (rpmlist, misc files, grub.cfg, chroot script)
        repo_info.write_rpmlist(config)?;
        repo_info.write_misc_files()?;
        repo_info.write_security_files(config)?;
        repo_info.write_grub_cfg(config.dm_verity.is_some())?;
        repo_info.write_set_in_chroot(config)?;

        // Write the OCI-specific Dockerfile (rootfs-only, no /os.tar)
        write_oci_dockerfile()?;

        // Generate the main kbimg.sh script
        let kbimg_path = format!("{}/{}", SCRIPTS_DIR, KBIMG_SH);
        let mut kbimg = File::create(&kbimg_path)?;
        base_gen(&mut kbimg, GLOBAL_VARS, true)?;
        gen_oci_vars(&mut kbimg, repo_info, self, &config.grub)?;
        gen_global_func(&mut kbimg)?;
        gen_mount_proc_dev_sys(&mut kbimg)?;
        gen_unmount_dir(&mut kbimg)?;
        gen_create_oci_image(&mut kbimg, repo_info, config)?;
        set_permissions(&kbimg_path, EXEC_PERMISSION)?;

        Ok(PathBuf::from(&kbimg_path))
    }
}

fn verify_oci_input(image_name: &str) -> Result<()> {
    if !utils::is_valid_param(image_name) {
        bail!("params {} is invalid, please check input", image_name);
    }
    Ok(())
}

fn check_oci_disk_space() -> Result<()> {
    let max_size: u64 = 10; // OCI image needs ~10 GiB (rootfs + tar + docker image)
    let current_dir = env::current_dir().expect("Failed to get current directory");
    debug!("Current Directory: {}", current_dir.display());
    let available = available_space(&current_dir).expect("Failed to get available space");
    debug!("Available space: {} bytes", available);
    if available < max_size * 1024 * 1024 * 1024 {
        bail!(
            "Not enough space to create oci image, available space: {} GiB, required space: {} GiB",
            available / 1024 / 1024 / 1024,
            max_size
        );
    }
    Ok(())
}

fn check_repo_file_valid(repo_path: &PathBuf) -> Result<()> {
    utils::is_file_valid("REPO file", repo_path)
}
