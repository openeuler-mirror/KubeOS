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
use regex::Regex;
use anyhow::Result;
use http::{Request, Response};
use hyper::{body::to_bytes, Body};
use k8s_openapi::api::core::v1::{Node, NodeStatus, NodeSystemInfo};
use kube::{
    api::ObjectMeta,
    core::{ErrorResponse, ListMeta, ObjectList},
    Client as KubeClient, ResourceExt,
};
use serde_json::json;

use self::mock_error::Error;
use common::{
    apiclient::{ApplyApi, ControllerClient},
    crd::{Configs, Config, Content, OSInstance, OSInstanceSpec},
    values::{NODE_STATUS_CONFIG, NODE_STATUS_UPGRADE, LABEL_MASTER, LABEL_UPGRADING, NODE_STATUS_IDLE},
};
use crate::controller::OperatorController;

type ApiServerHandle = tower_test::mock::Handle<Request<Body>, Response<Body>>;
pub struct ApiServerVerifier(ApiServerHandle);

#[derive(Clone, Debug, Default)]
pub struct K8sResources{
    pub node_list: Vec<Node>,
    pub osi_list: Vec<OSInstance>,
}

pub enum Testcases {
    Rollback(K8sResources),
    ConfigNormal(K8sResources),
    SkipNoOsiNode(K8sResources),
    ExchangeCurrentAndNext(K8sResources),
    GetConfigOSInstances(String),
    CheckUpgrading(String),
    GetIdleOSInstances(String),
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
                Testcases::Rollback(k8s_resc) => {
                    self.handler_worker_node_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_upgrading_node_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_worker_and_no_upgrade_noding_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    // repeat twice for the OSI upgrade on both nodes
                    .handler_osinstance_get_by_node_name(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_osinstance_patch_nodestatus_upgrade(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_replace_node_by_name(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_osinstance_get_by_node_name(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_osinstance_patch_nodestatus_upgrade(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_replace_node_by_name(k8s_resc.clone())
                    .await
                },
                Testcases::ConfigNormal(k8s_resc) => {
                    self.handler_worker_node_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_config_osi_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_idle_osi_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_osinstance_patch_spec_config(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_osinstance_patch_spec_config(k8s_resc.clone())
                    .await
                },
                Testcases::SkipNoOsiNode(k8s_resc) => {
                    self.handler_worker_node_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_upgrading_node_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_worker_and_no_upgrade_noding_list_get(k8s_resc.clone())
                    .await
                },
                Testcases::ExchangeCurrentAndNext(k8s_resc) => {
                    self.handler_worker_node_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_upgrading_node_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_worker_and_no_upgrade_noding_list_get(k8s_resc.clone())
                    .await
                    .unwrap()
                    // repeat twice for the OSI upgrade on both nodes
                    .handler_osinstance_get_by_node_name(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_osinstance_patch_nodestatus_exchange(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_replace_node_by_name(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_osinstance_get_by_node_name(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_osinstance_patch_nodestatus_exchange(k8s_resc.clone())
                    .await
                    .unwrap()
                    .handler_replace_node_by_name(k8s_resc.clone())
                    .await
                },
                _ => {
                    Err(Error::ArgumentError)
                }
            }
            .expect("Case completed without errors");
        })
    }

    pub fn test_function(self, cases: Testcases) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            match cases {
                Testcases::GetConfigOSInstances(error) => {
                    self.handler_config_osi_list_get_error(error)
                    .await
                },
                Testcases::CheckUpgrading(error) => {
                    self.handler_upgrading_node_list_get_error(error)
                    .await
                },
                Testcases::GetIdleOSInstances(error) => {
                    self.handler_idle_osi_list_get_error(error)
                    .await
                },
                _ => {
                    Err(Error::ArgumentError)
                }
            }
            .expect("Case completed without errors");
        })
    }

    // get all the worker nodes, corresponding to the first get_nodes function of reconcile
    async fn handler_worker_node_list_get(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            "/api/v1/nodes?&labelSelector=%21node-role.kubernetes.io%2Fcontrol-plane&limit=0");
        assert_eq!(request.extensions().get(), Some(&"list"));
        let mut nodes = vec![];
        for node in k8s_resc.node_list.clone() {
            if !node.labels().contains_key(LABEL_MASTER){
                nodes.push(node.clone());
            }
        }
        let node_list: ObjectList<Node> = ObjectList {
            metadata: ListMeta {
                ..Default::default()
            },
            items: nodes,
        };
        dbg!("handler_worker_node_list_get");
        let response = serde_json::to_vec(&node_list).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    // Get all nodes in the environment that are labeled upgrading
    async fn handler_upgrading_node_list_get(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            "/api/v1/nodes?&labelSelector=upgrade.openeuler.org%2Fupgrading&limit=0");
        assert_eq!(request.extensions().get(), Some(&"list"));
        let mut nodes = vec![];
        for node in k8s_resc.node_list.clone() {
            if node.labels().contains_key(LABEL_UPGRADING){
                nodes.push(node.clone());
            }
        }
        let node_list: ObjectList<Node> = ObjectList {
            metadata: ListMeta {
                ..Default::default()
            },
            items: nodes,
        };
        dbg!("handler_upgrading_node_list_get");
        let response = serde_json::to_vec(&node_list).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    // Get all non-upgrading worker nodes
    async fn handler_worker_and_no_upgrade_noding_list_get(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        let remove_limit = |input: &str| -> String {
            let re = Regex::new(r"limit=\d+").unwrap();
            re.replace_all(input, "").to_string()
        };
        assert_eq!(
            remove_limit(request.uri().to_string().as_str()),
            "/api/v1/nodes?&labelSelector=%21upgrade.openeuler.org%2Fupgrading%2C%21node-role.kubernetes.io%2Fcontrol-plane&");
        assert_eq!(request.extensions().get(), Some(&"list"));
        let mut nodes = vec![];
        for node in k8s_resc.node_list.clone() {
            if !node.labels().contains_key(LABEL_UPGRADING) && !node.labels().contains_key(LABEL_MASTER){
                nodes.push(node.clone());
            }
        }
        let node_list: ObjectList<Node> = ObjectList {
            metadata: ListMeta {
                ..Default::default()
            },
            items: nodes,
        };
        dbg!("handler_worker_and_no_upgrade_noding_list_get");
        let response = serde_json::to_vec(&node_list).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_osinstance_get_by_node_name(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        // get req_node_name from request uri, and match it from k8s_resc.node_list to get osi and send back
        let req_node_name = request.uri().path().split('/').last().unwrap().split('?').next().unwrap();
        let mut osinstance = OSInstance::set_osi_default("", "");
        let mut boolean_get_osi = false;
        for osi in k8s_resc.osi_list.clone() {
            if osi.name() == req_node_name {
                boolean_get_osi = true;
                osinstance = osi.clone();
                break;
            }
        }
        assert!(boolean_get_osi);
        println!("handler_osinstance_get_by_node_name: req_node_name: {:?}", req_node_name);
        let response = serde_json::to_vec(&osinstance).unwrap();
        dbg!("handler_osinstance_get_by_node_name");
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_osinstance_patch_nodestatus_upgrade(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PATCH);
        // get req_node_name from request uri, and match it from k8s_resc.node_list to get osi and send back
        let req_node_name = request.uri().path().split('/').last().unwrap().split('?').next().unwrap();
        let mut osinstance = OSInstance::set_osi_default("", "");
        let mut boolean_get_osi = false;
        for osi in k8s_resc.osi_list.clone() {
            if osi.name() == req_node_name {
                boolean_get_osi = true;
                osinstance = osi.clone();
                break;
            }
        }
        assert!(boolean_get_osi);
        println!("handler_osinstance_patch_nodestatus_upgrade: req_node_name: {:?}", req_node_name);
        let req_body = to_bytes(request.into_body()).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&req_body).expect("valid document from runtime");
        let spec_json = body_json.get("spec").expect("spec object").clone();
        let spec: OSInstanceSpec = serde_json::from_value(spec_json).expect("valid spec");
        assert_eq!(spec.nodestatus.clone(), NODE_STATUS_UPGRADE.to_string());
        dbg!("handler_osinstance_patch_nodestatus_upgrade");
        osinstance.spec.nodestatus = NODE_STATUS_UPGRADE.to_string();
        let response = serde_json::to_vec(&osinstance).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_osinstance_patch_nodestatus_exchange(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PATCH);
        // get req_node_name from request uri, and match it from k8s_resc.node_list to get osi and send back
        let req_node_name = request.uri().path().split('/').last().unwrap().split('?').next().unwrap();
        let mut osinstance = OSInstance::set_osi_default("", "");
        let mut boolean_get_osi = false;
        for osi in k8s_resc.osi_list.clone() {
            if osi.name() == req_node_name {
                boolean_get_osi = true;
                osinstance = osi.clone();
                break;
            }
        }
        assert!(boolean_get_osi);
        println!("handler_osinstance_patch_nodestatus_exchange: req_node_name: {:?}", req_node_name);
        let req_body = to_bytes(request.into_body()).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&req_body).expect("valid document from runtime");
        let spec_json = body_json.get("spec").expect("spec object").clone();
        let spec: OSInstanceSpec = serde_json::from_value(spec_json).expect("valid spec");
        let sysconfigs = Some(
            Configs{
                version: Some(String::from("v2")),
                configs: Some(vec![
                    Config {
                        model: Some(String::from("grub.cmdline.next")),
                        configpath: Some(String::from("")),
                        contents: Some(vec![
                            Content {
                                key: Some(String::from("a")),
                                value: Some(String::from("1")),
                                operation: Some(String::from("")),
                            }
                        ]),
                    },
                    Config {
                        model: Some(String::from("grub.cmdline.current")),
                        configpath: Some(String::from("")),
                        contents: Some(vec![
                            Content {
                                key: Some(String::from("b")),
                                value: Some(String::from("2")),
                                operation: Some(String::from("")),
                            }
                        ]),
                    },
                ]),
            }
        );
        let upgradeconfigs = Some(
            Configs{
                version: Some(String::from("v2")),
                configs: Some(vec![
                    Config {
                        model: Some(String::from("grub.cmdline.current")),
                        configpath: Some(String::from("")),
                        contents: Some(vec![
                            Content {
                                key: Some(String::from("a")),
                                value: Some(String::from("1")),
                                operation: Some(String::from("")),
                            }
                        ]),
                    },
                    Config {
                        model: Some(String::from("grub.cmdline.next")),
                        configpath: Some(String::from("")),
                        contents: Some(vec![
                            Content {
                                key: Some(String::from("b")),
                                value: Some(String::from("2")),
                                operation: Some(String::from("")),
                            }
                        ]),
                    },
                ]),
            }
        );
        assert_eq!(spec.sysconfigs.clone(), sysconfigs);
        assert_eq!(spec.upgradeconfigs.clone(), upgradeconfigs);
        assert_eq!(spec.nodestatus.clone(), NODE_STATUS_UPGRADE.to_string());
        dbg!("handler_osinstance_patch_nodestatus_exchange");
        osinstance.spec.nodestatus = NODE_STATUS_UPGRADE.to_string();
        let response = serde_json::to_vec(&osinstance).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }    

    // obtain the node by its name
    async fn handler_replace_node_by_name(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PUT);
        
        // get req_node_name from request uri, and match it from k8s_resc.node_list to get node and send back
        let req_node_name = request.uri().path().split('/').last().unwrap().split('?').next().unwrap();
        let mut node = Node{..Default::default()};
        let mut boolean_get_node = false;
        for node_iter in k8s_resc.node_list.clone() {
            if node_iter.name() == req_node_name {
                boolean_get_node = true;
                node = node_iter.clone();
                break;
            }
        }
        assert!(boolean_get_node);
        assert_eq!(request.extensions().get(), Some(&"replace"));
        println!("handler_replace_node_by_name: req_node_name: {:?}", req_node_name);
        let req_body = to_bytes(request.into_body()).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&req_body).expect("valid document from runtime");
        let metadata_json = body_json.get("metadata").expect("metadata object").clone();
        let metadata: ObjectMeta = serde_json::from_value(metadata_json).expect("valid metadata");
        assert!(metadata.labels.unwrap().contains_key(LABEL_UPGRADING));
        node.labels_mut().insert(LABEL_UPGRADING.to_string(), "".to_string());
        dbg!("handler_replace_node_by_name");
        let response = serde_json::to_vec(&node).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    // get the OSI on all nodes in the environment that are labeled config
    async fn handler_config_osi_list_get(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            "/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances?");
        assert_eq!(request.extensions().get(), Some(&"list"));

        let mut osis = vec![];
        for osi in k8s_resc.osi_list.clone() {
            if osi.spec.nodestatus == NODE_STATUS_CONFIG{
                osis.push(osi.clone());
            }
        }
        let node_list: ObjectList<OSInstance> = ObjectList {
            metadata: ListMeta {
                ..Default::default()
            },
            items: osis,
        };
        dbg!("handler_config_osi_list_get");
        let response = serde_json::to_vec(&node_list).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    // get the OSI on all nodes in the environment that are labeled idle
    async fn handler_idle_osi_list_get(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            "/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances?&limit=3");
        assert_eq!(request.extensions().get(), Some(&"list"));

        let mut osis = vec![];
        for osi in k8s_resc.osi_list.clone() {
            if osi.spec.nodestatus == NODE_STATUS_IDLE{
                osis.push(osi.clone());
            }
        }
        let node_list: ObjectList<OSInstance> = ObjectList {
            metadata: ListMeta {
                ..Default::default()
            },
            items: osis,
        };
        dbg!("handler_idle_osi_list_get");
        let response = serde_json::to_vec(&node_list).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_osinstance_patch_spec_config(mut self, k8s_resc: K8sResources) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::PATCH);

        // get req_node_name from request uri, and match it from k8s_resc.node_list to get osi and send back
        let req_osi_name = request.uri().path().split('/').last().unwrap().split('?').next().unwrap();
        let mut osinstance = OSInstance::set_osi_default("", "");
        let mut boolean_get_osi = false;
        for osi in k8s_resc.osi_list.clone() {
            if osi.name() == req_osi_name {
                boolean_get_osi = true;
                osinstance = osi.clone();
                break;
            }
        }
        assert!(boolean_get_osi);
        println!("handler_osinstance_patch_spec_config: req_osi_name: {:?}", req_osi_name);
        let req_body = to_bytes(request.into_body()).await.unwrap();
        let body_json: serde_json::Value = serde_json::from_slice(&req_body).expect("valid document from runtime");
        let spec_json = body_json.get("spec").expect("spec object").clone();
        let spec: OSInstanceSpec = serde_json::from_value(spec_json).expect("valid spec");
        assert_eq!(spec.nodestatus.clone(), NODE_STATUS_CONFIG.to_string());
        
        let sysconfig = Some(
            Configs {
                version: Some(String::from("v2")),
                configs: Some(vec![Config {
                    model: Some(String::from("kernel.sysctl")),
                    configpath: Some(String::from("")),
                    contents: 
                        Some(vec![
                            Content {
                                key: Some(String::from("key1")),
                                value: Some(String::from("a")),
                                operation: Some(String::from("")),
                            }, 
                            Content {
                                key: Some(String::from("key2")),
                                value: Some(String::from("b")),
                                operation: Some(String::from("")),
                            },
                        ]),
                }]),
            }
        );
        assert_eq!(
            spec.sysconfigs.clone(), 
            sysconfig
        );
        dbg!("handler_osinstance_patch_spec_config");
        osinstance.spec.nodestatus = NODE_STATUS_CONFIG.to_string();
        osinstance.spec.sysconfigs = sysconfig;
        let response = serde_json::to_vec(&osinstance).unwrap();
        send.send_response(Response::builder().body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_config_osi_list_get_error(mut self, error: String) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            "/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances?");
        assert_eq!(request.extensions().get(), Some(&"list"));
        dbg!("handler_config_osi_list_get_error");
        
        let error_response = ErrorResponse {
            status: "Failure".to_string(),
            message: error,
            reason: "NotFound".to_string(),
            code: 404,
        };
        let response_body = json!({
            "status": error_response.status,
            "message": error_response.message,
            "reason": error_response.reason,
            "code": error_response.code,
        });
        let response = serde_json::to_vec(&response_body).unwrap();
        send.send_response(Response::builder().status(404).body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_upgrading_node_list_get_error(mut self, error: String) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            "/api/v1/nodes?&labelSelector=upgrade.openeuler.org%2Fupgrading&limit=0");
        assert_eq!(request.extensions().get(), Some(&"list"));
        dbg!("handler_upgrading_node_list_get_error");
        
        let error_response = ErrorResponse {
            status: "Failure".to_string(),
            message: error,
            reason: "Invalid".to_string(),
            code: 400,
        };
        let response_body = json!({
            "status": error_response.status,
            "message": error_response.message,
            "reason": error_response.reason,
            "code": error_response.code,
        });
        let response = serde_json::to_vec(&response_body).unwrap();
        send.send_response(Response::builder().status(400).body(Body::from(response)).unwrap());
        Ok(self)
    }

    async fn handler_idle_osi_list_get_error(mut self, error: String) -> Result<Self, Error> {
        let (request, send) = self.0.next_request().await.expect("service not called");
        assert_eq!(request.method(), http::Method::GET);
        assert_eq!(
            request.uri().to_string(),
            "/apis/upgrade.openeuler.org/v1alpha1/namespaces/default/osinstances?&limit=3");
        assert_eq!(request.extensions().get(), Some(&"list"));
        dbg!("handler_idle_osi_list_get_error");
        
        let error_response = ErrorResponse {
            status: "Failure".to_string(),
            message: error,
            reason: "NotFound".to_string(),
            code: 404,
        };
        let response_body = json!({
            "status": error_response.status,
            "message": error_response.message,
            "reason": error_response.reason,
            "code": error_response.code,
        });
        let response = serde_json::to_vec(&response_body).unwrap();
        send.send_response(Response::builder().status(404).body(Body::from(response)).unwrap());
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

        #[error("Parameters other than expected were entered")]
        ArgumentError,
    }
}

impl<T: ApplyApi> OperatorController<T> {
    pub fn test() -> (OperatorController<ControllerClient>, ApiServerVerifier) {
        let (mock_service, handle) = tower_test::mock::pair::<Request<Body>, Response<Body>>();
        let mock_k8s_client = KubeClient::new(mock_service, "default");
        let mock_api_client = ControllerClient::new(mock_k8s_client.clone());
        let operator_controller: OperatorController<ControllerClient> =
            OperatorController::new(mock_k8s_client, mock_api_client);
        (operator_controller, ApiServerVerifier(handle))
    }
}

impl K8sResources {
    pub fn set_rollback_nodes_v2_and_osi_v1() -> Self {
        let node1 = Node {
            metadata: ObjectMeta {
                name: Some("openeuler-node1".into()),
                labels: Some(BTreeMap::from([("beta.kubernetes.io/os".into(), "linux".into())])),
                ..Default::default()
            },
            spec: None,
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo {
                    os_image: "KubeOS v2".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let node2 = Node {
            metadata: ObjectMeta {
                name: Some("openeuler-node2".into()),
                labels: Some(BTreeMap::from([("beta.kubernetes.io/os".into(), "linux".into())])),
                ..Default::default()
            },
            spec: None,
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo {
                    os_image: "KubeOS v2".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let osi1 = OSInstance::set_osi_default(&node1.name().clone(), "default");
        let osi2 = OSInstance::set_osi_default(&node2.name().clone(), "default");
        let node_list = Vec::from([node1, node2]);
        let osi_list = Vec::from([osi1, osi2]);
        K8sResources{
            node_list,
            osi_list
        }
    }

    pub fn set_nodes_v1_and_osi_v1() -> Self {
        let node1 = Node {
            metadata: ObjectMeta {
                name: Some("openeuler-node1".into()),
                labels: Some(BTreeMap::from([("beta.kubernetes.io/os".into(), "linux".into())])),
                ..Default::default()
            },
            spec: None,
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo {
                    os_image: "KubeOS v1".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let node2 = Node {
            metadata: ObjectMeta {
                name: Some("openeuler-node2".into()),
                labels: Some(BTreeMap::from([("beta.kubernetes.io/os".into(), "linux".into())])),
                ..Default::default()
            },
            spec: None,
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo {
                    os_image: "KubeOS v1".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let osi1 = OSInstance::set_osi_default(&node1.name().clone(), "default");
        let osi2 = OSInstance::set_osi_default(&node2.name().clone(), "default");
        let node_list = Vec::from([node1, node2]);
        let osi_list = Vec::from([osi1, osi2]);
        K8sResources{
            node_list,
            osi_list
        }
    }

    pub fn set_skip_nodes_and_osi() -> Self {
        // create node1 and don't set osi
        let node1 = Node {
            metadata: ObjectMeta {
                name: Some("openeuler-node1".into()),
                labels: Some(BTreeMap::from([("beta.kubernetes.io/os".into(), "linux".into())])),
                ..Default::default()
            },
            spec: None,
            status: Some(NodeStatus {
                node_info: Some(NodeSystemInfo {
                    os_image: "KubeOS v1".into(),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let node_list = Vec::from([node1]);
        let osi_list = Vec::new();
        K8sResources{
            node_list,
            osi_list
        }
    }
}
