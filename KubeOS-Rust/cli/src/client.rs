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

use std::path::Path;

use jsonrpc::{
    simple_uds::UdsTransport, Client as JsonRPCClient, Request as JsonRPCRequest, Response as JsonRPCResponse,
};
use serde_json::value::RawValue;

pub struct Client {
    json_rpc_client: JsonRPCClient,
}

pub struct Request<'a>(JsonRPCRequest<'a>);

impl<'a> Request<'a> {}

impl Client {
    pub fn new<P: AsRef<Path>>(socket_path: P) -> Self {
        Client { json_rpc_client: JsonRPCClient::with_transport(UdsTransport::new(socket_path)) }
    }

    pub fn build_request<'a>(&self, command: &'a str, params: &'a Vec<Box<RawValue>>) -> Request<'a> {
        let json_rpc_request = self.json_rpc_client.build_request(command, params);
        let request = Request(json_rpc_request);
        request
    }

    pub fn send_request(&self, request: Request) -> Result<JsonRPCResponse, jsonrpc::Error> {
        self.json_rpc_client.send_request(request.0)
    }
}

#[cfg(test)]
mod test {
    use kubeos_manager::api;

    use super::*;
    use crate::method::{callable_method::RpcMethod, configure::ConfigureMethod};

    #[test]
    #[ignore]
    fn test_client() {
        let socket_path = "/home/yuhang/os-agent-rust.sock";
        let cli = Client::new(socket_path);

        let configured = api::AgentStatus::Configured;
        let resp = api::Response { status: configured };
        let config_request = api::ConfigureRequest {
            configs: vec![api::Sysconfig {
                model: "kernel.sysctl".into(),
                config_path: "".into(),
                contents: std::collections::hash_map::HashMap::new(),
            }],
        };
        let config_resp = ConfigureMethod::new(config_request).call(&cli).unwrap();
        assert_eq!(resp, config_resp);
    }
}
