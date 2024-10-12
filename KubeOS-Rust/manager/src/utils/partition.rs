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

use anyhow::{bail, Result};
use log::{debug, trace};

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
    let lsblk = executor.run_command_with_output("lsblk", &["-blno", "NAME,MOUNTPOINT,FSTYPE,SIZE"])?;
    // After split whitespace, the root directory line should have 3 elements, which are "sda2 / ext4".
    let mut cur_partition = PartitionInfo::default();
    let mut next_partition = PartitionInfo::default();
    let splitted_len = 4;
    trace!("get_partition_info lsblk command output:\n{}", lsblk);
    for line in lsblk.lines() {
        let res: Vec<&str> = line.split_whitespace().collect();
        if res.len() == splitted_len && res[1] == "/" {
            debug!("root directory line: device={}, fs_type={}", res[0], res[2]);
            cur_partition.device = format!("/dev/{}", res[0]).to_string();
            cur_partition.fs_type = res[2].to_string();
            next_partition.fs_type = res[2].to_string();
            // convert &str to i64. if the conversion fails, use 0.
            cur_partition.size = res[3].parse().unwrap_or(0);
            // root partition is the same size.
            next_partition.size = cur_partition.size;
            if res[0].contains('2') {
                // root directory is mounted on sda2, so sda3 is the next partition
                cur_partition.menuentry = String::from("A");
                next_partition.menuentry = String::from("B");
                next_partition.device = format!("/dev/{}", res[0].replace('2', "3")).to_string();
            } else if res[0].contains('3') {
                // root directory is mounted on sda3, so sda2 is the next partition
                cur_partition.menuentry = String::from("B");
                next_partition.menuentry = String::from("A");
                next_partition.device = format!("/dev/{}", res[0].replace('3', "2")).to_string();
            }
        }
    }
    if cur_partition.menuentry.is_empty() {
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
        let command_output1 = "sda\nsda1 /boot/efi vfat 98566144\nsda2 / ext4 13000245248\nsda3  ext4 13000245248\nsda4 /persist ext4 453458788352\nsr0  iso9660 964689261\n";
        let mut mock = MockCommandExec::new();
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(command_output1.to_string()));
        let res = get_partition_info(&mock).unwrap();
        let expect_res = (
            PartitionInfo { device: "/dev/sda2".to_string(), menuentry: "A".to_string(), fs_type: "ext4".to_string(), size: 13000245248},
            PartitionInfo { device: "/dev/sda3".to_string(), menuentry: "B".to_string(), fs_type: "ext4".to_string(), size: 13000245248},
        );
        assert_eq!(res, expect_res);

        let command_output2 = "sda\nsda1 /boot/efi vfat 98566144\nsda2   ext4 13000245248\nsda3 / ext4 13000245248\nsda4 /persist ext4 453458788352\nsr0  iso9660 964689261\n";
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(command_output2.to_string()));
        let res = get_partition_info(&mock).unwrap();
        let expect_res = (
            PartitionInfo { device: "/dev/sda3".to_string(), menuentry: "B".to_string(), fs_type: "ext4".to_string(), size: 13000245248},
            PartitionInfo { device: "/dev/sda2".to_string(), menuentry: "A".to_string(), fs_type: "ext4".to_string(), size: 13000245248},
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
