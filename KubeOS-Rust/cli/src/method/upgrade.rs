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
use serde_json::value::RawValue;

use crate::method::callable_method::RpcMethod;

#[derive(Default)]
pub struct UpgradeMethod {}

impl RpcMethod for UpgradeMethod {
    type Response = api::Response;
    fn command_name(&self) -> &'static str {
        "upgrade"
    }
    fn command_params(&self) -> Vec<Box<RawValue>> {
        vec![]
    }
}
