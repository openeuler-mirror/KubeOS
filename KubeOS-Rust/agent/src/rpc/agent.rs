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

use super::function::{rpc, RpcResult};
use manager::api::{ConfigureRequest, Response, UpgradeRequest};

#[rpc(server)]
pub trait Agent {
    #[rpc(name = "prepare_upgrade")]
    fn prepare_upgrade(&self, req: UpgradeRequest) -> RpcResult<Response>;

    #[rpc(name = "upgrade")]
    fn upgrade(&self) -> RpcResult<Response>;

    #[rpc(name = "cleanup")]
    fn cleanup(&self) -> RpcResult<Response>;

    #[rpc(name = "configure")]
    fn configure(&self, req: ConfigureRequest) -> RpcResult<Response>;

    #[rpc(name = "rollback")]
    fn rollback(&self) -> RpcResult<Response>;
}
