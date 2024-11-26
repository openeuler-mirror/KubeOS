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

use std::collections::BTreeMap;

use anyhow::Result;
use cli::{
    client::Client,
    method::{
        callable_method::RpcMethod, configure::ConfigureMethod, prepare_upgrade::PrepareUpgradeMethod,
        rollback::RollbackMethod, upgrade::UpgradeMethod,
    },
};
use http::{Request, Response};
use hyper::{body::to_bytes, Body};
use k8s_openapi::api::core::v1::{Node, NodeSpec, NodeStatus, NodeSystemInfo, Pod};
use kube::{
    api::ObjectMeta,
    core::{ListMeta, ObjectList},
    Client as KubeClient, Resource, ResourceExt,
};
use serde_json::json;
use mockall::mock;

use self::mock_error::Error;
use super::{
    agentclient::*,
    crd::{Configs, NamespacedName, OSInstanceStatus},
    values::{NODE_STATUS_CONFIG, NODE_STATUS_UPGRADE, OPERATION_TYPE_ROLLBACK},
};
use crate::controller::{
    apiclient::{ApplyApi, ControllerClient},
    crd::{Config, Content, OSInstance, OSInstanceSpec, OSSpec, OS},
    values::{LABEL_OSINSTANCE, LABEL_UPGRADING, NODE_STATUS_IDLE},
    ProxyController,
};

type ApiServerHandle = tower_test::mock::Handle<Request<Body>, Response<Body>>;
pub struct ApiServerVerifier(ApiServerHandle);

pub enum Testcases {
    OSInstanceNotExist(OSInstance),
    UpgradeNormal(OSInstance),
    UpgradeUpgradeconfigsVersionMismatch(OSInstance),
    UpgradeOSInstaceNodestatusConfig(OSInstance),
    UpgradeOSInstaceNodestatusIdle(OSInstance),
    ConfigNormal(OSInstance),
    ConfigVersionMismatchReassign(OSInstance),
    ConfigVersionMismatchUpdate(OSInstance),
    Rollback(OSInstance),
}

pub async fn timeout_after_5s(handle: tokio::task::JoinHandle<()>) {
    tokio::time::timeout(std::time::Duration::from_secs(5), handle)
        .await
        .expect("timeout on mock apiserver")
        .expect("scenario succeeded")
}

impl ApiServerVerifier {
    pub fn run(self, cases: Testcases) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            match cases {
                Testcases::OSInstanceNotExist(osi) => {
                    self.handler_osinstance_get_not_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_creation(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get(osi)
                        .await
                },
                Testcases::UpgradeNormal(osi) => {
                    self.handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get_with_label(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_patch_upgradeconfig_v2(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_cordon(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_pod_list_get(osi)
                        .await
                },
                Testcases::UpgradeUpgradeconfigsVersionMismatch(osi) => {
                    self.handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get_with_label(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_update_delete_label(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_uncordon(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_patch_nodestatus_idle(osi)
                        .await
                },
                Testcases::UpgradeOSInstaceNodestatusConfig(osi) => {
                    self.handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get_with_label(osi.clone())
                        .await
                },
                Testcases::UpgradeOSInstaceNodestatusIdle(osi) => {
                    self.handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get_with_label(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_update_delete_label(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_uncordon(osi)
                        .await
                },
                Testcases::ConfigNormal(osi) => {
                    self.handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_patch_sysconfig_v2(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_patch_nodestatus_idle(osi)
                        .await
                },
                Testcases::ConfigVersionMismatchReassign(osi) => {
                    self.handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_patch_nodestatus_idle(osi)
                        .await
                },
                Testcases::ConfigVersionMismatchUpdate(osi) => {
                    self.handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_patch_spec_sysconfig_v2(osi)
                        .await
                },
                Testcases::Rollback(osi) => {
                    self.handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_get_exist(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_get_with_label(osi.clone())
                        .await
                        .unwrap()
                        .handler_osinstance_patch_upgradeconfig_v2(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_cordon(osi.clone())
                        .await
                        .unwrap()
                        .handler_node_pod_list_get(osi)
                        .await
                },
            }
            .expect("Case completed without errors");
        })
    }

    async fn handler_osinstance_get_not_exist(mut self, osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            format!("/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances/{}", osinstance.name())
        );
        let response_json = serde_json::json!(
            { "status": "Failure", "message": "osinstances.upgrade.openeuler.org \"openeuler\" not found", "reason": "NotFound", "code": 404 }
        );
        dbg!("handler_osinstance_get_not_exist");
        let response = serde_json::to_vec(&response_json).unwrap();
        send.send_response(Response::builder().status(404).body(Body::from(response)).unwrap());
        Ok(self)
    }
    async fn handler_osinstance_get_exist(mut self, osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            format!("/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances/{}", osinstance.name())
        );
        dbg!("handler_osinstance_get_exist");
        let response = serde_json::to_vec(&osinstance).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }
    async fn handler_osinstance_creation(mut self, osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::POST);
        assert_eq!(
            request.uri().to_string(),
            format!("/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances?")
        );
        dbg!("handler_osinstance_creation");
        let response = serde_json::to_vec(&osinstance).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_osinstance_patch_nodestatus_idle(mut self, mut osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PATCH);
        assert_eq!(
            request.uri().to_string(),
            format!("/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances/{}?", osinstance.name())
        );

        let req_body = to_bytes(request.into_body()).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&req_body).expect("valid document from runtime");
        let spec_json = body_json.get("spec").expect("spec object").clone();
        let spec: OSInstanceSpec = serde_json::from_value(spec_json).expect("valid spec");
        assert_eq!(spec.nodestatus.clone(), NODE_STATUS_IDLE.to_string());

        dbg!("handler_osinstance_patch_nodestatus_idle");
        osinstance.spec.nodestatus = NODE_STATUS_IDLE.to_string();
        let response = serde_json::to_vec(&osinstance).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_osinstance_patch_upgradeconfig_v2(mut self, mut osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PATCH);
        assert_eq!(
            request.uri().to_string(),
            format!(
                "/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances/{}/status?",
                osinstance.name()
            )
        );

        let req_body = to_bytes(request.into_body()).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&req_body).expect("valid document from runtime");
        let status_json = body_json.get("status").expect("status object").clone();
        let status: OSInstanceStatus = serde_json::from_value(status_json).expect("valid status");

        assert_eq!(
            status.upgradeconfigs.expect("upgradeconfigs is not None").clone(),
            osinstance.spec.clone().upgradeconfigs.expect("upgradeconfig is not None")
        );

        osinstance.status.as_mut().unwrap().upgradeconfigs = osinstance.spec.upgradeconfigs.clone();

        dbg!("handler_osinstance_patch_upgradeconfig_v2");
        let response = serde_json::to_vec(&osinstance).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_osinstance_patch_sysconfig_v2(mut self, mut osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PATCH);
        assert_eq!(
            request.uri().to_string(),
            format!(
                "/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances/{}/status?",
                osinstance.name()
            )
        );

        let req_body = to_bytes(request.into_body()).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&req_body).expect("valid osinstance");
        let status_json = body_json.get("status").expect("status object").clone();
        let status: OSInstanceStatus = serde_json::from_value(status_json).expect("valid status");

        assert_eq!(
            status.sysconfigs.expect("sysconfigs is not None").clone(),
            osinstance.spec.clone().sysconfigs.expect("sysconfig is not None")
        );

        osinstance.status.as_mut().unwrap().sysconfigs = osinstance.spec.sysconfigs.clone();

        dbg!("handler_osinstance_patch_sysconfig_v2");
        let response = serde_json::to_vec(&osinstance).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_osinstance_patch_spec_sysconfig_v2(mut self, mut osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PATCH);
        assert_eq!(
            request.uri().to_string(),
            format!("/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances/{}?", osinstance.name())
        );

        let req_body = to_bytes(request.into_body()).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&req_body).expect("valid osinstance");
        let spec_json = body_json.get("spec").expect("spec object").clone();
        let spec: OSInstanceSpec = serde_json::from_value(spec_json).expect("valid spec");

        assert_eq!(
            spec.sysconfigs.expect("upgradeconfigs is not None").clone().version.clone().unwrap(),
            String::from("v2")
        );

        osinstance.spec.sysconfigs.as_mut().unwrap().version = Some(String::from("v2"));

        dbg!("handler_osinstance_patch_spec_sysconfig_v2");
        let response = serde_json::to_vec(&osinstance).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_node_get(mut self, osinstance: OSInstance) -> Result<Self, Error> {
        // return node with name = openeuler, osimage = KubeOS v1，no upgrade label
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(request.uri().to_string(), format!("/api/v1/nodes/{}", osinstance.name()));
        let node = Node {
            metadata: ObjectMeta { name: Some(String::from("openeuler")), ..Default::default() },
            spec: None,
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo { os_image: String::from("KubeOS v1"), ..Default::default() }),
                ..Default::default()
            }),
        };
        assert_eq!(node.name(), String::from("openeuler"));
        assert_eq!(node.status.as_ref().unwrap().node_info.as_ref().unwrap().os_image, String::from("KubeOS v1"));
        dbg!("handler_node_get");
        let response = serde_json::to_vec(&node.clone()).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_node_get_with_label(mut self, osinstance: OSInstance) -> Result<Self, Error> {
        // return node with name = openeuler, osimage = KubeOS v1，has upgrade label
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(request.uri().to_string(), format!("/api/v1/nodes/{}", osinstance.name()));
        let mut node = Node {
            metadata: ObjectMeta { name: Some(String::from("openeuler")), ..Default::default() },
            spec: None,
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo { os_image: String::from("KubeOS v1"), ..Default::default() }),
                ..Default::default()
            }),
        };
        let node_labels = node.labels_mut();
        node_labels.insert(LABEL_UPGRADING.to_string(), "".to_string());
        assert_eq!(node.name(), String::from("openeuler"));
        assert_eq!(node.status.as_ref().unwrap().node_info.as_ref().unwrap().os_image, String::from("KubeOS v1"));
        assert!(node.labels().contains_key(LABEL_UPGRADING));
        dbg!("handler_node_get_with_label");
        let response = serde_json::to_vec(&node.clone()).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_node_update_delete_label(mut self, osinstance: OSInstance) -> Result<Self, Error> {
        // return node with name = openeuler, osimage = KubeOS v1，no upgrade label
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PUT);
        assert_eq!(request.uri().to_string(), format!("/api/v1/nodes/{}?", osinstance.name()));
        // check request body has upgrade label
        let node = Node {
            metadata: ObjectMeta { name: Some(String::from("openeuler")), ..Default::default() },
            spec: Some(NodeSpec { unschedulable: Some(true), ..Default::default() }),
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo { os_image: String::from("KubeOS v1"), ..Default::default() }),
                ..Default::default()
            }),
        };
        dbg!("handler_node_update_delete_label");
        let response = serde_json::to_vec(&node.clone()).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_node_cordon(mut self, osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PATCH);
        assert_eq!(request.uri().to_string(), format!("/api/v1/nodes/{}?", osinstance.name()));
        assert_eq!(request.extensions().get(), Some(&"cordon"));
        let node = Node {
            metadata: ObjectMeta { name: Some(String::from("openeuler")), ..Default::default() },
            spec: Some(NodeSpec { unschedulable: Some(true), ..Default::default() }),
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo { os_image: String::from("KubeOS v1"), ..Default::default() }),
                ..Default::default()
            }),
        };
        dbg!("handler_node_cordon");
        let response = serde_json::to_vec(&node.clone()).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_node_uncordon(mut self, osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PATCH);
        assert_eq!(request.uri().to_string(), format!("/api/v1/nodes/{}?", osinstance.name()));
        assert_eq!(request.extensions().get(), Some(&"cordon"));
        let node = Node {
            metadata: ObjectMeta { name: Some(String::from("openeuler")), ..Default::default() },
            spec: Some(NodeSpec { unschedulable: Some(false), ..Default::default() }),
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo { os_image: String::from("KubeOS v1"), ..Default::default() }),
                ..Default::default()
            }),
        };
        dbg!("handler_node_uncordon");
        let response = serde_json::to_vec(&node.clone()).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_node_pod_list_get(mut self, osinstance: OSInstance) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            format!("/api/v1/pods?&fieldSelector=spec.nodeName%3D{}", osinstance.name())
        );
        assert_eq!(request.extensions().get(), Some(&"list"));
        let pods_list = ObjectList::<Pod> { metadata: ListMeta::default(), items: vec![] };
        dbg!("handler_node_pod_list_get");
        let response = serde_json::to_vec(&pods_list).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }
}

pub mod mock_error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum Error {
        #[error("Kubernetes reported error: {source}")]
        KubeError {
            #[from]
            source: kube::Error,
        },
    }
}

mock! {
    pub AgentCallClient{}
    impl AgentCall for AgentCallClient{
        fn call_agent<T: RpcMethod + 'static>(&self, client:&Client, method: T) -> Result<(), agent_error::Error> {
                Ok(())
            }
    }

}
impl<T: ApplyApi, U: AgentCall> ProxyController<T, U> {
    pub fn test() -> (ProxyController<ControllerClient, MockAgentCallClient>, ApiServerVerifier) {
        let (mock_service, handle) = tower_test::mock::pair::<Request<Body>, Response<Body>>();
        let mock_k8s_client = KubeClient::new(mock_service, "default");
        let mock_api_client = ControllerClient::new(mock_k8s_client.clone());
        let mut mock_agent_call_client = MockAgentCallClient::new();
        mock_agent_call_client.expect_call_agent::<UpgradeMethod>().returning(|_x, _y| Ok(()));
        mock_agent_call_client.expect_call_agent::<PrepareUpgradeMethod>().returning(|_x, _y| Ok(()));
        mock_agent_call_client.expect_call_agent::<RollbackMethod>().returning(|_x, _y| Ok(()));
        mock_agent_call_client.expect_call_agent::<ConfigureMethod>().returning(|_x, _y| Ok(()));
        let mock_agent_client = AgentClient::new("test", mock_agent_call_client);
        let proxy_controller: ProxyController<ControllerClient, MockAgentCallClient> =
            ProxyController::new(mock_k8s_client, mock_api_client, mock_agent_client);
        (proxy_controller, ApiServerVerifier(handle))
    }
}

impl OSInstance {
    pub fn set_osi_default(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = idle, upgradeconfig.version=v1, sysconfig.version=v1
        let mut labels = BTreeMap::new();
        labels.insert(LABEL_OSINSTANCE.to_string(), node_name.to_string());
        OSInstance {
            metadata: ObjectMeta {
                name: Some(node_name.to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some(labels),
                ..ObjectMeta::default()
            },
            spec: OSInstanceSpec {
                nodestatus: NODE_STATUS_IDLE.to_string(),
                sysconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
                upgradeconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
                namespacedname: Some(NamespacedName { namespace: String::from("default"), name: String::from("test") }),
            },
            status: Some(OSInstanceStatus {
                sysconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
                upgradeconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
            }),
        }
    }

    pub fn set_osi_nodestatus_upgrade(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = upgrade, upgradeconfig.version=v1, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.nodestatus = NODE_STATUS_UPGRADE.to_string();
        osinstance
    }

    pub fn set_osi_nodestatus_config(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = config, upgradeconfig.version=v1, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.nodestatus = NODE_STATUS_CONFIG.to_string();
        osinstance
    }

    pub fn set_osi_upgradecon_v2(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = idle, upgradeconfig.version=v1, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.upgradeconfigs.as_mut().unwrap().version = Some(String::from("v2"));
        osinstance
    }

    pub fn set_osi_nodestatus_upgrade_upgradecon_v2(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = upgrade, upgradeconfig.version=v2, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.nodestatus = NODE_STATUS_UPGRADE.to_string();
        osinstance.spec.upgradeconfigs = Some(Configs {
            version: Some(String::from("v2")),
            configs: Some(vec![Config {
                model: Some(String::from("kernel.sysctl.persist")),
                configpath: Some(String::from("/persist/persist.conf")),
                contents: Some(vec![Content {
                    key: Some(String::from("kernel.test")),
                    value: Some(serde_json::Value::from(json!("test"))),
                    operation: Some(String::from("delete")),
                }]),
            }]),
        });
        osinstance
    }

    pub fn set_osi_nodestatus_config_syscon_v2(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = upgrade, upgradeconfig.version=v2, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.nodestatus = NODE_STATUS_CONFIG.to_string();
        osinstance.spec.sysconfigs = Some(Configs {
            version: Some(String::from("v2")),
            configs: Some(vec![Config {
                model: Some(String::from("kernel.sysctl.persist")),
                configpath: Some(String::from("/persist/persist.conf")),
                contents: Some(vec![Content {
                    key: Some(String::from("kernel.test")),
                    value: Some(serde_json::Value::from(json!("test"))),
                    operation: Some(String::from("delete")),
                }]),
            }]),
        });
        osinstance
    }
}

impl OS {
    pub fn set_os_default() -> Self {
        let mut os = OS::new("test", OSSpec::default());
        os.meta_mut().namespace = Some("default".into());
        os
    }

    pub fn set_os_osversion_v2_opstype_config() -> Self {
        let mut os = OS::set_os_default();
        os.spec.osversion = String::from("KubeOS v2");
        os.spec.opstype = String::from("config");
        os
    }

    pub fn set_os_osversion_v2_upgradecon_v2() -> Self {
        let mut os = OS::set_os_default();
        os.spec.osversion = String::from("KubeOS v2");
        os.spec.upgradeconfigs = Some(Configs { version: Some(String::from("v2")), configs: None });
        os
    }

    pub fn set_os_syscon_v2_opstype_config() -> Self {
        let mut os = OS::set_os_default();
        os.spec.opstype = String::from("config");
        os.spec.sysconfigs = Some(Configs {
            version: Some(String::from("v2")),
            configs: Some(vec![Config {
                model: Some(String::from("kernel.sysctl.persist")),
                configpath: Some(String::from("/persist/persist.conf")),
                contents: Some(vec![Content {
                    key: Some(String::from("kernel.test")),
                    value: Some(serde_json::Value::from(json!("test"))),
                    operation: Some(String::from("delete")),
                }]),
            }]),
        });
        os
    }

    pub fn set_os_rollback_osversion_v2_upgradecon_v2() -> Self {
        let mut os = OS::set_os_default();
        os.spec.osversion = String::from("KubeOS v2");
        os.spec.opstype = OPERATION_TYPE_ROLLBACK.to_string();
        os.spec.upgradeconfigs = Some(Configs {
            version: Some(String::from("v2")),
            configs: Some(vec![Config {
                model: Some(String::from("kernel.sysctl.persist")),
                configpath: Some(String::from("/persist/persist.conf")),
                contents: Some(vec![Content {
                    key: Some(String::from("kernel.test")),
                    value: Some(serde_json::Value::from(json!("test"))),
                    operation: Some(String::from("delete")),
                }]),
            }]),
        });
        os
    }
}

impl Default for OSSpec {
    fn default() -> Self {
        OSSpec {
            osversion: String::from("KubeOS v1"),
            maxunavailable: 2,
            checksum: String::from("test"),
            imagetype: String::from("containerd"),
            containerimage: String::from("test"),
            opstype: String::from("upgrade"),
            evictpodforce: true,
            imageurl: String::from(""),
            flagsafe: false,
            mtls: false,
            cacert: Some(String::from("")),
            clientcert: Some(String::from("")),
            clientkey: Some(String::from("")),
            sysconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
            upgradeconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
            nodeselector: None,
            timeinterval: None,
            timewindow: None,
            executionmode: None,
        }
    }
}
