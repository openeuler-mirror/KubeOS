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

use std::process::Command;

use anyhow::{anyhow, Result};
use log::trace;

pub trait CommandExecutor: Clone {
    fn run_command<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<()>;
    fn run_command_with_output<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<String>;
}

#[derive(Clone)]
pub struct RealCommandExecutor {}

impl CommandExecutor for RealCommandExecutor {
    fn run_command<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<()> {
        let output = Command::new(name).args(args).output()?;
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Failed to run command: {} {:?}, stderr: {}",
                name,
                args,
                error_message
            ));
        }
        trace!("run_command: {} {:?} done", name, args);
        Ok(())
    }

    fn run_command_with_output<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<String> {
        let output = Command::new(name).args(args).output()?;
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!(
                "Failed to run command: {} {:?}, stderr: {}",
                name,
                args,
                error_message
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        trace!("run_command_with_output: {} {:?} done", name, args);
        Ok(stdout.trim_end_matches("\n").to_string())
    }
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
    fn test_run_command_with_output() {
        init();
        let executor: RealCommandExecutor = RealCommandExecutor {};

        // test run_command_with_output
        let output = executor
            .run_command_with_output("echo", &["hello", "world"])
            .unwrap();
        assert_eq!(output, "hello world");
        let out = executor
            .run_command_with_output("sh", &["-c", format!("command -v {}", "cat").as_str()])
            .unwrap();
        assert_eq!(out, "/usr/bin/cat");
        let out = executor
            .run_command_with_output("sh", &["-c", format!("command -v {}", "apple").as_str()]);
        assert!(out.is_err());
    }

    #[test]
    fn test_run_command() {
        init();
        let executor: RealCommandExecutor = RealCommandExecutor {};
        // test run_command
        let out = executor.run_command("sh", &["-c", format!("command -v {}", "apple").as_str()]);
        assert!(out.is_err());

        let out = executor.run_command("sh", &["-c", format!("command -v {}", "cat").as_str()]);
        assert!(out.is_ok());
    }
}
