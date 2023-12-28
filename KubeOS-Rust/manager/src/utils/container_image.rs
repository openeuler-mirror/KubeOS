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
use log::{debug, info, trace};
use regex::Regex;

use super::executor::CommandExecutor;

pub fn is_valid_image_name(image: &str) -> Result<()> {
    let pattern = r"^(?P<Registry>[a-z0-9\-.]+\.[a-z0-9\-]+:?[0-9]*)?/?((?P<Name>[a-zA-Z0-9-_]+?)|(?P<UserName>[a-zA-Z0-9-_]+?)/(?P<ImageName>[a-zA-Z-_]+?))(?P<Tag>(?::[\w_.-]+)?|(?:@sha256:[a-fA-F0-9]+)?)$";
    let reg_ex = Regex::new(pattern)?;
    if !reg_ex.is_match(image) {
        bail!("Invalid image name: {}", image);
    }
    debug!("Image name {} is valid", image);
    Ok(())
}

pub fn check_oci_image_digest<T: CommandExecutor>(
    container_runtime: &str,
    image_name: &str,
    check_sum: &str,
    command_executor: &T,
) -> Result<()> {
    let image_digests = get_oci_image_digest(container_runtime, image_name, command_executor)?;
    if image_digests.to_lowercase() != check_sum.to_lowercase() {
        bail!(
            "Image digest mismatch, expect {}, got {}",
            check_sum,
            image_digests
        );
    }
    Ok(())
}

pub fn get_oci_image_digest<T: CommandExecutor>(
    container_runtime: &str,
    image_name: &str,
    executor: &T,
) -> Result<String> {
    let cmd_output: String;
    match container_runtime {
        "crictl" => {
            cmd_output = executor.run_command_with_output(
                "crictl",
                &[
                    "inspecti",
                    "--output",
                    "go-template",
                    "--template",
                    "{{.status.repoDigests}}",
                    image_name,
                ],
            )?;
        }
        "docker" => {
            cmd_output = executor.run_command_with_output(
                "docker",
                &["inspect", "--format", "{{.RepoDigests}}", image_name],
            )?;
        }
        "ctr" => {
            cmd_output = executor.run_command_with_output(
                "ctr",
                &[
                    "-n",
                    "k8s.io",
                    "images",
                    "ls",
                    &format!("name=={}", image_name),
                ],
            )?;
            // Split by whitespaces, we get vec like [REF TYPE DIGEST SIZE PLATFORMS LABELS x x x x x x]
            // get the 8th element, and split by ':' to get the digest
            let fields: Vec<&str> = cmd_output.split_whitespace().collect();
            if let Some(digest) = fields.get(8).and_then(|field| field.split(':').nth(1)) {
                trace!("get_oci_image_digest: {}", digest);
                return Ok(digest.to_string());
            } else {
                bail!(
                    "Failed to get digest from ctr command output: {}",
                    cmd_output
                );
            }
        }
        _ => {
            bail!(
                "Container runtime {} cannot be recognized",
                container_runtime
            );
        }
    }

    // Parse the cmd_output to extract the digest
    let parts: Vec<&str> = cmd_output.split('@').collect();
    if let Some(last_part) = parts.last() {
        if last_part.starts_with("sha256") {
            let parsed_parts: Vec<&str> = last_part.trim_matches(|c| c == ']').split(':').collect();
            // After spliiing by ':', we should get vec like [sha256, digests]
            if parsed_parts.len() == 2 {
                debug!("get_oci_image_digest: {}", parsed_parts[1]);
                return Ok(parsed_parts[1].to_string()); // 1 is the index of digests
            }
        }
    }

    bail!("Failed to get digest from command output: {}", cmd_output)
}

pub fn pull_image<T: CommandExecutor>(runtime: &str, image_name: &str, executor: &T) -> Result<()> {
    debug!("Pull image {}", image_name);
    match runtime {
        "crictl" => {
            executor.run_command("crictl", &["pull", image_name])?;
        }
        "ctr" => {
            executor.run_command(
                "ctr",
                &[
                    &"-n",
                    "k8s.io",
                    "images",
                    "pull",
                    "--hosts-dir",
                    "/etc/containerd/certs.d",
                    image_name,
                ],
            )?;
        }
        "docker" => {
            executor.run_command("docker", &["pull", image_name])?;
        }
        _ => {
            bail!("Container runtime {} cannot be recognized", runtime);
        }
    }
    Ok(())
}

pub fn remove_image_if_exist<T: CommandExecutor>(
    runtime: &str,
    image_name: &str,
    executor: &T,
) -> Result<()> {
    match runtime {
        "crictl" => {
            if executor
                .run_command("crictl", &["inspecti", image_name])
                .is_ok()
            {
                executor.run_command("crictl", &["rmi", image_name])?;
                info!("Remove existing upgrade image: {}", image_name);
            }
        }
        "ctr" => {
            let output = executor.run_command_with_output(
                "ctr",
                &[
                    &"-n",
                    "k8s.io",
                    "images",
                    "check",
                    &format!("name=={}", image_name),
                ],
            )?;
            if !output.is_empty() {
                executor.run_command(
                    "ctr",
                    &[&"-n", "k8s.io", "images", "rm", image_name, "--sync"],
                )?;
                info!("Remove existing upgrade image: {}", image_name);
            }
        }
        "docker" => {
            if executor
                .run_command("docker", &["inspect", image_name])
                .is_ok()
            {
                executor.run_command("docker", &["rmi", image_name])?;
                info!("Remove existing upgrade image: {}", image_name);
            }
        }
        _ => {
            bail!("Container runtime {} cannot be recognized", runtime);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::{mock, predicate::*};

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
    fn test_is_valid_image_name() {
        init();
        let out = is_valid_image_name("nginx").unwrap();
        assert_eq!(out, ());
        let out = is_valid_image_name(
            "docker.example.com:5000/gmr/alpine@sha256:11111111111111111111111111111111",
        )
        .unwrap();
        assert_eq!(out, ());
        let out = is_valid_image_name(
            "sosedoff/pgweb:latest@sha256:5a156ff125e5a12ac7ff43ee5120fa249cf62248337b6d04574c8",
        );
        match out {
            Ok(_) => assert_eq!(true, false),
            Err(_) => assert_eq!(true, true),
        }
    }

    #[test]
    fn test_get_oci_image_digest() {
        init();
        let mut mock = MockCommandExec::new();
        let container_runtime = "ctr";
        let image_name = "docker.io/nginx:latest";
        let command_output1 = "REF TYPE DIGEST SIZE PLATFORMS LABELS\ndocker.io/nginx:latest text/html sha256:1111 132.5 KIB - -\n";
        mock.expect_run_command_with_output()
            .times(1)
            .returning(|_, _| Ok(command_output1.to_string()));
        let out1 = get_oci_image_digest(container_runtime, image_name, &mock).unwrap();
        let expect_output = "1111";
        assert_eq!(out1, expect_output);

        let container_runtime = "crictl";
        let command_output2 = "[docker.io/nginx@sha256:1111]";
        mock.expect_run_command_with_output()
            .times(1)
            .returning(|_, _| Ok(command_output2.to_string()));
        let out2 = get_oci_image_digest(container_runtime, image_name, &mock).unwrap();
        assert_eq!(out2, expect_output);
    }

    #[test]
    fn test_check_oci_image_digest_match() {
        init();
        let mut mock = MockCommandExec::new();
        let image_name = "docker.io/nginx:latest";
        let container_runtime = "crictl";
        let command_output = "[docker.io/nginx@sha256:1111]";
        let check_sum = "1111";
        mock.expect_run_command_with_output()
            .times(1)
            .returning(|_, _| Ok(command_output.to_string()));
        let result = check_oci_image_digest(container_runtime, image_name, check_sum, &mock);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pull_image() {
        init();
        let mut mock_executor = MockCommandExec::new();

        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "crictl" && args.len() == 2 && args[0] == "pull") // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(()));

        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "ctr" && args.len() == 7 && args[3] == "pull") // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(()));

        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "docker" && args.len() == 2 && args[0] == "pull") // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(()));

        let image_name = "docker.io/nginx:latest";
        let result = pull_image("crictl", image_name, &mock_executor);
        assert!(result.is_ok());
        let result = pull_image("ctr", image_name, &mock_executor);
        assert!(result.is_ok());
        let result = pull_image("docker", image_name, &mock_executor);
        assert!(result.is_ok());
        let result = pull_image("aaa", image_name, &mock_executor);
        assert!(result.is_err());
    }
}
