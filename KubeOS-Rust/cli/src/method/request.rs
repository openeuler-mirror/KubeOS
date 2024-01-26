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

use anyhow::anyhow;
use jsonrpc::{Error, Response};
use log::debug;
use serde_json::value::RawValue;

use crate::client::Client;

pub fn request(client: &Client, command: &str, params: Vec<Box<RawValue>>) -> Result<Response, anyhow::Error> {
    let request = client.build_request(command, &params);
    let response = client.send_request(request).map_err(parse_error);
    debug!("{:#?}", response);
    response
}

pub fn parse_error(error: Error) -> anyhow::Error {
    match error {
        Error::Transport(e) => {
            anyhow!(
                "Cannot connect to KubeOS os-agent unix socket, {}",
                e.source().map(|e| e.to_string()).unwrap_or_else(|| "Connection timeout".to_string())
            )
        },
        Error::Json(e) => {
            debug!("Json parse error: {:?}", e);
            anyhow!("Failed to parse response")
        },
        Error::Rpc(ref e) => match e.message == "Method not found" {
            true => {
                anyhow!("Method is unimplemented")
            },
            false => {
                anyhow!("{}", e.message)
            },
        },
        _ => {
            debug!("{:?}", error);
            anyhow!("Response is invalid")
        },
    }
}

#[cfg(test)]
mod tests {
    use jsonrpc::error::RpcError;
    use serde::de::Error as DeError;

    use super::*;

    #[test]
    fn test_parse_error() {
        // Test Error::Transport
        let transport_error =
            Error::Transport(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Connection timeout")));
        let result = parse_error(transport_error);
        assert_eq!(result.to_string(), "Cannot connect to KubeOS os-agent unix socket, Connection timeout");

        // Test Error::Json
        let json_error = Error::Json(serde_json::Error::custom("Failed to parse response"));
        let result = parse_error(json_error);
        assert_eq!(result.to_string(), "Failed to parse response");

        // Test Error::Rpc with "Method not found" message
        let rpc_error = Error::Rpc(RpcError { code: -32601, message: "Method not found".to_string(), data: None });
        let result = parse_error(rpc_error);
        assert_eq!(result.to_string(), "Method is unimplemented");

        // Test Error::Rpc with other message
        let rpc_error = Error::Rpc(RpcError { code: -32603, message: "Internal server error".to_string(), data: None });
        let result = parse_error(rpc_error);
        assert_eq!(result.to_string(), "Internal server error");

        // Test other Error variant
        let other_error = Error::VersionMismatch;
        let result = parse_error(other_error);
        assert_eq!(result.to_string(), "Response is invalid");
    }
}
