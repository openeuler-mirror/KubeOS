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

use anyhow::Result;
use controller::{error_policy, reconcile, OperatorController};
use env_logger::{Builder, Env, Target};
use futures::StreamExt;
use kube::{
    api::{Api, ListParams},
    client::Client,
    runtime::controller::{Context, Controller},
};
use log::{error, info};
use tokio::signal;

pub mod controller;
use common::{crd::OS, apiclient::ControllerClient};

const OPERATOR_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
#[tokio::main]
async fn main() -> Result<()> {
    Builder::from_env(Env::default().default_filter_or("operator=info")).target(Target::Stdout).init();
    let client = Client::try_default().await?;
    let os: Api<OS> = Api::all(client.clone());
    let controller_client = ControllerClient::new(client.clone());
    let os_reconciler = OperatorController::new(client.clone(), controller_client.clone());
    info!(
        "os-operator version is {}, starting operator manager",
        OPERATOR_VERSION.unwrap_or("Not Found")
    );
    Controller::new(os, ListParams::default())
        .run(reconcile, error_policy, Context::new(os_reconciler))
        .for_each(|res| async move {
            match res {
                Ok(_) => {}
                Err(e) => error!("reconcile failed: {}", e.to_string()),
            }
        })
        .await;
    signal::ctrl_c().await?;
    info!("os-operator terminated");
    Ok(())
}
