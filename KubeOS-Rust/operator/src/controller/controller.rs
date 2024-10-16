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

use anyhow::Result;
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{Api, ListParams, ObjectList, PostParams},
    core::ErrorResponse,
    runtime::controller::{Context, ReconcilerAction},
    Client, ResourceExt,
};
use log::{debug, error};
use reconciler_error::Error;
use std::convert::TryFrom;

use common::{
    apiclient::ApplyApi,
    crd::{Configs, OSInstance, OS},
    values::{
        LABEL_MASTER, LABEL_UPGRADING, NODE_STATUS_CONFIG, NODE_STATUS_IDLE, 
        NO_REQUEUE, OPERATION_TYPE_CONFIG, OPERATION_TYPE_ROLLBACK, OPERATION_TYPE_UPGRADE, 
        REQUEUE_ERROR, REQUEUE_NORMAL, SYS_CONFIG_NAME, UPGRADE_CONFIG_NAME, NODE_STATUS_UPGRADE
    },
};

#[derive(Clone)]
pub struct OperatorController<T: ApplyApi> {
    k8s_client: Client,
    controller_client: T,
}

impl<T: ApplyApi> OperatorController<T> {
    pub fn new(k8s_client: Client, controller_client: T) -> Self {
        OperatorController {
            k8s_client,
            controller_client,
        }
    }

    // get the number of worker nodes
    async fn get_and_update_os(&self, _namespace: &str) -> Result<i64, Error> {
        let reqs = vec![
            format!("!{}", LABEL_MASTER),
        ];
        let nodes_items = self.get_nodes(0, reqs).await?;
        let len = nodes_items.items.len();
        Ok(i64::try_from(len).map_err(|_| Error::ConvertI64 { data: len.to_string() })?)
    }

    // get an array of nodes, and pass reqs as a String array, which indicates the filter conditions
    async fn get_nodes(&self, limit: i64, reqs: Vec<String>) -> Result<ObjectList<Node>, Error> {
        let nodes_api: Api<Node> = Api::all(self.k8s_client.clone());
        let label_selector = reqs.join(",");
        let list_params = ListParams::default()
            .labels(&label_selector)
            .limit(u32::try_from(limit).map_err(|_| Error::ConvertU32 { data: limit.to_string() })?);
        let nodes = match nodes_api.list(&list_params).await {
            Ok(nodes) => nodes,
            Err(e) => {
                log::error!("{:?} unable to list nodes with requirements", e);
                return Err(Error::KubeClient { source: e });
            },
        };
        Ok(nodes)
    }

    // get the number of nodes that can perform the upgrade operation
    async fn check_upgrading(&self, _namespace: &str, max_unavailable: i64) -> Result<i64, Error> {
        let reqs = vec![
            LABEL_UPGRADING.to_string(),
        ];
        let nodes_items = self.get_nodes( 0, reqs).await?;
        let nodes_len = nodes_items.items.len();
        Ok(max_unavailable - i64::try_from(nodes_len).map_err(|_| Error::ConvertI64 { data: nodes_len.to_string() })?)
    }

    // upgrade for a specified number of nodes
    async fn assign_upgrade(&self, os: &OS, limit: i64, namespace: &str) -> Result<bool, Error> {
        let reqs = vec![
            format!("!{}", LABEL_UPGRADING),
            format!("!{}", LABEL_MASTER),
        ];
        let mut nodes_items = self.get_nodes( limit + 1, reqs).await?;
        let count: i64 = self.upgrade_nodes(os, &mut nodes_items, limit, namespace).await?;
        Ok(count >= limit)
    }

    // the specific logic of the upgrade node
    async fn upgrade_nodes(&self, os: &OS, nodes: &mut ObjectList<Node>, limit: i64, namespace: &str) -> Result<i64, Error> {
        let mut count = 0;
        for node in nodes.iter_mut() {
            if count >= limit {
                break
            }
            let os_version_node = &node
                .status
                .clone()
                .ok_or(Error::MissingSubResource { value: String::from("node.status") })?
                .node_info
                .ok_or(Error::MissingSubResource { value: String::from("node.status.node_info") })?
                .os_image;
            debug!("node name: {}, os_version_node: {}, os_version: {}", node.name(), os_version_node, os.spec.osversion);
            if os_version_node != &os.spec.osversion {
                let osi_api: Api<OSInstance> = Api::namespaced(self.k8s_client.clone(), namespace);
                match osi_api.get(&node.name().clone()).await {
                    Ok(mut osi) => {
                        debug!("osinstance is exist: \n {:?} \n", osi);
                        match self.update_node_and_osins(os, node, &mut osi).await {
                            Ok(_) => {
                                count += 1;
                            },
                            Err(_) => {
                                continue;
                            },
                        }
                    },
                    Err(kube::Error::Api(ErrorResponse { reason, .. })) if &reason == "NotFound" => {
                        debug!("failed to get osInstance {}", &node.name().clone());
                        return Err(Error::KubeClient {
                                    source: kube::Error::Api(ErrorResponse { 
                                        reason, 
                                        status: "".to_string(), 
                                        message: "".to_string(), 
                                        code: 0 
                                    })});
                    },
                    Err(_) => continue,
                }
            }
        }
        Ok(count)
    }

    // upgrade the node and the OSinstance on the node
    async fn update_node_and_osins(&self, os: &OS, node: &mut Node, osinstance: &mut OSInstance, ) -> Result<(), Error> {
        debug!("start update_node_and_OSins");
        /* Check whether the upgrade configuration version in the OS instance matches the upgrade configuration version in the 
        OS object. OSI fields are copied directly when they are not initialized */
        let mut copy_sign = true;
        let os_spec_upgradeconfigs_version = os
            .spec
            .upgradeconfigs
            .clone()
            .ok_or(Error::MissingSubResource { value: String::from("os.spec.upgradeconfigs") })?
            .version
            .ok_or(Error::MissingSubResource { value: String::from("os.spec.upgradeconfigs.version") })?;
        if let Some(upgradeconfigs) = osinstance.spec.upgradeconfigs.clone() {
            if let Some(version) = upgradeconfigs.version {
                if version == os_spec_upgradeconfigs_version {
                    copy_sign = false;
                }
            }
        }
        if copy_sign {
            self.deep_copy_spec_configs(os, osinstance, UPGRADE_CONFIG_NAME.to_string()).await?;
        }
        /* Check whether the system configuration version in the OS instance matches the system configuration version in the 
        OS object. OSI fields are copied directly when they are not initialized */
        copy_sign = true;
        let os_spec_sysconfigs_version = os
            .spec
            .sysconfigs
            .clone()
            .ok_or(Error::MissingSubResource { value: String::from("os.spec.sysconfigs") })?
            .version
            .ok_or(Error::MissingSubResource { value: String::from("os.spec.sysconfigs.version") })?;
        if let Some(sysconfigs) = osinstance.spec.sysconfigs.clone() {
            if let Some(version) = sysconfigs.version {
                if version == os_spec_sysconfigs_version {
                    copy_sign = false;
                }
            }
        }
        if copy_sign {
            self.deep_copy_spec_configs(os, osinstance, SYS_CONFIG_NAME.to_string()).await?;
            if let Some(sysconfigs) = osinstance.spec.sysconfigs.as_mut() {
                if let Some(configs) = &mut sysconfigs.configs {
                    for config in configs {
                        if config.model.clone() == Some("grub.cmdline.current".to_string()) {
                            config.model = Some("grub.cmdline.next".to_string());
                        }
                        else if config.model.clone() == Some("grub.cmdline.next".to_string()) {
                            config.model = Some("grub.cmdline.current".to_string());
                        }
                    }
                }
            }
        }
        osinstance.spec.nodestatus = NODE_STATUS_UPGRADE.to_string();
        let namespace = osinstance.namespace().ok_or(Error::MissingObjectKey {
            resource: String::from("osinstance"),
            value: String::from("namespace"),
        })?;
        self.controller_client.update_osinstance_spec(&osinstance.name(), &namespace, &osinstance.spec).await?;
        node.labels_mut().insert(LABEL_UPGRADING.to_string(), "".to_string());
        let node_api: Api<Node> = Api::all(self.k8s_client.clone());
        node_api.replace(&node.name(), &PostParams::default(), &node).await?;
        Ok(())
    }

    async fn deep_copy_spec_configs(&self, os: &OS, os_instance: &mut OSInstance, config_type: String) -> Result<(), Error> {
        match config_type.as_str() {
            UPGRADE_CONFIG_NAME =>{
                if let Ok(data) = serde_json::to_vec(&os.spec.upgradeconfigs){
                    if let Ok(upgradeconfigs) = serde_json::from_slice(&data) {
                        os_instance.spec.upgradeconfigs = Some(upgradeconfigs);
                    }else {
                        debug!("{} Deserialization failure", config_type);
                        return Err(Error::Operation { value: "Deserialization".to_string()});
                    }
                }
                else {
                    debug!("{} Serialization failure", config_type);
                    return Err(Error::Operation { value: "Serialization".to_string()});
                }
            },
            SYS_CONFIG_NAME => {
                if let Ok(data) = serde_json::to_vec(&os.spec.sysconfigs){
                    if let Ok(sysconfigs) = serde_json::from_slice(&data) {
                        os_instance.spec.sysconfigs = Some(sysconfigs);
                    }else {
                        debug!("{} Deserialization failure", config_type);
                        return Err(Error::Operation { value: "Deserialization".to_string()});
                    }
                }
                else {
                    debug!("{} Serialization failure", config_type);
                    return Err(Error::Operation { value: "Serialization".to_string()});
                }
            },
            _ => {
                debug!("configType {} cannot be recognized", config_type);
                return Err(Error::Operation { value: config_type.clone() });
            },
        }
        Ok(())
    }

    // obtain the number of nodes that can perform config operations
    async fn check_config(&self, namespace: &str, max_unavailable: i64) -> Result<i64, Error> {
        let osinstances = self.get_config_osinstances(namespace).await?;
        let len = osinstances.len();
        Ok(max_unavailable - i64::try_from(len).map_err(|_| Error::ConvertI64 { data: len.to_string() })?)
    }

    // obtain the list of osinstances on the node that is in the config state
    async fn get_config_osinstances(&self, namespace: &str) -> Result<Vec<OSInstance>, Error> {
        let osi_api: Api<OSInstance> = Api::namespaced(self.k8s_client.clone(), namespace);
        // get all OSInstance 
        let all_osinstances = osi_api.list(&ListParams::default()).await?;
        // filtering on the client side with a node status of NODE_STATUS_CONFIG
        let osinstances: Vec<OSInstance> = all_osinstances
            .items
            .into_iter()
            .filter(|osi| osi.spec.nodestatus == NODE_STATUS_CONFIG)
            .collect();
        debug!("config_osi count = {:?}", osinstances.len());
        Ok(osinstances)
    }

    // perform config operations for a specified number of nodes
    async fn assign_config(&self, _os: &OS, sysconfigs: Configs, config_version: String, limit: i64, namespace: &str) -> Result<bool, Error> {
        debug!("start assign_config");
        let mut osinstances = self.get_idle_os_instances(namespace, limit + 1).await?;
        let mut count = 0;
        // traverse the OSI list
        for osi in osinstances.iter_mut() {
            if count > limit {
                break;
            }
            let mut config_sign = true;
            if let Some(sysconfigs) = osi.spec.sysconfigs.clone() {
                if let Some(version) = sysconfigs.version {
                    debug!("node name: {:?}, config_version_node: {:?}, config_version: {:?}", osi.name(), version, config_version);
                    if version == config_version {
                        config_sign = false;
                    }
                }
            }
            if config_sign {
                count += 1;
                osi.spec.sysconfigs = Some(sysconfigs.clone());
                osi.spec.nodestatus = NODE_STATUS_CONFIG.to_string();
                let namespace = osi.namespace().ok_or(Error::MissingObjectKey {
                    resource: String::from("osinstance"),
                    value: String::from("namespace"),
                })?;
                self.controller_client.update_osinstance_spec(&osi.name(), &namespace, &osi.spec).await?;
            }
        }
        Ok(count >= limit)
    }

    // obtain the list of osinstances on which the node is idle
    async fn get_idle_os_instances(&self, namespace: &str, limit: i64) -> Result<Vec<OSInstance>, Error> {
        let osi_api: Api<OSInstance> = Api::namespaced(self.k8s_client.clone(), namespace);
        // get all OSInstance 
        let all_osinstances: ObjectList<OSInstance> = osi_api.list(&ListParams::default().limit(u32::try_from(limit).map_err(|_| Error::ConvertU32 { data: limit.to_string() })?)).await?;
        // filtering on the client side with a node status of NODE_STATUS_IDLE
        let osinstances: Vec<OSInstance> = all_osinstances
            .items
            .into_iter()
            .filter(|osi| osi.spec.nodestatus == NODE_STATUS_IDLE)
            .collect();
        Ok(osinstances)
    }
}

pub async fn reconcile<T: ApplyApi>(
    os: OS,
    ctx: Context<OperatorController<T>>,
) -> Result<ReconcilerAction, Error> {
    // initialize operator_controller and OS, get NODE_NAME from environment variables
    debug!("start reconcile");
    let operator_controller = ctx.get_ref();
    let os_cr: &OS = &os;
    // get the namespace from os_cr and return an error if the namespace doesn't exist
    let namespace: String = os_cr
        .namespace()
        .ok_or(Error::MissingObjectKey { resource: "os".to_string(), value: "namespace".to_string() })?;
    debug!("namespace : {:?}", namespace);
    let node_num = match operator_controller.get_and_update_os(&namespace).await {
        Ok(node_num) => node_num,
        Err(Error::KubeClient { source: kube::Error::Api(ErrorResponse { reason, .. })}) if &reason == "NotFound" => {
            return Ok(NO_REQUEUE);
        },
        Err(_) => return Ok(REQUEUE_ERROR),
    };
    debug!("node_num : {:?}", node_num);
    let opstype = os_cr.spec.opstype.clone();
    let ops = opstype.as_str();
    debug!("opstype: {}", ops);
    match ops {
        OPERATION_TYPE_UPGRADE | OPERATION_TYPE_ROLLBACK =>{
            debug!("start upgrade OR rollback");
            let limit = operator_controller.check_upgrading(&namespace, os_cr.spec.maxunavailable.min(node_num)).await?;
            debug!("limit: {}", limit);
            let need_requeue = operator_controller.assign_upgrade(os_cr, limit, &namespace).await?;
            if need_requeue {
                return Ok(REQUEUE_NORMAL);
            }
        },
        OPERATION_TYPE_CONFIG =>{
            debug!("start config");
            let limit = operator_controller.check_config(&namespace, os_cr.spec.maxunavailable.min(node_num)).await?;
            debug!("limit: {}", limit);
            let sys_configs = os_cr
                .spec
                .clone()
                .sysconfigs
                .ok_or(Error::MissingSubResource { value: String::from("os.spec.sysconfigs") })?;
            let version = sys_configs
                .clone()
                .version
                .ok_or(Error::MissingSubResource { value: String::from("os.spec.sysconfigs.version") })?;
            let need_requeue = operator_controller.assign_config(os_cr, sys_configs, version, limit, &namespace).await?;
            if need_requeue {
                return Ok(REQUEUE_NORMAL);
            }
        },
        _ =>{
            log::error!("operation {} cannot be recognized", ops);
        }
    }
    return Ok(REQUEUE_NORMAL);
}

pub fn error_policy<T: ApplyApi>(
    error: &Error,
    _ctx: Context<OperatorController<T>>,
) -> ReconcilerAction {
    error!("Reconciliation error: {}", error.to_string());
    REQUEUE_ERROR
}

pub mod reconciler_error {
    use thiserror::Error;
    use common::apiclient::apiclient_error;
    #[derive(Error, Debug)]
    pub enum Error {
        #[error("Kubernetes reported error: {source}")]
        KubeClient {
            #[from]
            source: kube::Error,
        },

        #[error("Create/Patch OSInstance reported error: {source}")]
        ApplyApi {
            #[from]
            source: apiclient_error::Error,
        },

        #[error("Cannot get environment NODE_NAME, error: {source}")]
        Env {
            #[from]
            source: std::env::VarError,
        },

        #[error("{}.metadata.{} is not exist", resource, value)]
        MissingObjectKey { resource: String, value: String },
        
        #[error("Cannot get {}, {} is None", value, value)]
        MissingSubResource { value: String },

        #[error("operation {} cannot be recognized", value)]
        Operation { value: String },
        
        #[error("Error when {:?} converting data to i64", data)]
        ConvertI64 { data: String },

        #[error("Error when {:?} converting data to u32", data)]
        ConvertU32 { data: String },
        
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Borrow;
    use super::{reconcile, reconciler_error::Error, Context, OSInstance, OperatorController, OS};
    use crate::controller::apiserver_mock::{timeout_after_5s, K8sResources, Testcases};
    use common::apiclient::ControllerClient;

    #[tokio::test]
    async fn test_rollback() {
        let (test_operator_controller, fakeserver) = OperatorController::<ControllerClient>::test();
        let os = OS::set_os_rollback_osversion_v1_upgradecon_v1();
        let context = Context::new(test_operator_controller);
        let mocksrv = fakeserver
            .run(Testcases::Rollback(K8sResources::set_rollback_nodes_v2_and_osi_v1()));
        reconcile(os, context.clone()).await.expect("reconciler");
        timeout_after_5s(mocksrv).await;
    }

    #[tokio::test]
    async fn test_config_normal() {
        let (test_operator_controller, fakeserver) = OperatorController::<ControllerClient>::test();
        let os = OS::set_os_syscon_v2_opstype_config_operator();
        let context = Context::new(test_operator_controller);
        let mocksrv = fakeserver
            .run(Testcases::ConfigNormal(K8sResources::set_nodes_v1_and_osi_v1()));
        reconcile(os, context.clone()).await.expect("reconciler");
        timeout_after_5s(mocksrv).await;
    }

    #[tokio::test]
    async fn test_skip_no_osi_node() {
        let (test_operator_controller, fakeserver) = OperatorController::<ControllerClient>::test();
        let os = OS::set_os_skip_osversion_v2_upgradecon_v1();
        let context = Context::new(test_operator_controller);
        let mocksrv = fakeserver
            .run(Testcases::SkipNoOsiNode(K8sResources::set_skip_nodes_and_osi()));
        reconcile(os, context.clone()).await.expect("reconciler");
        timeout_after_5s(mocksrv).await;
    }

    #[tokio::test]
    async fn test_exchange_current_and_next() {
        let (test_operator_controller, fakeserver) = OperatorController::<ControllerClient>::test();
        let os = OS::set_os_exchange_current_and_next();
        let context = Context::new(test_operator_controller);
        let mocksrv = fakeserver
            .run(Testcases::ExchangeCurrentAndNext(K8sResources::set_nodes_v1_and_osi_v1()));
        reconcile(os, context.clone()).await.expect("reconciler");
        timeout_after_5s(mocksrv).await;
    }

    #[tokio::test]
    async fn test_deep_copy_spec_configs() {
        let (test_operator_controller, _fakeserver) = OperatorController::<ControllerClient>::test();
        let deep_copy_result = test_operator_controller.clone().deep_copy_spec_configs(&OS::set_os_default(), &mut OSInstance::set_osi_default("", ""), "test".to_string()).await;
        assert!(deep_copy_result.is_err());
        if let Err(err) = deep_copy_result {
            assert_eq!("operation test cannot be recognized".to_string(), err.borrow().to_string());
        }
    }

    #[tokio::test]
    async fn test_get_config_osinstances() {
        let (test_operator_controller, fakeserver) = OperatorController::<ControllerClient>::test();
        let expected_error = "list error".to_string();
        fakeserver.test_function(Testcases::GetConfigOSInstances(expected_error.clone()));
        // perform the test
        let result = test_operator_controller.get_config_osinstances("default").await;
        // verify the return value
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                Error::KubeClient { source } => {
                    match source {
                        kube::Error::Api(error_response) => {
                            assert_eq!(expected_error, error_response.message);
                        },
                        _ => {
                            assert!(false);
                        }
                    }
                }
                _ => {
                    assert!(false);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_check_upgrading() {
        let (test_operator_controller, fakeserver) = OperatorController::<ControllerClient>::test();
        fakeserver.test_function(Testcases::CheckUpgrading("label error".to_string()));
        // perform the test
        let result = test_operator_controller.check_upgrading("default", 2).await;
        // verify the return value
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                Error::KubeClient { source } => {
                    match source {
                        kube::Error::Api(error_response) => {
                            assert_eq!("label error", error_response.message);
                        },
                        _ => {
                            assert!(false);
                        }
                    }
                }
                _ => {
                    assert!(false);
                }
            }
        }
    }


    #[tokio::test]
    async fn test_get_idle_osinstances() {
        let (test_operator_controller, fakeserver) = OperatorController::<ControllerClient>::test();
        let expected_error = "list error".to_string();
        fakeserver.test_function(Testcases::GetIdleOSInstances(expected_error.clone()));
        // perform the test
        let result = test_operator_controller.get_idle_os_instances("default", 3).await;
        // verify the return value
        assert!(result.is_err());
        if let Err(err) = result {
            match err {
                Error::KubeClient { source } => {
                    match source {
                        kube::Error::Api(error_response) => {
                            assert_eq!(expected_error, error_response.message);
                        },
                        _ => {
                            assert!(false);
                        }
                    }
                }
                _ => {
                    assert!(false);
                }
            }
        }
    }
}
