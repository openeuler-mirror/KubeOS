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

pub fn request(
    client: &Client,
    command: &str,
    params: Vec<Box<RawValue>>,
) -> Result<Response, anyhow::Error> {
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
                e.source()
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Connection timeout".to_string())
            )
        }
        Error::Json(e) => {
            debug!("Json parse error: {:?}", e);
            anyhow!("Failed to parse response")
        }
        Error::Rpc(ref e) => match e.message == "Method not found" {
            true => {
                anyhow!("Method is unimplemented")
            }
            false => {
                anyhow!("{}", e.message)
            }
        },
        _ => {
            debug!("{:?}", error);
            anyhow!("Response is invalid")
        }
    }
}
