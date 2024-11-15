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
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};

use crate::commands::Config;

pub(crate) fn execute_scripts(script: PathBuf) -> Result<()> {
    if !script.exists() {
        bail!("Script does not exist: {:?}", script);
    }
    let status = Command::new("bash").arg(&script).status()?;
    if !status.success() {
        bail!("Failed to execute script: {}\n", script.display());
    }
    Ok(())
}

pub(crate) fn set_permissions<P: AsRef<Path>>(path: P, permission_value: u32) -> Result<()> {
    let metadata = fs::metadata(&path)?;
    let mut permissions = metadata.permissions();
    permissions.set_mode(permission_value);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

/// Check if the input parameter is valid
pub(crate) fn is_valid_param<S: AsRef<str> + std::fmt::Debug>(param: S) -> bool {
    let special_chars = vec!["|", ";", "&", "&&", "||", ">", ">>", "<", ",", "#", "!", "$"];
    !param.as_ref().chars().any(|c| special_chars.contains(&c.to_string().as_str()))
}

/// Check if the path exists and is indeed a file
pub(crate) fn is_file_valid(msg: &str, path: &PathBuf) -> Result<()> {
    if !path.exists() {
        bail!("{} does not exist: {:?}", msg, path);
    }
    if !path.is_file() {
        bail!("{} exists but is not a file: {:?}", msg, path);
    }
    Ok(())
}

/// Check if addr is valid
pub(crate) fn is_addr_valid(addr: &str) -> bool {
    let ip_pattern = regex::Regex::new(r"^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$").unwrap();
    if !ip_pattern.is_match(addr) {
        return false;
    }

    for quad in addr.split('.') {
        if let Ok(num) = quad.parse::<u32>() {
            if num <= 255 {
                continue;
            }
        }
        return false;
    }

    true
}

/// Get architecture
pub(crate) fn get_arch() -> Result<String> {
    let output = std::process::Command::new("arch").output().expect("Failed to execute `arch` command");
    let arch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if arch != "x86_64" && arch != "aarch64" {
        bail!("Unsupported architecture: {}", arch);
    }
    Ok(arch)
}

pub(crate) fn check_config_toml(config: &Config) -> Result<()> {
    if config.from_repo.is_some() && config.from_dockerimg.is_some() {
        bail!("Both from_repo and from_dockerimg are provided in config file. Please provide only one of them")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_is_valid_param() {
        init();
        assert_eq!(is_valid_param("test"), true);
        assert_eq!(is_valid_param("test|test"), false);
        assert_eq!(is_valid_param("test;test"), false);
        assert_eq!(is_valid_param("test&test"), false);
        assert_eq!(is_valid_param("test&&test"), false);
        assert_eq!(is_valid_param("test||test"), false);
        assert_eq!(is_valid_param("test>test"), false);
        assert_eq!(is_valid_param("test>>test"), false);
        assert_eq!(is_valid_param("test<test"), false);
        assert_eq!(is_valid_param("test,test"), false);
        assert_eq!(is_valid_param("test#test"), false);
        assert_eq!(is_valid_param("test!test"), false);
        assert_eq!(is_valid_param("test$test"), false);
    }
}
