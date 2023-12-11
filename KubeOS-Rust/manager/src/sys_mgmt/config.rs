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

use anyhow::{anyhow, Context, Ok, Result};
use lazy_static::lazy_static;
use log::{debug, info, trace, warn};
use regex::Regex;

use super::{api::*, values};
use crate::utils::*;

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
            Box::new(GrubCmdline {
                grub_path: values::DEFAULT_GRUB_CFG_PATH.to_string(),
                is_cur_partition: true,
            }) as Box<dyn Configuration + Sync>,
        );
        config_map.insert(
            values::GRUB_CMDLINE_NEXT.to_string(),
            Box::new(GrubCmdline {
                grub_path: values::DEFAULT_GRUB_CFG_PATH.to_string(),
                is_cur_partition: false,
            }) as Box<dyn Configuration + Sync>,
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

impl Configuration for KernelSysctl {
    fn set_config(&self, config: &mut Sysconfig) -> Result<()> {
        info!("Start set kernel.sysctl");
        for (key, key_info) in config.contents.iter() {
            let proc_path = self.get_proc_path(key);
            if key_info.operation == "delete" {
                warn!("Failed to delete kernel.sysctl config with key \"{}\"", key);
            } else if key_info.value != "" && key_info.operation == "" {
                fs::write(&proc_path, format!("{}\n", &key_info.value).as_bytes()).with_context(
                    || format!("Failed to write kernel.sysctl with key: \"{}\"", key),
                )?;
                info!("Configured kernel.sysctl {}={}", key, key_info.value);
            } else {
                warn!(
                    "Failed to parse kernel.sysctl, key: \"{}\", value: \"{}\", operation: \"{}\"",
                    key, key_info.value, key_info.operation
                );
            }
        }
        Ok(())
    }
}

impl KernelSysctl {
    fn new(proc_path: &str) -> Self {
        Self {
            proc_path: String::from(proc_path),
        }
    }

    fn get_proc_path(&self, key: &str) -> PathBuf {
        let path_str = format!("{}{}", self.proc_path, key.replace(".", "/"));
        Path::new(&path_str).to_path_buf()
    }
}

impl Configuration for KernelSysctlPersist {
    fn set_config(&self, config: &mut Sysconfig) -> Result<()> {
        info!("Start set kernel.sysctl.persist");
        let mut config_path = &values::DEFAULT_KERNEL_CONFIG_PATH.to_string();
        if config.config_path != "" {
            config_path = &config.config_path;
        }
        debug!("kernel.sysctl.persist config_path: \"{}\"", config_path);
        create_config_file(config_path)?;
        let configs = get_and_set_configs(&mut config.contents, config_path)?;
        write_configs_to_file(config_path, &configs)?;
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

fn get_and_set_configs(
    expect_configs: &mut HashMap<String, KeyInfo>,
    config_path: &str,
) -> Result<Vec<String>> {
    let f = File::open(config_path)?;
    let mut configs_write = Vec::new();
    for line in io::BufReader::new(f).lines() {
        let line = line?;
        // if line is a comment or blank
        if line.starts_with("#") || line.starts_with(";") || line.trim().is_empty() {
            configs_write.push(line);
            continue;
        }
        let config_kv: Vec<&str> = line.splitn(2, '=').map(|s| s.trim()).collect();
        // if config_kv is not a key-value pair
        if config_kv.len() != 2 {
            return Err(anyhow!("could not parse sysctl config {}", line));
        }
        let new_key_info = expect_configs.get(config_kv[0]);
        let new_config = match new_key_info {
            Some(new_key_info) if new_key_info.operation == "delete" => {
                handle_delete_key(&config_kv, new_key_info)
            }
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
        if line == "" {
            continue;
        }
        writeln!(w, "{}", line.as_str())?;
    }
    w.flush()
        .with_context(|| format!("Failed to flush file {}", config_path))?;
    w.get_mut()
        .sync_all()
        .with_context(|| format!("Failed to sync"))?;
    debug!("Write configuration to file \"{}\" success", config_path);
    Ok(())
}

fn handle_delete_key(config_kv: &Vec<&str>, new_config_info: &KeyInfo) -> String {
    let key = config_kv[0];
    if config_kv.len() == 1 && new_config_info.value == "" {
        info!("Delete configuration key: \"{}\"", key);
        return String::from("");
    } else if config_kv.len() == 1 && new_config_info.value != "" {
        warn!(
            "Failed to delete key \"{}\" with inconsistent values \"nil\" and \"{}\"",
            key, new_config_info.value
        );
        return key.to_string();
    }
    let old_value = config_kv[1];
    if old_value != new_config_info.value {
        warn!(
            "Failed to delete key \"{}\" with inconsistent values \"{}\" and \"{}\"",
            key, old_value, new_config_info.value
        );
        return config_kv.join("=");
    }
    info!("Delete configuration {}={}", key, old_value);
    String::from("")
}

fn handle_update_key(config_kv: &Vec<&str>, new_config_info: &KeyInfo) -> String {
    let key = config_kv[0];
    if new_config_info.operation != "" {
        warn!(
            "Unknown operation \"{}\", updating key \"{}\" with value \"{}\" by default",
            new_config_info.operation, key, new_config_info.value
        );
    }
    if config_kv.len() == values::ONLY_KEY && new_config_info.value == "" {
        return key.to_string();
    }
    let new_value = new_config_info.value.trim();
    if config_kv.len() == values::ONLY_KEY && new_config_info.value != "" {
        info!("Update configuration \"{}={}\"", key, new_value);
        return format!("{}={}", key, new_value);
    }
    if new_config_info.value == "" {
        warn!("Failed to update key \"{}\" with \"null\" value", key);
        return config_kv.join("=");
    }
    info!("Update configuration \"{}={}\"", key, new_value);
    format!("{}={}", key, new_value)
}

fn handle_add_key(
    expect_configs: &HashMap<String, KeyInfo>,
    is_only_key_valid: bool,
) -> Vec<String> {
    let mut configs_write = Vec::new();
    for (key, config_info) in expect_configs.iter() {
        if config_info.operation == "delete" {
            warn!("Failed to delete inexistent key: \"{}\"", key);
            continue;
        }
        if key == "" || key.contains("=") {
            warn!(
                "Failed to add \"null\" key or key containing \"=\", key: \"{}\"",
                key
            );
            continue;
        }
        if config_info.operation != "" {
            warn!(
                "Unknown operation \"{}\", adding key \"{}\" with value \"{}\" by default",
                config_info.operation, key, config_info.value
            );
        }
        let (k, v) = (key.trim(), config_info.value.trim());
        if v == "" && is_only_key_valid {
            info!("Add configuration \"{}\"", k);
            configs_write.push(k.to_string());
        } else if v == "" {
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
        if self.is_cur_partition {
            info!("Start set grub.cmdline.current configuration");
        } else {
            info!("Start set grub.cmdline.next configuration");
        }
        if !is_file_exist(&self.grub_path) {
            return Err(anyhow!("Failed to find grub.cfg file"));
        }
        let config_partition = if cfg!(test) {
            self.is_cur_partition
        } else {
            self.get_config_partition(RealCommandExecutor {})?
        };
        debug!(
            "Config_partition: {} (false means partition A, true means partition B)",
            config_partition
        );
        let configs = get_and_set_grubcfg(&mut config.contents, &self.grub_path, config_partition)?;
        write_configs_to_file(&self.grub_path, &configs)?;
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
    let f = File::open(grub_path)?;
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
    trace!(
        "Match partition that need to be configured, entering modify_boot_cfg, linux line: {}",
        line
    );
    let mut new_configs = vec!["       ".to_string()];
    let olg_configs: Vec<&str> = line.split(' ').collect();
    for old_config in olg_configs {
        if old_config == "" {
            continue;
        }
        // At most 2 substrings can be returned to satisfy the case like root=UUID=xxxx
        let config = old_config.splitn(2, "=").collect::<Vec<&str>>();
        if config.len() != values::ONLY_KEY && config.len() != values::KV_PAIR {
            return Err(anyhow!(
                "Failed to parse grub.cfg linux line {}",
                old_config
            ));
        }
        let new_key_info = expect_configs.get(config[0]);
        let new_config = match new_key_info {
            Some(new_key_info) if new_key_info.operation == "delete" => {
                handle_delete_key(&config, new_key_info)
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys_mgmt::{
        GRUB_CMDLINE_CURRENT, GRUB_CMDLINE_NEXT, KERNEL_SYSCTL, KERNEL_SYSCTL_PERSIST,
    };
    use mockall::{mock, predicate::*};
    use std::fs;
    use tempfile::{NamedTempFile, TempDir};

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
        let mut grub_cmdline = GrubCmdline {
            grub_path: String::from(""),
            is_cur_partition: true,
        };
        let mut executor = MockCommandExec::new();

        // the output shows that current root menuentry is A
        let command_output1 =
            "sda\nsda1 /boot/efi vfat\nsda2 / ext4\nsda3  ext4\nsda4 /persist ext4\nsr0  iso9660\n";
        executor
            .expect_run_command_with_output()
            .times(1)
            .returning(|_, _| Ok(command_output1.to_string()));

        let result = grub_cmdline.get_config_partition(executor).unwrap();
        // it should return false because the current root menuentry is A and we want to configure current partition
        assert_eq!(result, false);

        let mut executor = MockCommandExec::new();

        // the output shows that current root menuentry is A
        let command_output1 =
            "sda\nsda1 /boot/efi vfat\nsda2 / ext4\nsda3  ext4\nsda4 /persist ext4\nsr0  iso9660\n";
        executor
            .expect_run_command_with_output()
            .times(1)
            .returning(|_, _| Ok(command_output1.to_string()));
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
            (
                "a".to_string(),
                KeyInfo {
                    value: "1".to_string(),
                    operation: "".to_string(),
                },
            ),
            (
                "b".to_string(),
                KeyInfo {
                    value: "2".to_string(),
                    operation: "delete".to_string(),
                },
            ),
            (
                "c".to_string(),
                KeyInfo {
                    value: "3".to_string(),
                    operation: "add".to_string(),
                },
            ),
            (
                "d".to_string(),
                KeyInfo {
                    value: "".to_string(),
                    operation: "".to_string(),
                },
            ),
            (
                "e".to_string(),
                KeyInfo {
                    value: "".to_string(),
                    operation: "delete".to_string(),
                },
            ),
        ]);

        let mut config = Sysconfig {
            model: KERNEL_SYSCTL.to_string(),
            config_path: String::from(""),
            contents: config_detail,
        };
        kernel_sysctl.set_config(&mut config).unwrap();

        let result =
            fs::read_to_string(format!("{}{}", tmp_dir.path().to_str().unwrap(), "a")).unwrap();
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
        let kernel_sysctl_persist = KernelSysctlPersist {};
        let config_detail = HashMap::from([
            (
                "a".to_string(),
                KeyInfo {
                    value: "1".to_string(),
                    operation: "".to_string(),
                },
            ),
            (
                "b".to_string(),
                KeyInfo {
                    value: "2".to_string(),
                    operation: "delete".to_string(),
                },
            ),
            (
                "c".to_string(),
                KeyInfo {
                    value: "3".to_string(),
                    operation: "add".to_string(),
                },
            ),
        ]);
        let mut config = Sysconfig {
            model: KERNEL_SYSCTL_PERSIST.to_string(),
            config_path: String::from(tmp_file.path().to_str().unwrap()),
            contents: config_detail,
        };
        kernel_sysctl_persist.set_config(&mut config).unwrap();
        let result = fs::read_to_string(tmp_file.path().to_str().unwrap()).unwrap();
        let expected_res = format!("{}\n{}\n{}\n", comment, "a=1", "c=3");
        assert_eq!(result, expected_res);

        // test config_path is empty
        // remember modify DEFAULT_KERNEL_CONFIG_PATH first
        // let config_detail = HashMap::from([
        //     (
        //         "aaa".to_string(),
        //         KeyInfo {
        //             value: "3".to_string(),
        //             operation: "add".to_string(),
        //         },
        //     ),
        //     (
        //         "bbb".to_string(),
        //         KeyInfo {
        //             value: "1".to_string(),
        //             operation: "delete".to_string(),
        //         },
        //     ),
        // ]);
        // config.config_path = "".to_string();
        // config.contents = config_detail;
        // kernel_sysctl_persist.set_config(&mut config).unwrap();
        // let result = fs::read_to_string(crate::sys_mgmt::DEFAULT_KERNEL_CONFIG_PATH).unwrap();
        // let expected_res = format!("{}\n", "aaa=3",);
        // assert_eq!(result, expected_res);
    }

    #[test]
    fn write_configs_to_file_tests() {
        init();
        let path = "/home/yuhang/abc.txt";
        let configs = vec!["a=1".to_string(), "b=2".to_string()];
        write_configs_to_file(&path.to_string(), &configs).unwrap();
    }

    #[test]
    fn test_grub_cmdline() {
        init();
        let mut tmp_file = NamedTempFile::new().unwrap();
        let mut grub_cmdline = GrubCmdline {
            grub_path: tmp_file.path().to_str().unwrap().to_string(),
            is_cur_partition: true,
        };
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
        let config_first_part = HashMap::from([
            (
                "debug".to_string(),
                KeyInfo {
                    value: "".to_string(),
                    operation: "".to_string(),
                },
            ),
            (
                "quiet".to_string(),
                KeyInfo {
                    value: "".to_string(),
                    operation: "delete".to_string(),
                },
            ),
            (
                "panic".to_string(),
                KeyInfo {
                    value: "5".to_string(),
                    operation: "".to_string(),
                },
            ),
            (
                "nomodeset".to_string(),
                KeyInfo {
                    value: "".to_string(),
                    operation: "update".to_string(),
                },
            ),
            (
                "oops".to_string(),
                KeyInfo {
                    value: "".to_string(),
                    operation: "".to_string(),
                },
            ),
            (
                "".to_string(),
                KeyInfo {
                    value: "test".to_string(),
                    operation: "".to_string(),
                },
            ),
            (
                "selinux".to_string(),
                KeyInfo {
                    value: "1".to_string(),
                    operation: "delete".to_string(),
                },
            ),
            (
                "acpi".to_string(),
                KeyInfo {
                    value: "off".to_string(),
                    operation: "delete".to_string(),
                },
            ),
            (
                "ro".to_string(),
                KeyInfo {
                    value: "1".to_string(),
                    operation: "".to_string(),
                },
            ),
        ]);
        let mut config = Sysconfig {
            model: GRUB_CMDLINE_CURRENT.to_string(),
            config_path: String::new(),
            contents: config_first_part,
        };
        grub_cmdline.set_config(&mut config).unwrap();
        grub_cmdline.is_cur_partition = false;
        let config_second = HashMap::from([
            (
                "pci".to_string(),
                KeyInfo {
                    value: "nomis".to_string(),
                    operation: "".to_string(),
                },
            ),
            (
                "panic".to_string(),
                KeyInfo {
                    value: "5".to_string(),
                    operation: "".to_string(),
                },
            ),
        ]);
        config.contents = config_second;
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
    }

    #[test]
    fn test_create_config_file() {
        init();
        let tmp_file = "/tmp/kubeos-test-create-config-file.txt";
        create_config_file(&tmp_file).unwrap();
        assert!(is_file_exist(&tmp_file));
        fs::remove_file(tmp_file).unwrap();
    }
}
