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
    let mut cur_partition = PartitionInfo::default();
    let mut next_partition = PartitionInfo::default();
    cur_partition.device = executor.run_command_with_output("findmnt", &["-no", "SOURCE", "--mountpoint", "/"])?;
    trace!("{} is mounted on /", cur_partition.device);
    if cur_partition.device.contains('2') {
        cur_partition.menuentry = String::from("A");
        next_partition.menuentry = String::from("B");
        next_partition.device = cur_partition.device.replace("2", "3");
    } else if cur_partition.device.contains('3') {
        cur_partition.menuentry = String::from("B");
        next_partition.menuentry = String::from("A");
        next_partition.device = cur_partition.device.replace("3", "2");
    } else {
        bail!("Failed to get partition info, / is not mounted on the second or the third partition");
    }
    let lsblk = executor.run_command_with_output("lsblk", &["-blno", "FSTYPE,SIZE", &cur_partition.device])?;
    trace!("get_partition_info lsblk command output:\n{}", lsblk);
    let elements: Vec<&str> = lsblk.split_whitespace().collect();
    if elements.len() != 2 {
        bail!("Failed to get partition info of FSTYPE and SIZE, lsblk output: {}", lsblk);
    }
    cur_partition.fs_type = elements[0].to_string();
    next_partition.fs_type = elements[0].to_string();
    cur_partition.size = elements[1]
        .parse()
        .with_context(|| format!("Failed to parse current partition size to i64: \"{}\"", elements[1]))?;
    next_partition.size = cur_partition.size;
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
        let findmnt_output1 = "/dev/vda2";
        let lsblk_output1 = "ext4    3145728000\n";
        let mut mock = MockCommandExec::new();
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(findmnt_output1.to_string()));
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(lsblk_output1.to_string()));
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
                size: 3145728000,
            },
        );
        assert_eq!(res, expect_res);

        let findmnt_output2 = "/dev/vda3";
        let lsblk_output2 = "ext4    3145728000\n";
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(findmnt_output2.to_string()));
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(lsblk_output2.to_string()));
        let res = get_partition_info(&mock).unwrap();
        let expect_res = (
            PartitionInfo {
                device: "/dev/vda3".to_string(),
                menuentry: "B".to_string(),
                fs_type: "ext4".to_string(),
                size: 3145728000,
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

        let findmnt_output3 = "/dev/vda4";
        mock.expect_run_command_with_output().times(1).returning(|_, _| Ok(findmnt_output3.to_string()));
        let res = get_partition_info(&mock);
        assert!(res.is_err());
    }
}
