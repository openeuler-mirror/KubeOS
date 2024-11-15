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

use std::{fs::File, path::PathBuf, process::Command, str};

use anyhow::bail;

use crate::{
    commands::{DockerImgInfo, ImageType},
    scripts_gen::*,
    utils::{self, set_permissions},
    values::*,
    Config, CreateImage,
};

impl CreateImage for DockerImgInfo {
    fn prepare(&self, config: &mut Config) -> anyhow::Result<()> {
        if self.legacy_bios && self.arch.as_deref() == Some("aarch64") {
            bail!("aarch64 does not support legacy bios");
        }
        if config.dm_verity.is_some() {
            bail!("dm-verity is not supported for from_dockerimg");
        }
        let image_name = &self.upgrade_img;
        verify_docker_input(&image_name)?;
        check_docker_image(&image_name)?;
        Ok(())
    }

    fn generate_scripts(&self, config: &Config) -> anyhow::Result<PathBuf> {
        let kbimg_path = format!("{}/{}", SCRIPTS_DIR, KBIMG_SH);
        let mut kbimg = File::create(&kbimg_path)?;
        base_gen(&mut kbimg, GLOBAL_VARS, true)?;
        gen_docker_vars(&mut kbimg, &self.upgrade_img)?;
        gen_global_func(&mut kbimg)?;
        gen_mount_proc_dev_sys(&mut kbimg)?;
        gen_unmount_dir(&mut kbimg)?;
        gen_create_os_tar_from_docker(&mut kbimg)?;
        if self.image_type == Some(ImageType::VMDocker) {
            write_bootloader(self.arch.as_deref().unwrap(), self.legacy_bios)?;
            gen_init_partition(&mut kbimg)?;
            gen_set_partuuid(&mut kbimg, self.legacy_bios, config.dm_verity.is_some())?;
            gen_create_img(&mut kbimg, self.legacy_bios, &config)?;
            gen_create_vm_docker_img(&mut kbimg)?;
        } else {
            gen_create_pxe_docker_img(&mut kbimg)?;
        }
        set_permissions(&kbimg_path, EXEC_PERMISSION)?;

        Ok(PathBuf::from(&format!("{}/{}", SCRIPTS_DIR, KBIMG_SH)))
    }
}

fn verify_docker_input(image_name: &str) -> anyhow::Result<()> {
    if !utils::is_valid_param(image_name) {
        bail!("params {} is invalid, please check input", image_name);
    }
    Ok(())
}

fn check_docker_image(image_name: &str) -> anyhow::Result<()> {
    let output = Command::new("docker")
        .args(&["images", "-q", image_name])
        .output()
        .expect("Failed to execute command: docker images -q {img_name}");

    if output.status.success() {
        let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8 output");
        if stdout.trim().is_empty() {
            bail!("docker image does NOT exist, please pull {} first.", image_name);
        }
    }
    Ok(())
}
