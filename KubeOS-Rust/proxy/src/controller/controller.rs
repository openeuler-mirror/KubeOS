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

use super::crd::{Content, OSInstance, OS};
use super::drain::drain_os;
use super::utils::{check_version, get_config_version, ConfigOperation, ConfigType};
use super::values::{
    LABEL_UPGRADING, NODE_STATUS_CONFIG, NODE_STATUS_IDLE, OPERATION_TYPE_ROLLBACK,
    OPERATION_TYPE_UPGRADE, REQUEUE_ERROR, REQUEUE_NORMAL,
};
use super::{
    apiclient::{ApplyApi, ControllerClient},
    crd::Configs,
};
use anyhow::Result;
use cli::{
    client::Client as AgentClient,
    method::{
        callable_method::RpcMethod, configure::ConfigureMethod,
        prepare_upgrade::PrepareUpgradeMethod, rollback::RollbackMethod, upgrade::UpgradeMethod,
    },
};
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{Api, PostParams},
    core::ErrorResponse,
    runtime::controller::{Context, ReconcilerAction},
    Client, ResourceExt,
};
use log::{debug, error, info};
use manager::api::{ConfigureRequest, KeyInfo, Sysconfig as AgentSysconfig, UpgradeRequest};
use reconciler_error::Error;
use std::collections::HashMap;
use std::env;

pub async fn reconcile(
    os: OS,
    ctx: Context<ProxyController<ControllerClient>>,
) -> Result<ReconcilerAction, Error> {
    debug!("start reconcile");
    let proxy_controller = ctx.get_ref();
    let os_cr = &os;
    let node_name = env::var("NODE_NAME")?;
    let namespace: String = os_cr.namespace().ok_or(Error::MissingObjectKey {
        resource: "os".to_string(),
        value: "namespace".to_string(),
    })?;
    proxy_controller
        .check_osi_exisit(&namespace, &node_name)
        .await?;
    let controller_res = proxy_controller
        .get_resources(&namespace, &node_name)
        .await?;
    let node = controller_res.node;
    let mut osinstance = controller_res.osinstance;
    let node_os_image = &node
        .status
        .as_ref()
        .ok_or(Error::MissingSubResource {
            value: String::from("node.status"),
        })?
        .node_info
        .as_ref()
        .ok_or(Error::MissingSubResource {
            value: String::from("node.status.node_info"),
        })?
        .os_image;
    debug!(
        "os expected osversion is {},actual osversion is {}",
        os_cr.spec.osversion, node_os_image
    );
    if check_version(&os_cr.spec.osversion, &node_os_image) {
        match ConfigType::SysConfig.check_config_version(&os, &osinstance) {
            ConfigOperation::Reassign => {
                debug!("start reassign");
                proxy_controller
                    .refresh_node(
                        node,
                        osinstance,
                        &get_config_version(os_cr.spec.sysconfigs.as_ref()),
                        ConfigType::SysConfig,
                    )
                    .await?;
                return Ok(REQUEUE_NORMAL);
            }
            ConfigOperation::UpdateConfig => {
                debug!("start update config");
                osinstance.spec.sysconfigs = os_cr.spec.sysconfigs.clone();
                proxy_controller
                    .controller_client
                    .update_osinstance_spec(&osinstance.name(), &namespace, &osinstance.spec)
                    .await?;
                return Ok(REQUEUE_ERROR);
            }
            _ => {}
        }
        proxy_controller
            .set_config(&mut osinstance, ConfigType::SysConfig)
            .await?;
        proxy_controller
            .refresh_node(
                node,
                osinstance,
                &get_config_version(os_cr.spec.sysconfigs.as_ref()),
                ConfigType::SysConfig,
            )
            .await?;
    } else {
        if os_cr.spec.opstype == NODE_STATUS_CONFIG {
            return Err(Error::UpgradeBeforeConfig);
        }
        match ConfigType::UpgradeConfig.check_config_version(&os, &osinstance) {
            ConfigOperation::Reassign => {
                debug!("start reassign");
                proxy_controller
                    .refresh_node(
                        node,
                        osinstance,
                        &get_config_version(os_cr.spec.upgradeconfigs.as_ref()),
                        ConfigType::UpgradeConfig,
                    )
                    .await?;
                return Ok(REQUEUE_NORMAL);
            }
            _ => {}
        }
        if node.labels().contains_key(LABEL_UPGRADING) {
            if osinstance.spec.nodestatus == NODE_STATUS_IDLE {
                info!(
                    "node has upgrade label ,but osinstance.spec.nodestatus is idle. Operation:refesh node and wait reassgin"
                );
                proxy_controller
                    .refresh_node(
                        node,
                        osinstance,
                        &get_config_version(os_cr.spec.upgradeconfigs.as_ref()),
                        ConfigType::UpgradeConfig,
                    )
                    .await?;
                return Ok(REQUEUE_NORMAL);
            }
            proxy_controller
                .set_config(&mut osinstance, ConfigType::UpgradeConfig)
                .await?;
            proxy_controller.upgrade_node(os_cr, &node).await?;
        }
    }
    Ok(REQUEUE_NORMAL)
}

pub fn error_policy(
    error: &Error,
    _ctx: Context<ProxyController<ControllerClient>>,
) -> ReconcilerAction {
    error!("Reconciliation error:{}", error.to_string());
    REQUEUE_ERROR
}

struct ControllerResources {
    osinstance: OSInstance,
    node: Node,
}
pub struct ProxyController<T: ApplyApi> {
    k8s_client: Client,
    controller_client: T,
    agent_client: AgentClient,
}

impl<T: ApplyApi> ProxyController<T> {
    pub fn new(k8s_client: Client, controller_client: T, agent_client: AgentClient) -> Self {
        ProxyController {
            k8s_client,
            controller_client,
            agent_client,
        }
    }
}

impl<T: ApplyApi> ProxyController<T> {
    async fn check_osi_exisit(&self, namespace: &str, node_name: &str) -> Result<(), Error> {
        let osi_api: Api<OSInstance> = Api::namespaced(self.k8s_client.clone(), namespace);
        match osi_api.get(node_name).await {
            Ok(osi) => {
                debug!("osinstance is exist {:?}", osi.name());
                return Ok(());
            }
            Err(kube::Error::Api(ErrorResponse { reason, .. })) if &reason == "NotFound" => {
                info!("Create OSInstance {}", node_name);
                self.controller_client
                    .create_osinstance(node_name, namespace)
                    .await?;
                Ok(())
            }
            Err(err) => Err(Error::KubeError { source: err }),
        }
    }

    async fn get_resources(
        &self,
        namespace: &str,
        node_name: &str,
    ) -> Result<ControllerResources, Error> {
        let osi_api: Api<OSInstance> = Api::namespaced(self.k8s_client.clone(), namespace);
        let osinstance_cr = osi_api.get(node_name).await?;
        let node_api: Api<Node> = Api::all(self.k8s_client.clone());
        let node_cr = node_api.get(node_name).await?;
        Ok(ControllerResources {
            osinstance: osinstance_cr,
            node: node_cr,
        })
    }

    async fn refresh_node(
        &self,
        mut node: Node,
        osinstance: OSInstance,
        os_config_version: &str,
        config_type: ConfigType,
    ) -> Result<(), Error> {
        debug!("start refresh_node");
        let node_api: Api<Node> = Api::all(self.k8s_client.clone());
        let labels = node.labels_mut();
        if labels.contains_key(LABEL_UPGRADING) {
            labels.remove(LABEL_UPGRADING);
            node = node_api
                .replace(&node.name(), &PostParams::default(), &node)
                .await?;
        }
        if let Some(node_spec) = &node.spec {
            if let Some(node_unschedulable) = node_spec.unschedulable {
                if node_unschedulable {
                    node_api.uncordon(&node.name()).await?;
                    info!("Uncordon successfully node{}", node.name());
                }
            }
        }
        self.update_node_status(osinstance, os_config_version, config_type)
            .await?;
        Ok(())
    }

    async fn update_node_status(
        &self,
        mut osinstance: OSInstance,
        os_config_version: &str,
        config_type: ConfigType,
    ) -> Result<(), Error> {
        debug!("start update_node_status");
        if osinstance.spec.nodestatus == NODE_STATUS_IDLE {
            return Ok(());
        }
        let upgradeconfig_spec_version =
            get_config_version(osinstance.spec.upgradeconfigs.as_ref());
        let sysconfig_spec_version = get_config_version(osinstance.spec.sysconfigs.as_ref());
        let sysconfig_status_version: String;
        if let Some(osinstance_status) = osinstance.status.as_ref() {
            sysconfig_status_version = get_config_version(osinstance_status.sysconfigs.as_ref());
        } else {
            sysconfig_status_version = get_config_version(None);
        }
        if sysconfig_spec_version == sysconfig_status_version
            || (config_type == ConfigType::SysConfig && os_config_version != sysconfig_spec_version)
            || (config_type == ConfigType::UpgradeConfig
                && os_config_version != upgradeconfig_spec_version)
        {
            let namespace = osinstance.namespace().ok_or(Error::MissingObjectKey {
                resource: String::from("osinstance"),
                value: String::from("namespace"),
            })?;
            osinstance.spec.nodestatus = NODE_STATUS_IDLE.to_string();
            self.controller_client
                .update_osinstance_spec(&osinstance.name(), &namespace, &osinstance.spec)
                .await?;
        }
        Ok(())
    }

    async fn update_osi_status(
        &self,
        osinstance: &mut OSInstance,
        config_type: ConfigType,
    ) -> Result<(), Error> {
        debug!("start update_osi_status");
        config_type.set_osi_status_config(osinstance);
        debug!("osinstance status is update to {:?}", osinstance.status);
        let namespace = &osinstance.namespace().ok_or(Error::MissingObjectKey {
            resource: "osinstance".to_string(),
            value: "namespace".to_string(),
        })?;
        self.controller_client
            .update_osinstance_status(&osinstance.name(), &namespace, &osinstance.status)
            .await?;
        Ok(())
    }

    async fn set_config(
        &self,
        osinstance: &mut OSInstance,
        config_type: ConfigType,
    ) -> Result<(), Error> {
        debug!("start set_config");
        let config_info = config_type.check_config_start(osinstance);
        if config_info.need_config {
            match config_info.configs.and_then(convert_to_agent_config) {
                Some(agent_configs) => {
                    let config_request = ConfigureRequest {
                        configs: agent_configs,
                    };
                    match ConfigureMethod::new(config_request).call(&self.agent_client) {
                        Ok(_resp) => {}
                        Err(e) => {
                            return Err(Error::AgentError { source: e });
                        }
                    }
                }
                None => {
                    info!("config is none, no need to config");
                }
            };
            self.update_osi_status(osinstance, config_type).await?;
        }
        Ok(())
    }

    async fn upgrade_node(&self, os_cr: &OS, node: &Node) -> Result<(), Error> {
        debug!("start upgrade node");

        match os_cr.spec.opstype.as_str() {
            OPERATION_TYPE_UPGRADE => {
                let upgrade_request = UpgradeRequest {
                    version: os_cr.spec.osversion.clone(),
                    image_type: os_cr.spec.imagetype.clone(),
                    check_sum: os_cr.spec.checksum.clone(),
                    container_image: os_cr.spec.containerimage.clone(),
                };
                match PrepareUpgradeMethod::new(upgrade_request).call(&self.agent_client) {
                    Ok(_resp) => {}
                    Err(e) => {
                        return Err(Error::AgentError { source: e });
                    }
                }
                self.evict_node(&node.name(), os_cr.spec.evictpodforce)
                    .await?;
                match UpgradeMethod::new().call(&self.agent_client) {
                    Ok(_resp) => {}
                    Err(e) => {
                        return Err(Error::AgentError { source: e });
                    }
                }
            }
            OPERATION_TYPE_ROLLBACK => {
                self.evict_node(&node.name(), os_cr.spec.evictpodforce)
                    .await?;
                match RollbackMethod::new().call(&self.agent_client) {
                    Ok(_resp) => {}
                    Err(e) => {
                        return Err(Error::AgentError { source: e });
                    }
                }
            }
            _ => {
                return Err(Error::OperationError {
                    value: os_cr.spec.opstype.clone(),
                });
            }
        }
        Ok(())
    }

    async fn evict_node(&self, node_name: &str, evict_pod_force: bool) -> Result<(), Error> {
        debug!("start evict_node");
        let node_api = Api::all(self.k8s_client.clone());
        node_api.cordon(node_name).await?;
        info!("Cordon node Successfully{}, start drain nodes", node_name);
        match self.drain_node(node_name, evict_pod_force).await {
            Ok(()) => {}
            Err(e) => {
                node_api.uncordon(node_name).await?;
                info!("Drain node {} error, uncordon node successfully", node_name);
                return Err(e);
            }
        }
        Ok(())
    }

    async fn drain_node(&self, node_name: &str, force: bool) -> Result<(), Error> {
        use crate::controller::drain::error::DrainError::*;
        match drain_os(&self.k8s_client.clone(), node_name, force).await {
            Err(FindTargetPods { source, .. }) => Err(Error::KubeError { source: source }),
            Err(DeletePodsError { errors, .. }) => Err(Error::DrainNodeError {
                value: errors.join("; "),
            }),
            _ => Ok(()),
        }
    }
}

fn convert_to_agent_config(configs: Configs) -> Option<Vec<AgentSysconfig>> {
    let mut agent_configs: Vec<AgentSysconfig> = Vec::new();
    if let Some(config_list) = configs.configs {
        for config in config_list.into_iter() {
            match config.contents.and_then(convert_to_config_hashmap) {
                Some(contents_tmp) => {
                    let config_tmp = AgentSysconfig {
                        model: config.model.unwrap_or_default(),
                        config_path: config.configpath.unwrap_or_default(),
                        contents: contents_tmp,
                    };
                    agent_configs.push(config_tmp)
                }
                None => {
                    info!("model {} which has configpath {} do not has any contents no need to configure",config.model.unwrap_or_default(),config.configpath.unwrap_or_default());
                    continue;
                }
            };
        }
        if agent_configs.len() == 0 {
            info!("no contents in all models, no need to configure");
            return None;
        }
        return Some(agent_configs);
    }
    return None;
}

fn convert_to_config_hashmap(contents: Vec<Content>) -> Option<HashMap<String, KeyInfo>> {
    let mut contents_tmp: HashMap<String, KeyInfo> = HashMap::new();
    for content in contents.into_iter() {
        let key_info = KeyInfo {
            value: content.value.unwrap_or_default(),
            operation: content.operation.unwrap_or_default(),
        };
        contents_tmp.insert(content.key.unwrap_or_default(), key_info);
    }
    return Some(contents_tmp);
}

pub mod reconciler_error {
    use crate::controller::apiclient::apiclient_error;
    use thiserror::Error;
    #[derive(Error, Debug)]
    pub enum Error {
        #[error("Kubernetes reported error: {source}")]
        KubeError {
            #[from]
            source: kube::Error,
        },

        #[error("Create/Patch OSInstance reported error: {source}")]
        ApplyApiError {
            #[from]
            source: apiclient_error::Error,
        },

        #[error("Cannot get environment NODE_NAME, error: {source}")]
        EnvError {
            #[from]
            source: std::env::VarError,
        },

        #[error("{}.metadata.{} is not exist", resource, value)]
        MissingObjectKey { resource: String, value: String },

        #[error("Cannot get {}, {} is None", value, value)]
        MissingSubResource { value: String },

        #[error("operation {} cannot be recognized", value)]
        OperationError { value: String },

        #[error("Expect OS Version is not same with Node OS Version, please upgrade first")]
        UpgradeBeforeConfig,

        #[error("os-agent reported error:{source}")]
        AgentError { source: anyhow::Error },
        #[error("Error when drain node, error reported: {}", value)]
        DrainNodeError { value: String },
    }
}
