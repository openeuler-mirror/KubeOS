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

use std::{fs, path::PathBuf, process::exit};

use anyhow::Result;
use clap::Parser;
use env_logger::{Builder, Env, Target};
use log::{debug, error, info};

mod admin_container;
mod commands;
mod custom;
mod docker_img;
mod repo;
mod scripts_gen;
mod utils;
mod values;

use utils::{check_config_toml, execute_scripts, get_arch};
use values::{DIR_PERMISSION, SCRIPTS_DIR};

use crate::commands::{Cli, Config};

trait CreateImage {
    /// validate cmd args, check disk size and other prepare work
    fn prepare(&self, config: &mut Config) -> Result<()>;
    /// generate scripts for creating image. If debug is enabled, just generate the scripts without execution.
    fn generate_scripts(&self, config: &Config) -> Result<PathBuf>;
}

fn process(info: Box<dyn CreateImage>, mut config: Config, debug: bool) -> Result<()> {
    let dir = PathBuf::from(SCRIPTS_DIR);
    let lock = dir.join("test.lock");
    if lock.exists() {
        error!("It looks like another kbimg process is running. Please wait it to finish.");
        exit(1);
    }
    if dir.exists() {
        debug!("Removing existing scripts directory");
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    utils::set_permissions(&dir, DIR_PERMISSION)?;
    info.prepare(&mut config)?;
    let path = info.generate_scripts(&config)?;
    if !debug {
        execute_scripts(path)?;
        info!("Image created successfully");
    } else {
        debug!("Executed following command to generate KubeOS image: bash {:?}", path);
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let default_log_level: &str = if cli.debug { "debug" } else { "info" };
    Builder::from_env(Env::default().default_filter_or(default_log_level)).target(Target::Stdout).init();

    let arch = get_arch().expect("Failed to get architecture");
    debug!("Architecture: {:?}", arch);
    let (create_type, config) = match cli.commands {
        commands::Commands::Create { image_type, file } => (image_type, file),
    };
    debug!("Config file path: {:?}", config);
    let content = fs::read_to_string(config).expect("Failed to read config file");
    let data: Config = match toml::from_str(&content) {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to parse config file: {}", e);
            exit(1);
        },
    };
    debug!("Config: {:?}", data);

    let info;
    match create_type {
        commands::CreateType::VM => {
            check_config_toml(&data).unwrap();
            if let Some(mut i) = data.from_repo.clone() {
                i.arch = Some(arch);
                i.image_type = Some(commands::ImageType::VMRepo);
                info = Some(Box::new(i) as Box<dyn CreateImage>)
            } else if let Some(mut i) = data.from_dockerimg.clone() {
                i.arch = Some(arch);
                i.image_type = Some(commands::ImageType::VMDocker);
                info = Some(Box::new(i) as Box<dyn CreateImage>)
            } else {
                error!("Missing required fields in config file for creating vm image");
                exit(1);
            }
        },
        commands::CreateType::PXE => {
            check_config_toml(&data).unwrap();
            if let Some(mut i) = data.from_repo.clone() {
                i.arch = Some(arch);
                i.image_type = Some(commands::ImageType::PxeRepo);
                info = Some(Box::new(i) as Box<dyn CreateImage>)
            } else if let Some(mut i) = data.from_dockerimg.clone() {
                i.arch = Some(arch);
                i.image_type = Some(commands::ImageType::PxeDocker);
                info = Some(Box::new(i) as Box<dyn CreateImage>)
            } else {
                error!("Missing required fields in config file for creating pxe image");
                exit(1);
            }
        },
        commands::CreateType::Upgrade => {
            if let Some(mut i) = data.from_repo.clone() {
                i.arch = Some(arch);
                i.image_type = Some(commands::ImageType::UpgradeImage);
                info = Some(Box::new(i) as Box<dyn CreateImage>)
            } else {
                error!("Missing from_repo in config file for creating upgrade image");
                exit(1);
            }
        },
        commands::CreateType::AdminContainer => {
            if let Some(i) = data.admin_container.clone() {
                info = Some(Box::new(i) as Box<dyn CreateImage>)
            } else {
                error!("Missing admin_container in config file for creating admin container image");
                exit(1);
            }
        },
    }

    if let Some(i) = info {
        if let Err(e) = process(i, data, cli.debug) {
            error!("Failed to create image: {:?}", e);
            exit(1);
        }
    }
    exit(0);
}
