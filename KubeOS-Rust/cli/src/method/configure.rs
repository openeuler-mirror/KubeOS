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

use kubeos_manager::api;
use serde_json::value::{to_raw_value, RawValue};

use crate::method::callable_method::RpcMethod;

pub struct ConfigureMethod {
    req: api::ConfigureRequest,
}

impl ConfigureMethod {
    pub fn new(req: api::ConfigureRequest) -> Self {
        ConfigureMethod { req }
    }

    pub fn set_configure_request(&mut self, req: api::ConfigureRequest) -> &Self {
        self.req = req;
        self
    }
}

impl RpcMethod for ConfigureMethod {
    type Response = api::Response;
    fn command_name(&self) -> &'static str {
        "configure"
    }
    fn command_params(&self) -> Vec<Box<RawValue>> {
        vec![to_raw_value(&self.req).unwrap()]
    }
}
