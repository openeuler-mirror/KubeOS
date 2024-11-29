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
    collections::HashMap,
    fs::{self, File},
    io::{self, BufRead, BufWriter, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    string::String,
};

use anyhow::{anyhow, bail, Context, Result};
use lazy_static::lazy_static;
use log::{debug, info, trace, warn};
use regex::Regex;
use serde_yaml::Value;
use toml::{map::Map, Table};

use crate::{api::*, sys_mgmt::values, utils::*};

lazy_static! {
    pub static ref CONFIG_TEMPLATE: HashMap<String, Box<dyn Configuration + Sync>> = {
        let mut config_map = HashMap::new();
        config_map.insert(
            values::KERNEL_SYSCTL.to_string(),
            Box::new(KernelSysctl::new(values::DEFAULT_PROC_PATH)) as Box<dyn Configuration + Sync>,
        );
        config_map.insert(
            values::KERNEL_SYSCTL_PERSIST.to_string(),
            Box::new(KernelSysctlPersist) as Box<dyn Configuration + Sync>,
        );
        config_map.insert(
            values::GRUB_CMDLINE_CURRENT.to_string(),
            Box::new(GrubCmdline { grub_path: values::DEFAULT_GRUB_CFG_PATH.to_string(), is_cur_partition: true })
                as Box<dyn Configuration + Sync>,
        );
        config_map.insert(
            values::GRUB_CMDLINE_NEXT.to_string(),
            Box::new(GrubCmdline { grub_path: values::DEFAULT_GRUB_CFG_PATH.to_string(), is_cur_partition: false })
                as Box<dyn Configuration + Sync>,
        );
        config_map.insert(
            values::KUBERNETES_KUBELET.to_string(),
            Box::new(KubernetesKubelet) as Box<dyn Configuration + Sync>,
        );
        config_map.insert(
            values::CONTAINER_CONTAINERD.to_string(),
            Box::new(ContainerContainerd) as Box<dyn Configuration + Sync>,
        );
        config_map.insert(
            values::PAM_LIMTS.to_string(),
            Box::new(PamLimits { config_path: values::DEFAULT_PAM_LIMITS_PATH.to_string() })
                as Box<dyn Configuration + Sync>,
        );
        config_map
    };
}

pub trait Configuration {
    fn set_config(&self, config: &mut Sysconfig) -> Result<()>;
}

pub struct KernelSysctl {
    pub proc_path: String,
}
pub struct KernelSysctlPersist;
pub struct GrubCmdline {
    pub grub_path: String,
    pub is_cur_partition: bool,
}

pub struct KubernetesKubelet;

pub struct ContainerContainerd;

pub struct PamLimits {
    pub config_path: String,
}

impl Configuration for KernelSysctl {
    fn set_config(&self, config: &mut Sysconfig) -> Result<()> {
        info!("Start setting kernel.sysctl");
        for (key, key_info) in config.contents.iter() {
            let proc_path = self.get_proc_path(key);
            let (key_info_value, is_recognized) = convert_json_value_to_string(&key_info.value);
            if !is_recognized {
                warn!(
                    "Failed to handle keyinfo.value, the type of it is not in range of number, string, boolean, null"
                );
                continue;
            }
            if key_info.operation == "delete" {
                warn!("Failed to delete kernel.sysctl config with key \"{}\"", key);
            } else if !key_info_value.is_empty() && key_info.operation.is_empty() {
                fs::write(&proc_path, format!("{}\n", &key_info_value).as_bytes())
                    .with_context(|| format!("Failed to write kernel.sysctl with key: \"{}\"", key))?;
                info!("Configured kernel.sysctl {}={}", key, key_info_value);
            } else {
                warn!(
                    "Failed to parse kernel.sysctl, key: \"{}\", value: \"{}\", operation: \"{}\"",
                    key, key_info_value, key_info.operation
                );
            }
        }
        Ok(())
    }
}

impl KernelSysctl {
    fn new(proc_path: &str) -> Self {
        Self { proc_path: String::from(proc_path) }
    }

    fn get_proc_path(&self, key: &str) -> PathBuf {
        let path_str = format!("{}{}", self.proc_path, key.replace('.', "/"));
        Path::new(&path_str).to_path_buf()
    }
}

impl Configuration for KernelSysctlPersist {
    fn set_config(&self, config: &mut Sysconfig) -> Result<()> {
        info!("Start setting kernel.sysctl.persist");
        let mut config_path = &values::DEFAULT_KERNEL_CONFIG_PATH.to_string();
        if !config.config_path.is_empty() {
            config_path = &config.config_path;
        }
        debug!("kernel.sysctl.persist config_path: \"{}\"", config_path);
        create_config_file(config_path).with_context(|| format!("Failed to find config path \"{}\"", config_path))?;
        let configs = get_and_set_configs(&mut config.contents, config_path)
            .with_context(|| format!("Failed to set persist kernel configs \"{}\"", config_path))?;
        write_configs_to_file(config_path, &configs).with_context(|| "Failed to write configs to file".to_string())?;
        Ok(())
    }
}

fn create_config_file(config_path: &str) -> Result<()> {
    if !is_file_exist(config_path) {
        let f = fs::File::create(config_path)?;
        let metadata = f.metadata()?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(values::DEFAULT_KERNEL_CONFIG_PERM);
        debug!("Create file {} with permission 0644", config_path);
    }
    Ok(())
}

fn get_and_set_configs(expect_configs: &mut HashMap<String, KeyInfo>, config_path: &str) -> Result<Vec<String>> {
    let f = File::open(config_path).with_context(|| format!("Failed to open config path \"{}\"", config_path))?;
    let mut configs_write = Vec::new();
    for line in io::BufReader::new(f).lines() {
        let line = line?;
        // if line is a comment or blank
        if line.starts_with('#') || line.starts_with(';') || line.trim().is_empty() {
            configs_write.push(line);
            continue;
        }
        let config_kv: Vec<&str> = line.splitn(2, '=').map(|s| s.trim()).collect();
        // if config_kv is not a key-value pair
        if config_kv.len() != 2 {
            bail!("could not parse sysctl config {}", line);
        }
        let new_key_info = expect_configs.get(config_kv[0]);
        let new_config = match new_key_info {
            Some(new_key_info) if new_key_info.operation == "delete" => handle_delete_key(&config_kv, new_key_info),
            Some(new_key_info) => handle_update_key(&config_kv, new_key_info),
            None => config_kv.join("="),
        };
        configs_write.push(new_config);
        expect_configs.remove(config_kv[0]);
    }
    let new_config = handle_add_key(expect_configs, false);
    configs_write.extend(new_config);
    Ok(configs_write)
}

fn write_configs_to_file(config_path: &str, configs: &Vec<String>) -> Result<()> {
    info!("Write configuration to file \"{}\"", config_path);
    let f = File::create(config_path)?;
    let mut w = BufWriter::new(f);
    for line in configs {
        if line.is_empty() {
            continue;
        }
        writeln!(w, "{}", line.as_str())?;
    }
    w.flush().with_context(|| format!("Failed to flush file {}", config_path))?;
    w.get_mut().sync_all().with_context(|| "Failed to sync".to_string())?;
    debug!("Write configuration to file \"{}\" success", config_path);
    Ok(())
}

fn handle_delete_key(config_kv: &[&str], new_config_info: &KeyInfo) -> String {
    let key = config_kv[0];
    let (new_config_info_value, is_recognized) = convert_json_value_to_string(&new_config_info.value);
    if config_kv.len() == values::ONLY_KEY && new_config_info_value.is_empty() {
        info!("Delete configuration key: \"{}\"", key);
        return String::from("");
    } else if config_kv.len() == values::ONLY_KEY && !new_config_info_value.is_empty() {
        warn!("Failed to delete key \"{}\" with inconsistent values \"nil\" and \"{}\"", key, new_config_info_value);
        return key.to_string();
    } else if !is_recognized {
        warn!("Failed to handle keyinfo.value, the type of it is not in range of number, string, boolean, null");
        match config_kv.len() {
            values::ONLY_KEY => return key.to_string(),
            values::KV_PAIR => return config_kv.join("="),
            values::PAM_LIMITS_KV => return config_kv.join(" "),
            _ => return "".to_string(),
        }
    }
    let old_value: String;
    if config_kv.len() == values::PAM_LIMITS_KV {
        let config_str = config_kv[1..].join(".");
        old_value = config_str;
    } else {
        old_value = config_kv[1].to_string();
    }
    if old_value != new_config_info_value {
        warn!(
            "Failed to delete key \"{}\" with inconsistent values \"{}\" and \"{}\"",
            key, old_value, new_config_info_value
        );
        return if config_kv.len() == values::KV_PAIR { config_kv.join("=") } else { config_kv.join(" ") };
    }
    info!("Delete configuration {}={}", key, old_value);
    String::new()
}

fn handle_update_key(config_kv: &[&str], new_config_info: &KeyInfo) -> String {
    let key = config_kv[0];
    if !new_config_info.operation.is_empty() {
        warn!(
            "Unknown operation \"{}\", updating key \"{}\" with value \"{}\" by default",
            new_config_info.operation, key, new_config_info.value
        );
    }
    let (new_config_info_value, is_recognized) = convert_json_value_to_string(&new_config_info.value);
    if config_kv.len() == values::ONLY_KEY && new_config_info_value.is_empty() {
        return key.to_string();
    } else if !is_recognized {
        warn!("Failed to handle keyinfo.value, the type of it is not in range of number, string, boolean, null");
        match config_kv.len() {
            values::ONLY_KEY => return key.to_string(),
            values::KV_PAIR => return config_kv.join("="),
            values::PAM_LIMITS_KV => return config_kv.join(" "),
            _ => return "".to_string(),
        }
    }
    let new_value = new_config_info_value.trim();
    if config_kv.len() == values::ONLY_KEY && !new_config_info_value.is_empty() {
        info!("Update configuration \"{}={}\"", key, new_value);
        return format!("{}={}", key, new_value);
    }
    if new_config_info_value.is_empty() {
        warn!("Failed to update key \"{}\" with \"null\" value", key);
        return if config_kv.len() == values::KV_PAIR { config_kv.join("=") } else { config_kv.join(" ") };
    }

    if config_kv.len() == values::PAM_LIMITS_KV {
        let value_list: Vec<&str> = new_value.split(".").collect();
        if value_list.len() != 3 {
            warn!(
                "Failed to update pam limits key \"{}\" with value {} because of illegal format of value",
                key, new_value
            );
            return config_kv.join(" ");
        }
        let mut new_value_list: Vec<&str> = Vec::new();
        for (i, value) in value_list.iter().enumerate() {
            if value == &"_" {
                new_value_list.push(config_kv[i + 1]);
                continue;
            }
            new_value_list.push(value_list[i]);
        }
        info!("Update configuration \"{} {}\"", key, new_value_list.join(" "));
        return format!("{} {}", key, new_value_list.join(" "));
    }
    info!("Update configuration \"{}={}\"", key, new_value);
    format!("{}={}", key, new_value)
}

fn handle_add_key(expect_configs: &HashMap<String, KeyInfo>, is_only_key_valid: bool) -> Vec<String> {
    let mut configs_write = Vec::new();
    for (key, config_info) in expect_configs.iter() {
        if config_info.operation == "delete" {
            warn!("Failed to delete inexistent key: \"{}\"", key);
            continue;
        }
        if key.is_empty() || key.contains('=') {
            warn!("Failed to add \"null\" key or key containing \"=\", key: \"{}\"", key);
            continue;
        }
        if !config_info.operation.is_empty() {
            warn!(
                "Unknown operation \"{}\", adding key \"{}\" with value \"{}\" by default",
                config_info.operation, key, config_info.value
            );
        }
        let (config_info_value, is_recognized) = convert_json_value_to_string(&config_info.value);
        if !is_recognized {
            warn!("Failed to handle keyinfo.value, the type of it is not in range of number, string, boolean, null");
            continue;
        }
        let (k, v) = (key.trim(), config_info_value.trim());
        if v.is_empty() && is_only_key_valid {
            info!("Add configuration \"{}\"", k);
            configs_write.push(k.to_string());
        } else if v.is_empty() {
            warn!("Failed to add key \"{}\" with \"null\" value", k);
        } else {
            info!("Add configuration \"{}={}\"", k, v);
            configs_write.push(format!("{}={}", k, v));
        }
    }
    configs_write
}

impl Configuration for GrubCmdline {
    fn set_config(&self, config: &mut Sysconfig) -> Result<()> {
        let c = RealCommandExecutor {};
        if is_dmv_mode(&c) {
            warn!("dm-verity mode is enabled, skip setting grub.cmdline configuration");
            return Ok(());
        }
        if self.is_cur_partition {
            info!("Start setting grub.cmdline.current configuration");
        } else {
            info!("Start setting grub.cmdline.next configuration");
        }
        if !is_file_exist(&self.grub_path) {
            bail!("Failed to find grub.cfg file");
        }
        let config_partition = if cfg!(test) {
            self.is_cur_partition
        } else {
            self.get_config_partition(c).with_context(|| "Failed to get config partition".to_string())?
        };
        debug!("Config_partition: {} (false means partition A, true means partition B)", config_partition);
        let configs = get_and_set_grubcfg(&mut config.contents, &self.grub_path, config_partition)
            .with_context(|| "Failed to set grub configs".to_string())?;
        write_configs_to_file(&self.grub_path, &configs)
            .with_context(|| "Failed to write configs to file".to_string())?;
        Ok(())
    }
}

impl GrubCmdline {
    // get_config_partition returns false if the menuentry to be configured is A, true for menuentry B
    fn get_config_partition<T: CommandExecutor>(&self, executor: T) -> Result<bool> {
        let (_, next_partition) = get_partition_info(&executor)?;
        let mut flag = false;
        if next_partition.menuentry == "B" {
            flag = true
        }
        Ok(self.is_cur_partition != flag)
    }
}

fn get_and_set_grubcfg(
    expect_configs: &mut HashMap<String, KeyInfo>,
    grub_path: &str,
    config_partition: bool,
) -> Result<Vec<String>> {
    let f = File::open(grub_path).with_context(|| format!("Failed to open grub.cfg \"{}\"", grub_path))?;
    let re_find_cur_linux = r"^\s*linux.*root=.*";
    let re = Regex::new(re_find_cur_linux)?;
    let mut configs_write = Vec::new();
    let mut match_config_partition = false;
    for line in io::BufReader::new(f).lines() {
        let mut line = line?;
        if re.is_match(&line) {
            if match_config_partition == config_partition {
                line = modify_boot_cfg(expect_configs, &line)?;
            }
            match_config_partition = true;
        }
        configs_write.push(line);
    }
    Ok(configs_write)
}

fn modify_boot_cfg(expect_configs: &mut HashMap<String, KeyInfo>, line: &String) -> Result<String> {
    trace!("Match partition that need to be configured, entering modify_boot_cfg, linux line: {}", line);
    let mut new_configs = vec!["       ".to_string()];
    let olg_configs: Vec<&str> = line.split(' ').collect();
    for old_config in olg_configs {
        if old_config.is_empty() {
            continue;
        }
        // At most 2 substrings can be returned to satisfy the case like root=UUID=xxxx
        let config = old_config.splitn(2, '=').collect::<Vec<&str>>();
        if config.len() != values::ONLY_KEY && config.len() != values::KV_PAIR {
            bail!("Failed to parse grub.cfg linux line {}", old_config);
        }
        let new_key_info = expect_configs.get(config[0]);
        let new_config = match new_key_info {
            Some(new_key_info) if new_key_info.operation == "delete" => handle_delete_key(&config, new_key_info),
            Some(new_key_info) => handle_update_key(&config, new_key_info),
            None => config.join("="),
        };
        if !new_config.is_empty() {
            new_configs.push(new_config);
        }
        expect_configs.remove(config[0]);
    }
    let new_config = handle_add_key(expect_configs, true);
    new_configs.extend(new_config);
    Ok(new_configs.join(" "))
}

fn convert_json_value_to_string(value: &serde_json::Value) -> (String, bool) {
    if value.is_null() {
        return ("".to_string(), true);
    }
    if value.is_string() {
        // Even if value is "", the value will not be none after as_str is executed.
        // Therefore, the value will never be none here. unwrap() is safe.
        return (value.as_str().unwrap().to_string(), true);
    }
    if value.is_number() || value.is_boolean() {
        return (value.to_string(), true);
    }
    return ("".to_string(), false);
}

impl Configuration for KubernetesKubelet {
    fn set_config(&self, config: &mut Sysconfig) -> Result<()> {
        info!("Start setting kubernetes.kubelet");
        let mut config_path = &values::DEFAULT_KUBELET_CONFIG_PATH.to_string();
        if !config.config_path.is_empty() {
            config_path = &config.config_path;
        }
        debug!("kubernetes.kubelet config_path: \"{}\"", config_path);

        create_config_file(config_path).with_context(|| format!("Failed to find config path \"{}\"", config_path))?;
        let file: File = std::fs::File::open(config_path)
            .with_context(|| format!("Failed to open config file \"{}\"", config_path))?;
        let mut value: serde_yaml::Value = serde_yaml::from_reader(file)
            .with_context(|| format!("Failed to read from config file \"{}\"", config_path))?;
        let pattern = Regex::new(r#"[^\."']+|"([^"]*)"|'([^']*)'"#)
            .with_context(|| format!("Failed to create regex used by split key"))?;
        if value.is_null() {
            value = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        }
        for (key, key_info) in config.contents.iter() {
            debug!("Start configuration of key={}", key);
            if key.is_empty() {
                warn!("Failed to add \"null\" key, key: \"{}\"", key);
                continue;
            }
            let key_list: Vec<String> = pattern.find_iter(&key).map(|m| m.as_str().to_string()).collect();
            let mut value_iter = &mut value;
            for (i, k_tmp) in key_list.clone().iter().enumerate() {
                let k = &k_tmp.replace("\"", "");
                debug!("    Current part is {}, part of key {}", k, key);
                if let Some(_) = value_iter.get(k) {
                    // if key exsit, update or delete
                    if i == key_list.len() - 1 {
                        if key_info.operation == "delete" {
                            let value_mapping = value_iter.as_mapping_mut().unwrap();
                            let file_value = value_mapping.get(k).unwrap();
                            info!("Delete configuration {}={}", key, serde_yaml::to_string(file_value).unwrap());
                            value_mapping.remove(k);
                            break;
                        }
                        if !key_info.operation.is_empty() {
                            warn!(
                                "Unknown operation \"{}\", updating key \"{}\" with value \"{}\" by default",
                                key_info.operation,
                                key,
                                serde_json::to_string(&key_info.value).unwrap()
                            );
                        }
                        value_iter = value_iter.get_mut(k).unwrap();
                        let json_value = serde_json::to_string(&key_info.value).unwrap();
                        let config_value: Value = serde_yaml::from_str(&json_value)?;
                        // if value type is array need insert

                        if value_iter.is_sequence() {
                            let value_array = match value_iter.as_sequence_mut() {
                                Some(v) => v,
                                None => {
                                    warn!("Failed to convert yaml Value to sequence, skip this value");
                                    break;
                                },
                            };
                            let config_value_array = match config_value.as_sequence() {
                                Some(v) => v,
                                None => {
                                    warn!("Failed to convert yaml Value to sequence, skip this value");
                                    break;
                                },
                            };
                            value_array.extend_from_slice(config_value_array);
                            info!("Update configuration {}: {}", key, key_info.value.to_string());
                            break;
                        }
                        *value_iter = config_value.into();
                        info!("Update configuration {}: {}", key, key_info.value.to_string());
                        break;
                    }
                    // Has check on the condition of if, unwrap is safe
                    value_iter = value_iter.get_mut(k).unwrap();
                } else {
                    if key_info.operation == "delete" {
                        warn!("Failed to delete inexistent key: \"{}\"", key);
                        continue;
                    }
                    // create if not contains key
                    let json_value = serde_json::to_string(&key_info.value).unwrap();
                    let mut config_value: Value = serde_yaml::from_str(&json_value)?;
                    let mut key_index = key_list.len() - 1;
                    while key_index > i {
                        let mut value_map = serde_yaml::Mapping::new();
                        value_map.insert(Value::String(key_list[key_index].replace("\"", "")).clone(), config_value);
                        config_value = serde_yaml::Value::Mapping(value_map);
                        key_index = key_index - 1;
                    }
                    if value_iter.is_null() {
                        *value_iter = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
                    }
                    let value_mapping = match value_iter.as_mapping_mut() {
                        Some(m) => m,
                        None => {
                            warn!(
                                "Failed to convert yaml value to mapping, maybe read the file in the wrong format,
                                 or write wrong value when handle the configuration of key {}",
                                key
                            );
                            break;
                        },
                    };
                    info!("Add configuration \"{}: {}\"", key, key_info.value.clone());
                    value_mapping.insert(Value::String(k.to_string()).into(), config_value);
                    break;
                }
            }
        }
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(config_path)
            .with_context(|| format!("Failed to open kubelet config file \"{}\"", config_path))?;
        serde_yaml::to_writer(file, &value)
            .with_context(|| format!("Failed to write yaml file \"{}\"", config_path))?;
        return Ok(());
    }
}

impl Configuration for ContainerContainerd {
    fn set_config(&self, config: &mut Sysconfig) -> Result<()> {
        info!("Start setting container.containerd");
        let mut config_path = &values::DEFAULT_CONTAINERD_CONFIG_PATH.to_string();
        if !config.config_path.is_empty() {
            config_path = &config.config_path;
        }
        debug!("container.containerd config_path: \"{}\"", config_path);

        create_config_file(config_path).with_context(|| format!("Failed to find config path \"{}\"", config_path))?;
        let file = std::fs::read_to_string(config_path)
            .with_context(|| format!("Failed to open config file \"{}\"", config_path))?;
        let mut value: Table =
            toml::from_str(&file).with_context(|| format!("Failed to read from config file \"{}\"", config_path))?;
        if value.is_empty() {
            value = toml::map::Map::new();
        }
        let pattern = Regex::new(r#"[^\."']+|"([^"]*)"|'([^']*)'"#)
            .with_context(|| format!("Failed to create regex used by split key"))?;

        for (key, key_info) in config.contents.iter() {
            debug!("Start configuration of key={}", key);
            if key.is_empty() {
                warn!("Failed to add \"null\" key, key: \"{}\"", key);
                continue;
            }
            let key_list: Vec<String> = pattern.find_iter(&key).map(|m| m.as_str().to_string()).collect();
            let mut value_iter = &mut value;
            for (i, k_tmp) in key_list.clone().iter().enumerate() {
                let k = &k_tmp.replace("\"", "");
                debug!("    Current part is {}, part of key {}", k, key);
                if let Some(_) = value_iter.get(k) {
                    debug!("        Key {} is exist", k);
                    if i == key_list.len() - 1 {
                        if key_info.operation == "delete" {
                            let file_value = value_iter.get(k).unwrap();
                            info!("Delete configuration {}={}", key, serde_json::to_string(file_value).unwrap());
                            value_iter.remove(k);
                            break;
                        }
                        if !key_info.operation.is_empty() {
                            warn!(
                                "Unknown operation \"{}\", updating key \"{}\" with value \"{}\" by default",
                                key_info.operation,
                                key,
                                serde_json::to_string(&key_info.value).unwrap()
                            );
                        }
                        let value_last = value_iter.get_mut(k).unwrap();
                        let config_value = match convert_json_to_toml(key_info.value.clone()) {
                            Ok(toml_config) => toml_config,
                            Err(_) => break,
                        };
                        // if value type is array need insert
                        if value_last.is_array() {
                            let value_array = match value_last.as_array_mut() {
                                Some(v) => v,
                                None => {
                                    warn!("Failed to convert toml Value to sequence, skip this value");
                                    break;
                                },
                            };
                            let config_value_array = match config_value.as_array() {
                                Some(v) => v,
                                None => {
                                    warn!("Failed to convert toml Value to sequence, skip this value");
                                    break;
                                },
                            };
                            value_array.extend_from_slice(config_value_array);
                            info!("Update configuration {}: {}", key, key_info.value.to_string());
                            break;
                        }
                        *value_last = config_value.into();
                        info!("Update configuration {}: {}", key, key_info.value.to_string());
                        break;
                    }
                    // Has check value.get() is Some() on the condition of if, value.get(k).unwrap() is safe
                    value_iter = match value_iter.get_mut(k).unwrap().as_table_mut() {
                        Some(value_table) => value_table,
                        None => {
                            warn!("Failed to convert value to table, skip this value");
                            break;
                        },
                    };
                } else {
                    debug!("        Key {} is not exist", k);
                    if key_info.operation == "delete" {
                        warn!("Failed to delete inexistent key: \"{}\"", key);
                        break;
                    }
                    // create if not contains key
                    let mut config_value = match convert_json_to_toml(key_info.value.clone()) {
                        Ok(toml_config) => toml_config,
                        Err(_) => break,
                    };
                    let mut key_index = key_list.len() - 1;
                    while key_index > i {
                        let key_trim = key_list[key_index].replace("\"", "");
                        debug!("Start add key {}", key_trim);
                        let mut value_tmp = toml::Table::from(Map::new());
                        value_tmp.insert(key_trim, config_value.into());
                        config_value = toml::Value::Table(value_tmp);
                        key_index = key_index - 1;
                    }
                    info!("Add configuration \"{}: {}\"", key, key_info.value.clone());
                    value_iter.insert(k.to_string(), config_value);
                    break;
                }
            }
        }
        let toml_string = toml::to_string(&value).with_context(|| format!("Failed to convert value to string"))?;
        std::fs::write(config_path, toml_string).with_context(|| format!("Failed to write file {}", config_path))?;
        Ok(())
    }
}

fn convert_json_to_toml(config: serde_json::Value) -> Result<toml::Value> {
    match config {
        serde_json::Value::Number(c) => {
            if c.is_i64() || c.is_u64() {
                // the type of value is number,value cannot be none, unwrap() is safe
                return Ok(c.as_i64().unwrap().into());
            }
            if c.is_f64() {
                // the type of value is number,value cannot be none, unwrap() is safe
                return Ok(c.as_f64().unwrap().into());
            }
            warn!("Not support number type of value in configuration");
            return Err(anyhow!("Not support number type of value in configuration"));
        },
        serde_json::Value::String(c) => return Ok(c.to_string().into()),
        serde_json::Value::Bool(c) => return Ok(c.into()),
        serde_json::Value::Array(c) => {
            let mut res: Vec<toml::Value> = Vec::new();
            for value in c.iter() {
                let toml_value = match convert_json_to_toml(value.clone()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                res.push(toml_value);
            }
            return Ok(toml::Value::Array(res));
        },
        serde_json::Value::Null => {
            warn!("Failed to convert null value, skip this value");
            return Err(anyhow!("Failed to convert null value"));
        },
        _ => bail!("Not support type of value in configuration, skip this value"),
    }
}

impl Configuration for PamLimits {
    fn set_config(&self, config: &mut Sysconfig) -> Result<()> {
        if !is_file_exist(&self.config_path) {
            bail!("Failed to find file {}", values::DEFAULT_PAM_LIMITS_PATH);
        }
        let configs_write = get_and_set_pam_limits(&self.config_path, &mut config.contents)
            .with_context(|| "Failed to set pam limits configs".to_string())?;
        write_configs_to_file(&self.config_path, &configs_write)
            .with_context(|| "Failed to write configs to file".to_string())?;
        Ok(())
    }
}

fn get_and_set_pam_limits(config_path: &str, configs: &mut HashMap<String, KeyInfo>) -> Result<Vec<String>> {
    let f = File::open(config_path).with_context(|| format!("Failed to open config path \"{}\"", config_path))?;
    let mut configs_write = Vec::new();
    for line in io::BufReader::new(f).lines() {
        let line = line?;
        // if line is a comment or blank
        if line.starts_with('#') || line.trim().is_empty() {
            configs_write.push(line);
            continue;
        }
        let config_kv: Vec<&str> = line.splitn(4, ' ').map(|s| s.trim()).collect();
        // if config_kv is not a key-value pair
        if config_kv.len() != 4 {
            bail!("could not parse pam limits config {}", line);
        }
        let new_key_info = configs.get(config_kv[0]);
        let new_config = match new_key_info {
            Some(new_key_info) if new_key_info.operation == "delete" => handle_delete_key(&config_kv, new_key_info),
            Some(new_key_info) => handle_update_key(&config_kv, new_key_info),
            None => config_kv.join(" "),
        };
        if !new_config.is_empty() {
            configs_write.push(new_config);
        }
        configs.remove(config_kv[0]);
    }
    let new_config = handle_add_key_pam_limits(&configs);
    configs_write.extend(new_config);
    Ok(configs_write)
}

fn handle_add_key_pam_limits(new_configs: &HashMap<String, KeyInfo>) -> Vec<String> {
    let mut configs_write = Vec::new();
    'configs: for (key, config_info) in new_configs.iter() {
        if config_info.operation == "delete" {
            warn!("Failed to delete inexistent key: \"{}\"", key);
            continue;
        }
        if key.is_empty() || key.contains(' ') {
            warn!("Failed to add \"null\" key or key containing \" \", key: \"{}\"", key);
            continue;
        }
        if !config_info.operation.is_empty() {
            warn!(
                "Unknown operation \"{}\", adding key \"{}\" with value \"{}\" by default",
                config_info.operation, key, config_info.value
            );
        }
        let (config_info_value, is_recognized) = convert_json_value_to_string(&config_info.value);
        if !is_recognized {
            warn!("Failed to handle keyinfo.value, the type of it is not in range of number, string, boolean, null");
            continue;
        }
        let (k, v) = (key.trim(), config_info_value.trim());
        if v.is_empty() {
            warn!("Failed to add key \"{}\" with \"null\" value", k);
        }
        let new_value_list: Vec<&str> = config_info_value.split(".").collect();
        if new_value_list.len() != 3 {
            warn!(
                "Failed to update pam limits key \"{}\" with value {} because of illegal format of value",
                key, config_info_value
            );
            continue;
        }
        for v in new_value_list.iter() {
            if v.trim() == "_" {
                warn!("Failed to add key \"{}\" with \"_\" value, skip this configuration", k);
                continue 'configs;
            }
        }
        info!("Add configuration \"{} {}\"", key, new_value_list.join(" "));
        configs_write.push(format!("{} {}", key, new_value_list.join(" ")));
    }
    configs_write
}

#[cfg(test)]
mod tests {
    use std::fs;

    use mockall::{mock, predicate::*};
    use serde_json::json;
    use tempfile::{NamedTempFile, TempDir};
    use values::{CONTAINER_CONTAINERD, KUBERNETES_KUBELET, PAM_LIMTS};

    use super::*;
    use crate::sys_mgmt::{GRUB_CMDLINE_CURRENT, GRUB_CMDLINE_NEXT, KERNEL_SYSCTL, KERNEL_SYSCTL_PERSIST};

    // Mock the CommandExecutor trait
    mock! {
        pub CommandExec{}
        impl CommandExecutor for CommandExec {
            fn run_command<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<()>;
            fn run_command_with_output<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<String>;
        }
        impl Clone for CommandExec {
            fn clone(&self) -> Self;
        }
    }

    fn init() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_get_config_partition() {
        init();
        let mut grub_cmdline = GrubCmdline { grub_path: String::from(""), is_cur_partition: true };
        let mut executor = MockCommandExec::new();

        // the output shows that current root menuentry is A
        let command_output1 = r#"vda                   23622320128
vda1 /boot/efi vfat      61865984 BOOT
vda2 /         ext4    3145728000 ROOT-A
vda3           ext4    2621440000 ROOT-B
vda4 /persist  ext4   17791188992 PERSIST
"#;
        executor.expect_run_command_with_output().times(1).returning(|_, _| Ok(command_output1.to_string()));

        let result = grub_cmdline.get_config_partition(executor).unwrap();
        // it should return false because the current root menuentry is A and we want to configure current partition
        assert_eq!(result, false);

        let mut executor = MockCommandExec::new();

        // the output shows that current root menuentry is A
        let command_output1 = r#"vda                   23622320128
vda1 /boot/efi vfat      61865984 BOOT
vda2 /         ext4    3145728000 ROOT-A
vda3           ext4    2621440000 ROOT-B
vda4 /persist  ext4   17791188992 PERSIST
"#;
        executor.expect_run_command_with_output().times(1).returning(|_, _| Ok(command_output1.to_string()));
        grub_cmdline.is_cur_partition = false;
        let result = grub_cmdline.get_config_partition(executor).unwrap();
        // it should return true because the current root menuentry is A and we want to configure next partition
        assert_eq!(result, true);
    }

    #[test]
    fn test_kernel_sysctl() {
        init();
        let tmp_dir = TempDir::new().unwrap();
        assert_eq!(tmp_dir.path().exists(), true);
        let kernel_sysctl = KernelSysctl::new(tmp_dir.path().to_str().unwrap());

        let config_detail = HashMap::from([
            ("a".to_string(), KeyInfo { value: serde_json::Value::from(json!(1)), operation: "".to_string() }),
            ("b".to_string(), KeyInfo { value: serde_json::Value::from(json!(2)), operation: "delete".to_string() }),
            ("c".to_string(), KeyInfo { value: serde_json::Value::from(json!(3)), operation: "add".to_string() }),
            ("d".to_string(), KeyInfo { value: serde_json::Value::from(json!("")), operation: "".to_string() }),
            ("e".to_string(), KeyInfo { value: serde_json::Value::from(json!("")), operation: "delete".to_string() }),
        ]);

        let mut config =
            Sysconfig { model: KERNEL_SYSCTL.to_string(), config_path: String::from(""), contents: config_detail };
        kernel_sysctl.set_config(&mut config).unwrap();

        let result = fs::read_to_string(format!("{}{}", tmp_dir.path().to_str().unwrap(), "a")).unwrap();
        assert_eq!(result, "1\n");
    }

    #[test]
    fn test_kernel_sysctl_persist() {
        init();
        let comment = r"# This file is managed by KubeOS for unit testing.";
        // create a tmp file with comment
        let mut tmp_file = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp_file, "{}", comment).unwrap();
        writeln!(tmp_file, "a=0").unwrap();
        writeln!(tmp_file, "d=4").unwrap();
        writeln!(tmp_file, "e=5").unwrap();
        writeln!(tmp_file, "g=7").unwrap();
        let kernel_sysctl_persist = KernelSysctlPersist {};
        let config_detail = HashMap::from([
            ("a".to_string(), KeyInfo { value: serde_json::Value::from(json!(1)), operation: "".to_string() }),
            ("b".to_string(), KeyInfo { value: serde_json::Value::from(json!(2)), operation: "delete".to_string() }),
            ("c".to_string(), KeyInfo { value: serde_json::Value::from(json!(3)), operation: "add".to_string() }),
            ("d".to_string(), KeyInfo { value: serde_json::Value::from(json!("")), operation: "".to_string() }),
            ("e".to_string(), KeyInfo { value: serde_json::Value::from(json!("")), operation: "delete".to_string() }),
            ("f".to_string(), KeyInfo { value: serde_json::Value::from(json!("")), operation: "add".to_string() }),
            ("g".to_string(), KeyInfo { value: serde_json::Value::from(json!(7)), operation: "delete".to_string() }),
            ("".to_string(), KeyInfo { value: serde_json::Value::from(json!(8)), operation: "".to_string() }),
            ("s=x".to_string(), KeyInfo { value: serde_json::Value::from(json!(8)), operation: "".to_string() }),
        ]);
        let mut config = Sysconfig {
            model: KERNEL_SYSCTL_PERSIST.to_string(),
            config_path: String::from(tmp_file.path().to_str().unwrap()),
            contents: config_detail,
        };
        kernel_sysctl_persist.set_config(&mut config).unwrap();
        let result = fs::read_to_string(tmp_file.path().to_str().unwrap()).unwrap();
        let expected_res = format!("{}\n{}\n{}\n{}\n{}\n", comment, "a=1", "d=4", "e=5", "c=3");
        assert_eq!(result, expected_res);
        let mut config = Sysconfig {
            model: KERNEL_SYSCTL_PERSIST.to_string(),
            config_path: String::from("/tmp/kubeos-test-kernel-sysctl-persist.txt"),
            contents: HashMap::new(),
        };
        kernel_sysctl_persist.set_config(&mut config).unwrap();
        assert!(is_file_exist(&config.config_path));
        delete_file_or_dir(&config.config_path).unwrap();
    }

    #[test]
    fn write_configs_to_file_tests() {
        init();
        let tmp_file = NamedTempFile::new().unwrap();
        let configs = vec!["a=1".to_string(), "b=2".to_string()];
        write_configs_to_file(tmp_file.path().to_str().unwrap(), &configs).unwrap();
        assert_eq!(fs::read(tmp_file.path()).unwrap(), b"a=1\nb=2\n");
    }

    #[test]
    fn test_grub_cmdline() {
        init();
        let mut tmp_file = NamedTempFile::new().unwrap();
        let mut grub_cmdline =
            GrubCmdline { grub_path: tmp_file.path().to_str().unwrap().to_string(), is_cur_partition: true };
        let grub_cfg = r"menuentry 'A' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-A' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        set root='hd0,gpt2'
        linux   /boot/vmlinuz root=UUID=1 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3
        initrd  /boot/initramfs.img
}

menuentry 'B' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-B' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        set root='hd0,gpt3'
        linux   /boot/vmlinuz root=UUID=2 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3
        initrd  /boot/initramfs.img
}";
        writeln!(tmp_file, "{}", grub_cfg).unwrap();
        let config_second_part = HashMap::from([
            (
                "debug".to_string(),
                KeyInfo { value: serde_json::Value::String("".to_string()), operation: "".to_string() },
            ),
            (
                "quiet".to_string(),
                KeyInfo { value: serde_json::Value::String("".to_string()), operation: "delete".to_string() },
            ),
            (
                "panic".to_string(),
                KeyInfo { value: serde_json::Value::String("5".to_string()), operation: "".to_string() },
            ),
            (
                "nomodeset".to_string(),
                KeyInfo { value: serde_json::Value::String("".to_string()), operation: "update".to_string() },
            ),
            (
                "oops".to_string(),
                KeyInfo { value: serde_json::Value::String("".to_string()), operation: "".to_string() },
            ),
            (
                "".to_string(),
                KeyInfo { value: serde_json::Value::String("test".to_string()), operation: "".to_string() },
            ),
            (
                "selinux".to_string(),
                KeyInfo { value: serde_json::Value::String("1".to_string()), operation: "delete".to_string() },
            ),
            (
                "acpi".to_string(),
                KeyInfo { value: serde_json::Value::String("off".to_string()), operation: "delete".to_string() },
            ),
            (
                "ro".to_string(),
                KeyInfo { value: serde_json::Value::String("1".to_string()), operation: "".to_string() },
            ),
        ]);
        let mut config = Sysconfig {
            model: GRUB_CMDLINE_CURRENT.to_string(),
            config_path: String::new(),
            contents: config_second_part,
        };
        grub_cmdline.set_config(&mut config).unwrap();
        grub_cmdline.is_cur_partition = false;
        let config_first_part = HashMap::from([
            (
                "pci".to_string(),
                KeyInfo { value: serde_json::Value::String("nomis".to_string()), operation: "".to_string() },
            ),
            (
                "quiet".to_string(),
                KeyInfo { value: serde_json::Value::String("11".to_string()), operation: "delete".to_string() },
            ),
            (
                "panic".to_string(),
                KeyInfo { value: serde_json::Value::String("5".to_string()), operation: "update".to_string() },
            ),
        ]);
        config.contents = config_first_part;
        config.model = GRUB_CMDLINE_NEXT.to_string();
        grub_cmdline.set_config(&mut config).unwrap();
        let result = fs::read_to_string(tmp_file.path().to_str().unwrap()).unwrap();
        let expected_res = r"menuentry 'A' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-A' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        set root='hd0,gpt2'
        linux /boot/vmlinuz root=UUID=1 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=5 pci=nomis
        initrd  /boot/initramfs.img
}
menuentry 'B' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-B' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        set root='hd0,gpt3'
        linux /boot/vmlinuz root=UUID=2 ro=1 rootfstype=ext4 nomodeset oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=5 debug
        initrd  /boot/initramfs.img
}
";
        assert_eq!(result, expected_res);

        // test grub.cfg not exist
        grub_cmdline.grub_path = "/tmp/grub-KubeOS-test.cfg".to_string();
        let res = grub_cmdline.set_config(&mut config);
        assert!(res.is_err());
    }

    #[test]
    fn test_create_config_file() {
        init();
        let tmp_file = "/tmp/kubeos-test-create-config-file.txt";
        create_config_file(&tmp_file).unwrap();
        assert!(is_file_exist(&tmp_file));
        fs::remove_file(tmp_file).unwrap();
    }

    #[test]
    fn test_kubernetes_kubelet() {
        init();
        let test_file = "test.yaml";
        let config_kubelet_add = HashMap::from([
            (
                "apiVersion".to_string(),
                KeyInfo {
                    value: serde_json::Value::from(json!("kubelet.config.k8s.io/v1beta1")),
                    operation: "".to_string(),
                },
            ),
            (
                "kind".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("KubeletConfiguration")), operation: "".to_string() },
            ),
            (
                "address".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("192.168.0.8")), operation: "".to_string() },
            ),
            ("port".to_string(), KeyInfo { value: serde_json::Value::from(json!(20250)), operation: "".to_string() }),
            (
                "evictionHard.\"memory.available\"".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("100Mi")), operation: "".to_string() },
            ),
            (
                "evictionHard.\"nodefs.available\"".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("10%")), operation: "".to_string() },
            ),
            (
                "authorization.mode".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("Webhook")), operation: "".to_string() },
            ),
            (
                "logging.options.json.infoBufferSize".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("0")), operation: "".to_string() },
            ),
            (
                "clusterDNS".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(["10.96.0.10"])), operation: "".to_string() },
            ),
        ]);
        let mut sysconfig_add = Sysconfig {
            model: KUBERNETES_KUBELET.to_string(),
            config_path: test_file.to_string(),
            contents: config_kubelet_add.clone(),
        };
        let k8s_kubelet = KubernetesKubelet {};
        let res_add = k8s_kubelet.set_config(&mut sysconfig_add);
        assert!(!res_add.is_err());

        // check value
        let file = std::fs::File::open(test_file).expect("create yaml file failed");
        let mut value: serde_yaml::Value = serde_yaml::from_reader(file).unwrap();
        let pattern = Regex::new(r#"[^\."']+|"([^"]*)"|'([^']*)'"#)
            .with_context(|| format!("Failed to create regex used by split key"))
            .unwrap();

        for (key, key_info) in config_kubelet_add {
            let mut value_iter = &mut value;
            let key_list: Vec<String> = pattern.find_iter(&key).map(|m| m.as_str().to_string()).collect();
            for (i, k_tmp) in key_list.clone().iter().enumerate() {
                let k = &k_tmp.replace("\"", "");
                if i == key_list.len() - 1 {
                    let config_value: Value =
                        serde_yaml::from_str(&serde_json::to_string(&key_info.value).unwrap()).unwrap();
                    let file_value = value_iter.get(k).unwrap();
                    assert!(config_value.eq(file_value));
                    break;
                }
                value_iter = value_iter.get_mut(k).unwrap();
            }
        }

        let config_kubelet = HashMap::from([
            //normal update，value type is string, boolean, null and list
            (
                "evictionHard.\"memory.available\"".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("500Mi".to_string())), operation: "".to_string() },
            ),
            (
                "serializeImagePulls".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(true)), operation: "".to_string() },
            ),
            ("port".to_string(), KeyInfo { value: serde_json::Value::from(json!(20000)), operation: "".to_string() }),
            (
                "logging.options.json".to_string(),
                KeyInfo { value: serde_json::Value::default(), operation: "".to_string() },
            ),
            (
                "clusterDNS".to_string(),
                KeyInfo {
                    value: serde_json::Value::from(json!(["10.96.0.11", "10.96.0.12"])),
                    operation: "".to_string(),
                },
            ),
            // normal delete
            (
                "evictionHard.\"nodefs.inodesFree\"".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("")), operation: "delete".to_string() },
            ),
            (
                "authoriazation".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("")), operation: "delete".to_string() },
            ),
            // normal add
            (
                "evictionHard.\"test.test\"".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("test")), operation: "".to_string() },
            ),
            ("testA".to_string(), KeyInfo { value: serde_json::Value::from(json!(true)), operation: "".to_string() }),
            (
                "logging.options.testB.testC.testD.testE".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(2.34)), operation: "".to_string() },
            ),
            //abnormal

            // delete but key is not exisst
            (
                "evictionHard.key.not.exist".to_string(),
                KeyInfo { value: serde_json::Value::from("".to_string()), operation: "delete".to_string() },
            ),
            // key is empty
            (
                "".to_string(),
                KeyInfo { value: serde_json::Value::String("".to_string()), operation: "delete".to_string() },
            ),
        ]);
        let mut config = Sysconfig {
            model: KUBERNETES_KUBELET.to_string(),
            config_path: test_file.to_string(),
            contents: config_kubelet,
        };
        let k8s_kubelet = KubernetesKubelet {};
        let res = k8s_kubelet.set_config(&mut config);
        assert!(!res.is_err());

        let del_res = std::fs::remove_file(test_file);
        assert!(!del_res.is_err());
    }

    #[test]
    fn test_container_containerd() {
        init();
        let test_file = "test.toml";
        let config_contained_add = HashMap::from([
            (
                "disabled_plugins".to_string(),
                KeyInfo { value: serde_json::Value::from(json!([1, 2, 3])), operation: "".to_string() },
            ),
            (
                "grpc.address".to_string(),
                KeyInfo {
                    value: serde_json::Value::from(json!("/run/containerd/containerd.sock")),
                    operation: "".to_string(),
                },
            ),
            ("grpc.uid".to_string(), KeyInfo { value: serde_json::Value::from(json!(0)), operation: "".to_string() }),
            (
                "grpc.client_tls_auto".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(true)), operation: "".to_string() },
            ),
            (
                "plugins.\"io.containerd.grpc.v1.cri\".containerd.default_runtime_name".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("runc")), operation: "".to_string() },
            ),
            (
                "plugins.\"io.containerd.grpc.v1.cri\".image.pause_image".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("k8s.gcr.io/pause:3.2")), operation: "".to_string() },
            ),
            (
                "plugins.\"io.containerd.gc.v1.scheduler\".deletion_threshold".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(0)), operation: "".to_string() },
            ),
            (
                "timeouts.\"io.containerd.timeout.bolt.open\"".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("0s")), operation: "".to_string() },
            ),
        ]);
        let mut sysconfig_add = Sysconfig {
            model: CONTAINER_CONTAINERD.to_string(),
            config_path: test_file.to_string(),
            contents: config_contained_add.clone(),
        };
        let con_containerd = ContainerContainerd {};
        let res_add = con_containerd.set_config(&mut sysconfig_add);
        assert!(!res_add.is_err());

        let config_contained = HashMap::from([
            // normal update, value type is number, bool, list
            (
                "disabled_plugins".to_string(),
                KeyInfo { value: serde_json::Value::from(json!([4, 5, 6])), operation: "".to_string() },
            ),
            (
                "grpc.uid".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(1)), operation: "update".to_string() },
            ),
            (
                "grpc.client_tls_auto".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(false)), operation: "".to_string() },
            ),
            (
                "plugins.\"io.containerd.grpc.v1.cri\".image.pause_image".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("k8s.gcr.io/pause:3.2")), operation: "".to_string() },
            ),
            (
                "plugins.\"io.containerd.grpc.v1.cri\".containerd.default_runtime_name".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("runc")), operation: "delete".to_string() },
            ),
            // normal add
            (
                "plugins.\"io.containerd.snapshotter.v1.native\".root_path".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(0)), operation: "".to_string() },
            ),
            (
                "timeouts.\"io.containerd.timeout.shim.cleanup\"".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("0s")), operation: "".to_string() },
            ),
            // abnormal
            // key is empty
            ("".to_string(), KeyInfo { value: serde_json::Value::from(json!("0s")), operation: "".to_string() }),
            // delete key which does not exist
            (
                "timeouts.\"io.containerd.timeout.shim.test\"".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("0s")), operation: "delete".to_string() },
            ),
            (
                "timeouts.\"io.containerd.timeout.shim.test.test\"".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(null)), operation: "".to_string() },
            ),
        ]);
        let mut sysconfig = Sysconfig {
            model: CONTAINER_CONTAINERD.to_string(),
            config_path: test_file.to_string(),
            contents: config_contained.clone(),
        };
        let res_add = con_containerd.set_config(&mut sysconfig);
        assert!(!res_add.is_err());

        let del_res = std::fs::remove_file(test_file);
        assert!(!del_res.is_err());
    }
    #[test]
    fn pam_limits() {
        init();
        let comment = r"# This file is managed by KubeOS for unit testing.";
        let mut tmp_file = NamedTempFile::new().unwrap();
        writeln!(tmp_file, "{}", comment).unwrap();
        writeln!(tmp_file, "a 1 2 3").unwrap();
        writeln!(tmp_file, "b 4 5 6").unwrap();
        writeln!(tmp_file, "d 1 2 3").unwrap();
        writeln!(tmp_file, "e 4 5 6").unwrap();
        writeln!(tmp_file, "f 7 8 9").unwrap();
        writeln!(tmp_file, "g 7 8 9").unwrap();
        let config_pam_limits = HashMap::from([
            //normal add
            ("c".to_string(), KeyInfo { value: serde_json::Value::from(json!("7.8.9")), operation: "".to_string() }),
            // normal update
            ("a".to_string(), KeyInfo { value: serde_json::Value::from(json!("4.5.6")), operation: "".to_string() }),
            ("b".to_string(), KeyInfo { value: serde_json::Value::from(json!("1.2._")), operation: "add".to_string() }),
            // normal delete
            (
                "e".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("4.5.6")), operation: "delete".to_string() },
            ),
            // abnormal
            // key is ""
            ("".to_string(), KeyInfo { value: serde_json::Value::from(json!(20250)), operation: "".to_string() }),
            // key has whitespace
            (
                "a b".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("Webhook")), operation: "".to_string() },
            ),
            // delete but key not exist
            (
                "q".to_string(),
                KeyInfo { value: serde_json::Value::from(json!(20250)), operation: "delete".to_string() },
            ),
            // delete but value not equal
            (
                "d".to_string(),
                KeyInfo { value: serde_json::Value::from(json!("1.2.3")), operation: "delete".to_string() },
            ),
            // update but value is ""
            ("f".to_string(), KeyInfo { value: serde_json::Value::from(json!("")), operation: "".to_string() }),
            // update but value is illegal formats
            ("g".to_string(), KeyInfo { value: serde_json::Value::from(json!("1.2")), operation: "".to_string() }),
            // add value is ""
            ("r".to_string(), KeyInfo { value: serde_json::Value::from(json!("1.2")), operation: "".to_string() }),
            // add value is illegal formats
            ("w".to_string(), KeyInfo { value: serde_json::Value::from(json!("1.2")), operation: "".to_string() }),
            // add but value contains "_"
            ("d".to_string(), KeyInfo { value: serde_json::Value::from(json!("1._.3")), operation: "".to_string() }),
        ]);
        let pam_limits = PamLimits { config_path: tmp_file.path().to_str().unwrap().to_string() };
        let mut config = Sysconfig {
            model: PAM_LIMTS.to_string(),
            config_path: String::from(tmp_file.path().to_str().unwrap()),
            contents: config_pam_limits,
        };
        pam_limits.set_config(&mut config).unwrap();
        let result = fs::read_to_string(tmp_file.path().to_str().unwrap()).unwrap();
        let expected_res = format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n",
            comment, "a 4 5 6", "b 1 2 6", "d 1 2 3", "f 7 8 9", "g 7 8 9", "c 7 8 9"
        );
        assert_eq!(result, expected_res);
        assert!(is_file_exist(&config.config_path));
        delete_file_or_dir(&config.config_path).unwrap();
    }
}
