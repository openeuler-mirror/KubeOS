/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2025. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::process::exit;

use clap::{Parser, Subcommand};
use cli::{
    client::Client,
    method::{callable_method::RpcMethod, prepare::PrepareCmdMethod},
};
use manager::api::{CmdRequest, CertsInfo};

const SOCK_PATH: &str = "/run/os-agent/os-agent.sock";

#[derive(Parser)]
#[clap(name = "kbosctl")]
#[clap(about = "CLI tool for KubeOS upgrade and rollback")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Upgrade {
        #[clap(long)]
        os_image: String,
        #[clap(long)]
        cloud_init_config: Option<String>,
        #[clap(long)]
        ignition_config: Option<String>,
        #[clap(long)]
        skip_tls: bool,
        #[clap(long)]
        reboot: bool,
    },
    Rollback {
        #[clap(long)]
        reboot: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let client = Client::new(SOCK_PATH);

    let req = match cli.command {
        Commands::Upgrade { os_image, cloud_init_config, ignition_config, skip_tls, reboot } => CmdRequest {
            version: String::new(),
            image_url: String::new(),
            certs: CertsInfo { ca_cert: String::new(), client_cert: String::new(), client_key: String::new() },
            oci_image: os_image,
            cloud_init_config,
            ignition_config,
            skip_tls,
            reboot,
            is_rollback: false,
        },
        Commands::Rollback { reboot } => CmdRequest {
            version: String::new(),
            image_url: String::new(),
            certs: CertsInfo { ca_cert: String::new(), client_cert: String::new(), client_key: String::new() },
            oci_image: String::new(),
            cloud_init_config: None,
            ignition_config: None,
            skip_tls: false,
            reboot,
            is_rollback: true,
        },
    };

    let method = PrepareCmdMethod::new(req);
    match method.call(&client) {
        Ok(resp) => {
            println!("Operation completed: {:?}", resp.status);
            exit(0);
        },
        Err(e) => {
            eprintln!("Operation failed: {:?}", e);
            exit(1);
        },
    }
}
