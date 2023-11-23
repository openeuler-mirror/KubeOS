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

use std::{
    fs::{self, DirBuilder, Permissions},
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::Path,
};

use env_logger::{Builder, Env, Target};
use jsonrpc_core::{IoHandler, IoHandlerExtension};
use jsonrpc_ipc_server::ServerBuilder;

mod function;
mod rpc;

use log::info;
use rpc::{Agent, AgentImpl};

const SOCK_PATH: &str = "/run/os-agent/os-agent.sock";
const CARGO_PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

fn start_and_run(sock_path: &str) {
    let socket_path = Path::new(sock_path);

    // Create directory for socket if it doesn't exist
    if let Some(dir_path) = socket_path.parent() {
        if !dir_path.exists() {
            DirBuilder::new()
                .mode(0o750)
                .create(dir_path)
                .expect("Couldn't create directory for socket");
        }
    }

    // Add RPC methods to IoHandler
    let mut io = IoHandler::new();
    AgentImpl::default().to_delegate().augment(&mut io);

    // Build and start server
    let builder = ServerBuilder::new(io);
    let server = builder.start(sock_path).expect("Couldn't open socket");

    let gid = nix::unistd::getgid();
    nix::unistd::chown(socket_path, Some(nix::unistd::ROOT), Some(gid))
        .expect("Couldn't set socket group");

    // Set socket permissions to 0640
    let socket_permissions = Permissions::from_mode(0o640);
    fs::set_permissions(socket_path, socket_permissions).expect("Couldn't set socket permissions");

    info!("os-agent started, waiting for requests...");
    server.wait();
}

fn main() {
    Builder::from_env(Env::default().default_filter_or("info"))
        .target(Target::Stdout)
        .init();

    info!(
        "os-agent version is: {}",
        CARGO_PKG_VERSION.unwrap_or("NOT FOUND")
    );
    start_and_run(SOCK_PATH);
}
