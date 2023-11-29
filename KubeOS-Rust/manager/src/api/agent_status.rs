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

use serde::{Deserialize, Serialize};

const AGENT_STATUS_UNKNOWN: &str = "UNKNOWN";
const AGENT_STATUS_NOT_APPLIED: &str = "NOT-APPLIED";
const AGENT_STATUS_UPGRADEREADY: &str = "UPGRADE-READY";
const AGENT_STATUS_UPGRADED: &str = "UPGRADED";
const AGENT_STATUS_ROLLBACKED: &str = "ROLLBACKED";
const AGENT_STATUS_CONFIGURED: &str = "CONFIGURED";
const AGENT_STATUS_CLEANEDUP: &str = "CLEANEDUP";

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum AgentStatus {
    Unknown,
    NotApplied,
    UpgradeReady,
    Upgraded,
    Rollbacked,
    Configured,
    CleanedUp,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AgentStatus::Unknown => AGENT_STATUS_UNKNOWN,
            AgentStatus::NotApplied => AGENT_STATUS_NOT_APPLIED,
            AgentStatus::UpgradeReady => AGENT_STATUS_UPGRADEREADY,
            AgentStatus::Upgraded => AGENT_STATUS_UPGRADED,
            AgentStatus::Rollbacked => AGENT_STATUS_ROLLBACKED,
            AgentStatus::Configured => AGENT_STATUS_CONFIGURED,
            AgentStatus::CleanedUp => AGENT_STATUS_CLEANEDUP,
        })
    }
}
