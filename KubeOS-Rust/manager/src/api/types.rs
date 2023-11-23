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
    sys_mgmt::CtrImageHandler,
    utils::{CommandExecutor, UpgradeImageManager},
};

#[derive(Deserialize, Serialize, Debug)]
pub struct UpgradeRequest {
    pub version: String,
    pub check_sum: String,
    pub image_type: String,
    pub container_image: String,
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
}

impl<T: CommandExecutor> ImageType<T> {
    pub fn download_image(&self, req: &UpgradeRequest) -> anyhow::Result<UpgradeImageManager<T>> {
        match self {
            ImageType::Containerd(handler) => handler.download_image(req),
        }
    }
}
pub trait ImageHandler<T: CommandExecutor> {
    fn download_image(&self, req: &UpgradeRequest) -> anyhow::Result<UpgradeImageManager<T>>;
}
