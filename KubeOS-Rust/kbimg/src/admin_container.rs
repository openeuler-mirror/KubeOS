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
    fs::{create_dir_all, File},
    path::PathBuf,
};

use anyhow::{bail, Result};

use crate::{
    commands::AdminContainerInfo,
    scripts_gen::*,
    utils::{self, set_permissions},
    values::*,
    Config, CreateImage,
};

impl CreateImage for AdminContainerInfo {
    fn prepare(&self, _: &mut Config) -> Result<()> {
        let hostshell = &self.hostshell;
        let image_name = &self.img_name;
        verify_admin_input(hostshell, &image_name)?;
        utils::is_file_valid("admin-container hosthshell", hostshell)?;
        Ok(())
    }

    fn generate_scripts(&self, _: &Config) -> Result<PathBuf> {
        create_dir_all(ADMIN_CONTAINER_DIR)?;
        utils::set_permissions(ADMIN_CONTAINER_DIR, DIR_PERMISSION)?;
        self.write_dockerfile()?;
        self.write_set_ssh_pub_key_service()?;
        self.write_set_ssh_pub_key_sh()?;

        let kbimg_path = format!("{}/{}", SCRIPTS_DIR, KBIMG_SH);
        let mut kbimg = File::create(&format!("{}/{}", SCRIPTS_DIR, KBIMG_SH))?;
        gen_admin_vars(&mut kbimg, &self.img_name, &self.hostshell)?;
        gen_test_lock(&mut kbimg)?;
        gen_create_admin_img(&mut kbimg)?;
        set_permissions(&kbimg_path, EXEC_PERMISSION)?;

        Ok(PathBuf::from(&format!("{}/{}", SCRIPTS_DIR, KBIMG_SH)))
    }
}

impl AdminContainerInfo {
    fn write_dockerfile(&self) -> Result<()> {
        let dockerfile_path = format!("{}/{}", ADMIN_CONTAINER_DIR, ADMIN_DOCKERFILE);
        let mut dockerfile = File::create(&dockerfile_path)?;
        base_gen(&mut dockerfile, ADMIN_DOCKERFILE_CONTENT, false)?;
        set_permissions(&dockerfile_path, CONFIG_PERMISSION)?;
        Ok(())
    }

    fn write_set_ssh_pub_key_service(&self) -> Result<()> {
        let set_ssh_pub_key_service_path = format!("{}/{}", ADMIN_CONTAINER_DIR, ADMIN_SET_SSH_PUB_KEY_SERVICE);
        let mut set_ssh_pub_key_service = File::create(&set_ssh_pub_key_service_path)?;
        base_gen(&mut set_ssh_pub_key_service, SET_SSH_PUB_KEY_SERVICE, false)?;
        set_permissions(&set_ssh_pub_key_service_path, CONFIG_PERMISSION)?;
        Ok(())
    }
    fn write_set_ssh_pub_key_sh(&self) -> Result<()> {
        let set_ssh_pub_key_path = format!("{}/{}", ADMIN_CONTAINER_DIR, ADMIN_SET_SSH_PUB_KEY_SH);
        let mut set_ssh_pub_key = File::create(&set_ssh_pub_key_path)?;
        base_gen(&mut set_ssh_pub_key, SET_SSH_PUB_KEY_SH, true)?;
        set_permissions(&set_ssh_pub_key_path, EXEC_PERMISSION)?;
        Ok(())
    }
}

fn verify_admin_input(hostshell: &PathBuf, image_name: &str) -> Result<()> {
    if !utils::is_valid_param(hostshell.to_str().unwrap()) {
        bail!("params {} is invalid, please check input", hostshell.to_str().unwrap());
    }
    if !utils::is_valid_param(image_name) {
        bail!("params {} is invalid, please check input", image_name);
    }
    Ok(())
}
