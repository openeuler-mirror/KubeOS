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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::agent_status::*;
use crate::{
    sys_mgmt::{CtrImageHandler, DiskImageHandler, DockerImageHandler},
    utils::{CommandExecutor, UpgradeImageManager},
};

#[derive(Deserialize, Serialize, Debug)]
pub struct UpgradeRequest {
    pub version: String,
    pub check_sum: String,
    pub image_type: String,
    pub container_image: String,
    pub image_url: String,
    pub flag_safe: bool,
    pub mtls: bool,
    pub certs: CertsInfo,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CertsInfo {
    pub ca_cert: String,
    pub client_cert: String,
    pub client_key: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct KeyInfo {
    pub value: String,
    pub operation: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Sysconfig {
    pub model: String,
    pub config_path: String,
    pub contents: HashMap<String, KeyInfo>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ConfigureRequest {
    pub configs: Vec<Sysconfig>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct Response {
    pub status: AgentStatus,
}

pub enum ImageType<T: CommandExecutor> {
    Containerd(CtrImageHandler<T>),
    Docker(DockerImageHandler<T>),
    Disk(DiskImageHandler<T>),
}

impl<T: CommandExecutor> ImageType<T> {
    pub fn download_image(&self, req: &UpgradeRequest) -> anyhow::Result<UpgradeImageManager<T>> {
        match self {
            ImageType::Containerd(handler) => handler.download_image(req),
            ImageType::Docker(handler) => handler.download_image(req),
            ImageType::Disk(handler) => handler.download_image(req),
        }
    }
}
pub trait ImageHandler<T: CommandExecutor> {
    fn download_image(&self, req: &UpgradeRequest) -> anyhow::Result<UpgradeImageManager<T>>;
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use mockall::mock;

    use super::*;
    use crate::utils::PreparePath;

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

    #[test]
    fn test_download_image() {
        let req = UpgradeRequest {
            version: "KubeOS v2".to_string(),
            image_type: "containerd".to_string(),
            container_image: "kubeos-temp".to_string(),
            check_sum: "22222".to_string(),
            image_url: "".to_string(),
            flag_safe: false,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };

        let mut mock_executor1 = MockCommandExec::new();
        mock_executor1.expect_run_command().returning(|_, _| Ok(()));
        mock_executor1.expect_run_command_with_output().returning(|_, _| Ok(String::new()));
        let c_handler = CtrImageHandler::new(PreparePath::default(), mock_executor1);
        let image_type = ImageType::Containerd(c_handler);
        let result = image_type.download_image(&req);
        assert!(result.is_err());

        let mut mock_executor2 = MockCommandExec::new();
        mock_executor2.expect_run_command().returning(|_, _| Ok(()));
        mock_executor2.expect_run_command_with_output().returning(|_, _| Ok(String::new()));
        let docker_handler = DockerImageHandler::new(PreparePath::default(), "test".into(), mock_executor2);
        let image_type = ImageType::Docker(docker_handler);
        let result = image_type.download_image(&req);
        assert!(result.is_err());

        let mut mock_executor3 = MockCommandExec::new();
        mock_executor3.expect_run_command().returning(|_, _| Ok(()));
        mock_executor3.expect_run_command_with_output().returning(|_, _| Ok(String::new()));
        let disk_handler = DiskImageHandler::new(PreparePath::default(), mock_executor3, "test".into());
        let image_type = ImageType::Disk(disk_handler);
        let result = image_type.download_image(&req);
        assert!(result.is_err());
    }
}
