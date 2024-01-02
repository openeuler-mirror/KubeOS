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

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(group = "upgrade.openeuler.org", version = "v1alpha1", kind = "OS", plural = "os", singular = "os", namespaced)]
pub struct OSSpec {
    pub osversion: String,
    pub maxunavailable: i64,
    pub checksum: String,
    pub imagetype: String,
    pub containerimage: String,
    pub opstype: String,
    pub evictpodforce: bool,
    pub sysconfigs: Option<Configs>,
    pub upgradeconfigs: Option<Configs>,
}

#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "upgrade.openeuler.org",
    version = "v1alpha1",
    kind = "OSInstance",
    plural = "osinstances",
    singular = "osinstance",
    status = "OSInstanceStatus",
    namespaced
)]
pub struct OSInstanceSpec {
    pub nodestatus: String,
    pub sysconfigs: Option<Configs>,
    pub upgradeconfigs: Option<Configs>,
}

#[derive(Clone, Deserialize, Serialize, Debug, Eq, PartialEq, JsonSchema)]
pub struct OSInstanceStatus {
    pub sysconfigs: Option<Configs>,
    pub upgradeconfigs: Option<Configs>,
}

#[derive(Clone, Deserialize, Serialize, Debug, Eq, PartialEq, JsonSchema)]
pub struct Configs {
    pub version: Option<String>,
    pub configs: Option<Vec<Config>>,
}

#[derive(Clone, Deserialize, Serialize, Debug, Eq, PartialEq, JsonSchema)]
pub struct Config {
    pub model: Option<String>,
    pub configpath: Option<String>,
    pub contents: Option<Vec<Content>>,
}

#[derive(Clone, Deserialize, Serialize, Debug, Eq, PartialEq, JsonSchema)]
pub struct Content {
    pub key: Option<String>,
    pub value: Option<String>,
    pub operation: Option<String>,
}
