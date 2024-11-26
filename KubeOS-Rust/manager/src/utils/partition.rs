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

use anyhow::{bail, Context, Result};
use log::trace;

use super::executor::CommandExecutor;

#[derive(PartialEq, Debug, Default)]
pub struct PartitionInfo {
    pub device: String,
    pub menuentry: String,
    pub fs_type: String,
    pub size: i64,
}

/// get_partition_info returns the current partition info and the next partition info.
pub fn get_partition_info<T: CommandExecutor>(executor: &T) -> Result<(PartitionInfo, PartitionInfo), anyhow::Error> {
    let lsblk = executor.run_command_with_output("lsblk", &["-blno", "NAME,MOUNTPOINT,FSTYPE,SIZE,LABEL"])?;
    let mut cur_partition = PartitionInfo::default();
    let mut next_partition = PartitionInfo::default();
    let mut found_boot = 0;
    trace!("get_partition_info lsblk command output:\n{}", lsblk);
    for line in lsblk.lines() {
        let res: Vec<&str> = line.split_whitespace().collect();
        if res.len() == 5 && res[4] == "BOOT" {
            trace!("Found boot partition:\n{:?}", res);
            found_boot = 2;
            continue;
        }
        if found_boot > 0 {
            trace!("Handling two root partitions:\n{:?}", res);
            if res[1] == "/" {
                // current partition
                cur_partition.device = format!("/dev/{}", res[0]).to_string();
                cur_partition.fs_type = res[2].to_string();
                cur_partition.size = res[3]
                    .parse()
                    .with_context(|| format!("Failed to parse current partition size to i64: \"{}\"", res[3]))?;
                cur_partition.menuentry = if res[0].contains("2") { String::from("A") } else { String::from("B") };
            } else {
                // next partition
                next_partition.device = format!("/dev/{}", res[0]).to_string();
                next_partition.fs_type = res[1].to_string();
                next_partition.size = res[2]
                    .parse()
                    .with_context(|| format!("Failed to parse next partition size to i64: \"{}\"", res[2]))?;
                next_partition.menuentry = if res[0].contains("2") { String::from("A") } else { String::from("B") };
            }
            found_boot -= 1;
        }
    }
    if cur_partition.menuentry.is_empty() || next_partition.menuentry.is_empty() {
        bail!("Failed to get partition info, lsblk output: {}", lsblk);
    }
    Ok((cur_partition, next_partition))
}

#[cfg(test)]
mod tests {
    use mockall::{mock, predicate::*};

    use super::*;

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
    fn test_get_partition_info() {
        init();
        let command_output1 = r#"vda                   23622320128
vda1 /boot/efi vfat      61865984 BOOT
vda2 /         ext4    3145728000 ROOT-A
vda3           ext4    2621440000 ROOT-B
vda4 /persist  ext4   17791188992 PERSIST
"#;
        let mut mock = MockCommandExec::new();
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(command_output1.to_string()));
        let res = get_partition_info(&mock).unwrap();
        let expect_res = (
            PartitionInfo {
                device: "/dev/vda2".to_string(),
                menuentry: "A".to_string(),
                fs_type: "ext4".to_string(),
                size: 3145728000,
            },
            PartitionInfo {
                device: "/dev/vda3".to_string(),
                menuentry: "B".to_string(),
                fs_type: "ext4".to_string(),
                size: 2621440000,
            },
        );
        assert_eq!(res, expect_res);

        let command_output2 = r#"vda                   23622320128
vda1 /boot/efi vfat      61865984 BOOT
vda2           ext4    3145728000 ROOT-A
vda3 /         ext4    2621440000 ROOT-B
vda4 /persist  ext4   17791188992 PERSIST
"#;
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(command_output2.to_string()));
        let res = get_partition_info(&mock).unwrap();
        let expect_res = (
            PartitionInfo {
                device: "/dev/vda3".to_string(),
                menuentry: "B".to_string(),
                fs_type: "ext4".to_string(),
                size: 2621440000,
            },
            PartitionInfo {
                device: "/dev/vda2".to_string(),
                menuentry: "A".to_string(),
                fs_type: "ext4".to_string(),
                size: 3145728000,
            },
        );
        assert_eq!(res, expect_res);

        let command_output3 = "";
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(command_output3.to_string()));
        let res = get_partition_info(&mock);
        assert!(res.is_err());

        let command_output4 = "sda4 / ext4 13000245248";
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(command_output4.to_string()));
        let res = get_partition_info(&mock);
        assert!(res.is_err());
    }
}
