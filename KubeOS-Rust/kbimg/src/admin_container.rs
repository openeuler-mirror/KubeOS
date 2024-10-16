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
    fs::{create_dir_all, File},
    path::PathBuf,
};

use anyhow::bail;

use crate::{
    commands::AdminContainerInfo,
    scripts_gen::*,
    utils::{self, set_permissions},
    values::*,
    Config, CreateImage,
};

impl CreateImage for AdminContainerInfo {
    fn prepare(&self, _: &mut Config) -> anyhow::Result<()> {
        let dockerfile = &self.dockerfile;
        let image_name = &self.docker_img;
        verify_admin_input(&dockerfile, &image_name)?;
        check_dockerfile_valid(&dockerfile)?;
        Ok(())
    }

    fn generate_scripts(&self, _: &Config) -> anyhow::Result<PathBuf> {
        // admin-container
        match create_dir_all(ADMIN_CONTAINER_DIR) {
            Ok(_) => {
                // Dockerfile
                let dockerfile_path = format!("{}/{}", ADMIN_CONTAINER_DIR, ADMIN_DOCKERFILE);
                let mut dockerfile = File::create(&dockerfile_path)?;
                gen_admin_dockerfile(&mut dockerfile)?;
                set_permissions(&dockerfile_path, CONFIG_PERMISSION)?;
                // set-ssh-pub-key.service
                let set_ssh_pub_key_service_path = format!("{}/{}", ADMIN_CONTAINER_DIR, ADMIN_SET_SSH_PUB_KEY_SERVICE);
                let mut set_ssh_pub_key_service = File::create(&set_ssh_pub_key_service_path)?;
                gen_set_ssh_pub_key_service(&mut set_ssh_pub_key_service)?;
                set_permissions(&set_ssh_pub_key_service_path, CONFIG_PERMISSION)?;
                // set-ssh-pub-key.sh
                let set_ssh_pub_key_path = format!("{}/{}", ADMIN_CONTAINER_DIR, ADMIN_SET_SSH_PUB_KEY_SH);
                let mut set_ssh_pub_key = File::create(&set_ssh_pub_key_path)?;
                gen_set_ssh_pub_key(&mut set_ssh_pub_key)?;
                set_permissions(&set_ssh_pub_key_path, EXEC_PERMISSION)?;
            },
            Err(e) => {
                bail!(e);
            },
        }
        // kbimg.sh
        let kbimg_path = format!("{}/{}", SCRIPTS_DIR, KBIMG_SH);
        let mut kbimg = File::create(&format!("{}/{}", SCRIPTS_DIR, KBIMG_SH))?;
        gen_admin_vars(&mut kbimg, &self.docker_img, &self.dockerfile)?;
        gen_create_admin_img(&mut kbimg)?;
        set_permissions(&kbimg_path, EXEC_PERMISSION)?;

        Ok(PathBuf::from(&format!("{}/{}", SCRIPTS_DIR, KBIMG_SH)))
    }
}

fn verify_admin_input(dockerfile: &PathBuf, image_name: &str) -> anyhow::Result<()> {
    if !utils::is_valid_param(dockerfile.to_str().unwrap()) {
        bail!("params {} is invalid, please check input", dockerfile.to_str().unwrap());
    }
    if !utils::is_valid_param(image_name) {
        bail!("params {} is invalid, please check input", image_name);
    }
    Ok(())
}

fn check_dockerfile_valid(dockerfile: &PathBuf) -> anyhow::Result<()> {
    utils::is_file_valid("admin-container Dockerfile", dockerfile)
}
