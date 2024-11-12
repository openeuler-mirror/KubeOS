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

use std::{collections::HashMap, path::Path};

use agent_error::Error;
use cli::{
    client::Client,
    method::{
        callable_method::RpcMethod, configure::ConfigureMethod, prepare_upgrade::PrepareUpgradeMethod,
        rollback::RollbackMethod, upgrade::UpgradeMethod,
    },
};
use manager::api::{CertsInfo, ConfigureRequest, KeyInfo as AgentKeyInfo, Sysconfig as AgentSysconfig, UpgradeRequest};

pub struct UpgradeInfo {
    pub version: String,
    pub image_type: String,
    pub check_sum: String,
    pub container_image: String,
    pub imageurl: String,
    pub flagsafe: bool,
    pub mtls: bool,
    pub cacert: String,
    pub clientcert: String,
    pub clientkey: String,
}

pub struct ConfigInfo {
    pub configs: Vec<Sysconfig>,
}

pub struct Sysconfig {
    pub model: String,
    pub config_path: String,
    pub contents: HashMap<String, KeyInfo>,
}

pub struct KeyInfo {
    pub value: serde_json::Value,
    pub operation: String,
}

pub trait AgentMethod {
    fn prepare_upgrade_method(&self, upgrade_info: UpgradeInfo) -> Result<(), Error>;
    fn upgrade_method(&self) -> Result<(), Error>;
    fn rollback_method(&self) -> Result<(), Error>;
    fn configure_method(&self, config_info: ConfigInfo) -> Result<(), Error>;
}
pub trait AgentCall {
    fn call_agent<T: RpcMethod + 'static>(&self, client: &Client, method: T) -> Result<(), Error>;
}

pub struct AgentClient<T: AgentCall> {
    pub agent_client: Client,
    pub agent_call_client: T,
}

impl<T: AgentCall> AgentClient<T> {
    pub fn new<P: AsRef<Path>>(socket_path: P, agent_call_client: T) -> Self {
        AgentClient { agent_client: Client::new(socket_path), agent_call_client }
    }
}

#[derive(Default)]
pub struct AgentCallClient {}
impl AgentCall for AgentCallClient {
    fn call_agent<T: RpcMethod + 'static>(&self, client: &Client, method: T) -> Result<(), Error> {
        match method.call(client) {
            Ok(_resp) => Ok(()),
            Err(e) => Err(Error::AgentError { source: e }),
        }
    }
}

impl<T: AgentCall> AgentMethod for AgentClient<T> {
    fn prepare_upgrade_method(&self, upgrade_info: UpgradeInfo) -> Result<(), Error> {
        let upgrade_request = UpgradeRequest {
            version: upgrade_info.version,
            image_type: upgrade_info.image_type,
            check_sum: upgrade_info.check_sum,
            container_image: upgrade_info.container_image,
            image_url: upgrade_info.imageurl,
            flag_safe: upgrade_info.flagsafe,
            mtls: upgrade_info.mtls,
            certs: CertsInfo {
                ca_cert: upgrade_info.cacert,
                client_cert: upgrade_info.clientcert,
                client_key: upgrade_info.clientkey,
            },
        };
        match self.agent_call_client.call_agent(&self.agent_client, PrepareUpgradeMethod::new(upgrade_request)) {
            Ok(_resp) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn upgrade_method(&self) -> Result<(), Error> {
        match self.agent_call_client.call_agent(&self.agent_client, UpgradeMethod::default()) {
            Ok(_resp) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn rollback_method(&self) -> Result<(), Error> {
        match self.agent_call_client.call_agent(&self.agent_client, RollbackMethod::default()) {
            Ok(_resp) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn configure_method(&self, config_info: ConfigInfo) -> Result<(), Error> {
        let mut agent_configs: Vec<AgentSysconfig> = Vec::new();
        for config in config_info.configs {
            let mut contents_tmp: HashMap<String, AgentKeyInfo> = HashMap::new();
            for (key, key_info) in config.contents.iter() {
                contents_tmp.insert(
                    key.to_string(),
                    AgentKeyInfo { value: key_info.value.clone(), operation: key_info.operation.clone() },
                );
            }
            agent_configs.push(AgentSysconfig {
                model: config.model,
                config_path: config.config_path,
                contents: contents_tmp,
            })
        }
        let config_request = ConfigureRequest { configs: agent_configs };
        match self.agent_call_client.call_agent(&self.agent_client, ConfigureMethod::new(config_request)) {
            Ok(_resp) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

pub mod agent_error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("{source}")]
        AgentError { source: anyhow::Error },
    }
}
