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

use super::crd::{OSInstance, OSInstanceSpec, OSInstanceStatus};
use super::values::{LABEL_OSINSTANCE, NODE_STATUS_IDLE, OSINSTANCE_API_VERSION, OSINSTANCE_KIND};
use anyhow::Result;
use apiclient_error::Error;
use async_trait::async_trait;
use kube::{
    api::{Api, ObjectMeta, Patch, PatchParams, PostParams},
    Client,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize)]
struct OSInstanceSpecPatch {
    #[serde(rename = "apiVersion")]
    api_version: String,
    kind: String,
    spec: OSInstanceSpec,
}

impl Default for OSInstanceSpecPatch {
    fn default() -> Self {
        OSInstanceSpecPatch {
            api_version: OSINSTANCE_API_VERSION.to_string(),
            kind: OSINSTANCE_KIND.to_string(),
            spec: OSInstanceSpec {
                nodestatus: NODE_STATUS_IDLE.to_string(),
                sysconfigs: None,
                upgradeconfigs: None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OSInstanceStatusPatch {
    #[serde(rename = "apiVersion")]
    api_version: String,
    kind: String,
    status: Option<OSInstanceStatus>,
}

impl Default for OSInstanceStatusPatch {
    fn default() -> Self {
        OSInstanceStatusPatch {
            api_version: OSINSTANCE_API_VERSION.to_string(),
            kind: OSINSTANCE_KIND.to_string(),
            status: Some(OSInstanceStatus {
                sysconfigs: None,
                upgradeconfigs: None,
            }),
        }
    }
}

#[derive(Clone)]
pub struct ControllerClient {
    pub client: Client,
}

impl ControllerClient {
    pub fn new(client: Client) -> Self {
        ControllerClient { client }
    }
}

#[async_trait]
pub trait ApplyApi: Clone + Sized + Send + Sync {
    async fn create_osinstance(&self, node_name: &str, namespace: &str) -> Result<(), Error>;
    async fn update_osinstance_spec(
        &self,
        node_name: &str,
        namespace: &str,
        spec: &OSInstanceSpec,
    ) -> Result<(), Error>;
    async fn update_osinstance_status(
        &self,
        node_name: &str,
        namespace: &str,
        status: &Option<OSInstanceStatus>,
    ) -> Result<(), Error>;
}

#[async_trait]
impl ApplyApi for ControllerClient {
    async fn create_osinstance(&self, node_name: &str, namespace: &str) -> Result<(), Error> {
        let mut labels = BTreeMap::new();
        labels.insert(LABEL_OSINSTANCE.to_string(), node_name.to_string());
        let osinstance = OSInstance {
            metadata: ObjectMeta {
                name: Some(node_name.to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some(labels),
                ..ObjectMeta::default()
            },
            spec: OSInstanceSpec {
                nodestatus: NODE_STATUS_IDLE.to_string(),
                sysconfigs: None,
                upgradeconfigs: None,
            },
            status: None,
        };
        let osi_api = Api::namespaced(self.client.clone(), namespace);
        osi_api.create(&PostParams::default(), &osinstance).await?;
        Ok(())
    }

    async fn update_osinstance_spec(
        &self,
        node_name: &str,
        namespace: &str,
        spec: &OSInstanceSpec,
    ) -> Result<(), Error> {
        let osi_api: Api<OSInstance> = Api::namespaced(self.client.clone(), namespace);
        let osi_spec_patch = OSInstanceSpecPatch {
            spec: spec.clone(),
            ..Default::default()
        };
        osi_api
            .patch(
                node_name,
                &PatchParams::default(),
                &Patch::Merge(&osi_spec_patch),
            )
            .await?;
        Ok(())
    }

    async fn update_osinstance_status(
        &self,
        node_name: &str,
        namespace: &str,
        status: &Option<OSInstanceStatus>,
    ) -> Result<(), Error> {
        let osi_api: Api<OSInstance> = Api::namespaced(self.client.clone(), namespace);
        let osi_status_patch = OSInstanceStatusPatch {
            status: status.clone(),
            ..Default::default()
        };
        osi_api
            .patch_status(
                node_name,
                &PatchParams::default(),
                &Patch::Merge(&osi_status_patch),
            )
            .await?;
        Ok(())
    }
}
pub mod apiclient_error {
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
