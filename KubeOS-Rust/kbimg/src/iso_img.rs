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

use std::{env, fs::File, io::Write, path::PathBuf};

use anyhow::{bail, Result};
use fs2::available_space;
use log::debug;

use crate::{
    commands::IsoImgInfo,
    scripts_gen::*,
    utils::{self, set_permissions},
    values::*,
    Config, CreateImage,
};

impl CreateImage for IsoImgInfo {
    fn prepare(&self, _config: &mut Config) -> Result<()> {
        verify_iso_input(&self.oci_image)?;
        verify_iso_input(&self.iso_image)?;
        verify_iso_input(self.elemental_cli_path.to_str().unwrap_or(""))?;
        utils::is_file_valid("elemental-cli binary", &self.elemental_cli_path)?;
        if let Some(output_dir) = &self.output_dir {
            if !utils::is_valid_param(output_dir) {
                bail!("params {} is invalid, please check input", output_dir);
            }
        }
        if let Some(ref entry) = self.grub_entry_name {
            if !utils::is_valid_param(entry) {
                bail!("params {} is invalid, please check input", entry);
            }
        }
        if let Some(ref name) = self.name {
            if !utils::is_valid_param(name) {
                bail!("params {} is invalid, please check input", name);
            }
        }
        check_iso_disk_space()?;
        Ok(())
    }

    fn generate_scripts(&self, config: &Config) -> Result<PathBuf> {
        // Write the ISO manifest.yaml for elemental build-iso
        write_iso_manifest(self)?;

        // Write the ISO Dockerfile (base image + elemental init)
        write_iso_dockerfile(self, config.from_repo.as_ref())?;

        // Generate the main kbimg.sh script
        let kbimg_path = format!("{}/{}", SCRIPTS_DIR, KBIMG_SH);
        let mut kbimg = File::create(&kbimg_path)?;
        writeln!(kbimg, "#!/bin/bash")?;
        gen_copyright(&mut kbimg)?;
        gen_iso_vars(&mut kbimg, self, config.from_repo.as_ref())?;
        gen_test_lock(&mut kbimg)?;
        gen_create_iso_image(&mut kbimg)?;
        set_permissions(&kbimg_path, EXEC_PERMISSION)?;

        Ok(PathBuf::from(&kbimg_path))
    }
}

fn verify_iso_input(oci_image: &str) -> Result<()> {
    if !utils::is_valid_param(oci_image) {
        bail!("params {} is invalid, please check input", oci_image);
    }
    Ok(())
}

fn check_iso_disk_space() -> Result<()> {
    let max_size: u64 = 5; // ISO image needs ~5 GiB
    let current_dir = env::current_dir().expect("Failed to get current directory");
    debug!("Current Directory: {}", current_dir.display());
    let available = available_space(&current_dir).expect("Failed to get available space");
    debug!("Available space: {} bytes", available);
    if available < max_size * 1024 * 1024 * 1024 {
        bail!(
            "Not enough space to create iso image, available space: {} GiB, required space: {} GiB",
            available / 1024 / 1024 / 1024,
            max_size
        );
    }
    Ok(())
}
