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

use std::{fs::File, path::PathBuf};

use anyhow::{anyhow, bail, Result};
use log::debug;

use crate::{
    commands::InstallConfig,
    scripts_gen::*,
    utils::{self, set_permissions},
    values::*,
    Config,
};

pub(crate) trait InstallImage {
    fn prepare(&self, config: &Config) -> Result<()>;
    fn generate_scripts(&self, config: &Config) -> Result<PathBuf>;
}

impl InstallImage for InstallConfig {
    fn prepare(&self, config: &Config) -> Result<()> {
        if config.from_repo.is_none() {
            bail!("from_repo is required in config file for install");
        }
        let repo_info = config
            .from_repo
            .as_ref()
            .ok_or_else(|| anyhow!("from_repo is required in config file for install"))?;
        let arch = repo_info.arch.as_deref().unwrap_or("");
        if arch != "x86_64" && arch != "aarch64" {
            bail!("Unsupported architecture for install: {:?}", repo_info.arch);
        }

        if !utils::is_valid_param(&self.target_disk) {
            bail!(
                "params {} is invalid, please check input",
                self.target_disk
            );
        }
        if !utils::is_valid_param(&self.oci_image) {
            bail!(
                "params {} is invalid, please check input",
                self.oci_image
            );
        }
        for c in &self.configs {
            if !utils::is_valid_param(&c.src) || !utils::is_valid_param(&c.dst) {
                bail!("params in configs {:?} is invalid, please check input", c);
            }
        }

        if config.dm_verity.is_some() {
            bail!("dm-verity is not supported for disk install");
        }

        debug!("Install config validated successfully");
        Ok(())
    }

    fn generate_scripts(&self, config: &Config) -> Result<PathBuf> {
        let install_path = format!("{}/{}", SCRIPTS_DIR, "install.sh");
        let mut install_script = File::create(&install_path)?;
        base_gen(&mut install_script, INSTALL_GLOBAL_VARS, true)?;
        gen_global_func(&mut install_script)?;
        gen_install_script(&mut install_script, self, config)?;
        set_permissions(&install_path, EXEC_PERMISSION)?;
        Ok(PathBuf::from(&install_path))
    }
}
