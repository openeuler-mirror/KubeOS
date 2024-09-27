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

use std::{fs, path::PathBuf, process::exit};

use anyhow::{bail, Result};
use clap::Parser;
use env_logger::{Builder, Env, Target};
use log::{debug, error, info};

mod admin_container;
mod commands;
mod docker_img;
mod repo;
mod scripts_gen;
mod utils;
mod values;

use utils::{execute_scripts, get_arch};
use values::SCRIPTS_DIR;

use crate::commands::{Cli, Commands, Config};

trait CreateImage {
    /// validate cmd args, check disk size and other prepare work
    fn prepare(&self, config: &mut Config) -> Result<()>;
    /// generate scripts for creating image. If debug is enabled, keep the scripts, otherwise execute them
    fn generate_scripts(&self, config: &Config) -> Result<PathBuf>;
}

fn process(info: Box<dyn CreateImage>, mut config: Config) -> Result<()> {
    match fs::create_dir_all(SCRIPTS_DIR) {
        Ok(_) => {
            info.prepare(&mut config)?;
            let path = info.generate_scripts(&config)?;
            execute_scripts(path)?;
            Ok(())
        },
        Err(e) => bail!(e),
    }
}

fn main() {
    let cli = Cli::parse();
    let default_log_level: &str = if cli.debug { "debug" } else { "info" };
    Builder::from_env(Env::default().default_filter_or(default_log_level)).target(Target::Stdout).init();
    match cli.config {
        Some(config) => {
            info!("Loading config file");
            debug!("Config file path: {:?}", config);
            let content = fs::read_to_string(config).unwrap();
            let data: Config = toml::from_str(&content).unwrap();
            debug!("Config: {:?}", data);
            let info = if let Some(mut info) = data.from_repo.clone() {
                info.arch = Some(get_arch());
                Some(Box::new(info) as Box<dyn CreateImage>)
            } else if let Some(info) = data.from_dockerimg.clone() {
                Some(Box::new(info) as Box<dyn CreateImage>)
            } else if let Some(info) = data.admin_container.clone() {
                Some(Box::new(info) as Box<dyn CreateImage>)
            } else {
                None
            };
            if let Some(i) = info {
                match process(i, data) {
                    Ok(_) => {
                        info!("Image created successfully");
                    },
                    Err(e) => {
                        error!("Failed to create image: {:?}", e);
                    },
                }
            }
            exit(0);
        },
        None => {},
    }
    let info = match cli.commands {
        Some(Commands::UpgradeImage(mut info)) => {
            info.image_type = "upgrade".to_string();
            Some(Box::new(info) as Box<dyn CreateImage>)
        },
        Some(Commands::VMRepo(mut info)) => {
            info.image_type = "vm-repo".to_string();
            debug!("VMRepo: {:?}", info);
            Some(Box::new(info) as Box<dyn CreateImage>)
        },
        Some(Commands::VMDocker(mut info)) => {
            info.image_type = "vm-docker".to_string();
            Some(Box::new(info) as Box<dyn CreateImage>)
        },
        Some(Commands::PxeRepo(mut info)) => {
            info.image_type = "pxe-repo".to_string();
            Some(Box::new(info) as Box<dyn CreateImage>)
        },
        Some(Commands::PxeDocker(mut info)) => {
            info.image_type = "pxe-docker".to_string();
            Some(Box::new(info) as Box<dyn CreateImage>)
        },
        Some(Commands::AdminContainer(info)) => Some(Box::new(info) as Box<dyn CreateImage>),
        None => None,
    };
    if let Some(i) = info {
        match process(i, Config::default()) {
            Ok(_) => {
                info!("Image created successfully");
            },
            Err(e) => {
                error!("Failed to create image: {:?}", e);
            },
        }
    }
}
