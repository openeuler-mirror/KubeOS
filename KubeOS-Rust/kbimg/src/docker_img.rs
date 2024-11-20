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

use std::{fs::File, path::PathBuf, process::Command, str};

use anyhow::bail;

use crate::{
    commands::DockerInfo,
    scripts_gen::*,
    utils::{self, set_permissions},
    values::*,
    Config, CreateImage,
};

impl CreateImage for DockerInfo {
    fn prepare(&self, _: &mut Config) -> anyhow::Result<()> {
        let image_name = &self.docker_img;
        verify_docker_input(&image_name)?;
        check_docker_image(&image_name)?;
        Ok(())
    }

    fn generate_scripts(&self, config: &Config) -> anyhow::Result<PathBuf> {
        // kbimg.sh
        let kbimg_path = format!("{}/{}", SCRIPTS_DIR, KBIMG_SH);
        let mut kbimg = File::create(&kbimg_path)?;
        gen_global_vars(&mut kbimg)?;
        gen_docker_vars(&mut kbimg, &self.docker_img)?;
        gen_global_func(&mut kbimg)?;
        gen_create_os_tar_from_docker(&mut kbimg)?;
        if self.image_type == "vm-docker" {
            // kbimg.sh
            gen_init_part(&mut kbimg)?;
            gen_create_img(&mut kbimg, false, &config)?;
            gen_create_vm_docker_img(&mut kbimg)?;
        } else {
            // kbimg.sh
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
    let output =
        Command::new("docker").args(&["images", "-q", image_name]).output().expect("Failed to execute command");

    if output.status.success() {
        let stdout = str::from_utf8(&output.stdout).expect("Invalid UTF-8 output");
        if stdout.trim().is_empty() {
            bail!("docker image does NOT exist, please pull {} first.", image_name);
        }
    }
    Ok(())
}