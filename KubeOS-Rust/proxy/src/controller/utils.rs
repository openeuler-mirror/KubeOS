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

use log::{debug, info};

use common::{
    crd::{Configs, OSInstance, OSInstanceStatus, OS},
    values::{NODE_STATUS_CONFIG, NODE_STATUS_IDLE, NODE_STATUS_UPGRADE},
};

#[derive(PartialEq, Clone, Copy)]
pub enum ConfigType {
    UpgradeConfig,
    SysConfig,
}

pub enum ConfigOperation {
    DoNothing,
    Reassign,
    UpdateConfig,
}

pub struct ConfigInfo {
    pub need_config: bool,
    pub configs: Option<Configs>,
}

impl ConfigType {
    pub fn check_config_version(&self, os: &OS, osinstance: &OSInstance) -> ConfigOperation {
        debug!("start check_config_version");
        let node_status = &osinstance.spec.nodestatus;
        if node_status == NODE_STATUS_IDLE {
            debug!("node status is idle");
            return ConfigOperation::DoNothing;
        };
        match self {
            ConfigType::UpgradeConfig => {
                let os_config_version = get_config_version(os.spec.upgradeconfigs.as_ref());
                let osi_config_version = get_config_version(osinstance.spec.upgradeconfigs.as_ref());
                debug!(
                    "os upgradeconfig version is {}, osinstance spec upragdeconfig version is {}",
                    os_config_version, osi_config_version
                );
                if !check_version(&os_config_version, &osi_config_version) {
                    info!(
                        "os.spec.upgradeconfig.version is not equal to oninstance.spec.upragdeconfig.version, operation: reassgin upgrade to get newest upgradeconfigs"
                    );
                    return ConfigOperation::Reassign;
                }
            },
            ConfigType::SysConfig => {
                let os_config_version = get_config_version(os.spec.sysconfigs.as_ref());
                let osi_config_version = get_config_version(osinstance.spec.sysconfigs.as_ref());
                debug!(
                    "os sysconfig version is {},osinstance spec sysconfig version is {}",
                    os_config_version, osi_config_version
                );
                if !check_version(&os_config_version, &osi_config_version) {
                    if node_status == NODE_STATUS_CONFIG {
                        info!(
                            "os.spec.sysconfig.version is not equal to oninstance.spec.sysconfig.version, operation: reassgin config to get newest sysconfigs"
                        );
                        return ConfigOperation::Reassign;
                    }
                    if node_status == NODE_STATUS_UPGRADE {
                        info!(
                            "os.spec.sysconfig.version is not equal to oninstance.spec.sysconfig.version, operation: update osinstance.spec.sysconfig and reconcile"
                        );
                        return ConfigOperation::UpdateConfig;
                    }
                }
            },
        };
        ConfigOperation::DoNothing
    }
    pub fn check_config_start(&self, osinstance: &OSInstance) -> ConfigInfo {
        debug!("start check_config_start");
        let spec_config_version: String;
        let status_config_version: String;
        let configs: Option<Configs>;
        match self {
            ConfigType::UpgradeConfig => {
                spec_config_version = get_config_version(osinstance.spec.upgradeconfigs.as_ref());
                if let Some(osinstance_status) = osinstance.status.as_ref() {
                    status_config_version = get_config_version(osinstance_status.upgradeconfigs.as_ref());
                } else {
                    status_config_version = get_config_version(None);
                }
                configs = osinstance.spec.upgradeconfigs.clone();
            },
            ConfigType::SysConfig => {
                spec_config_version = get_config_version(osinstance.spec.sysconfigs.as_ref());
                if let Some(osinstance_status) = osinstance.status.as_ref() {
                    status_config_version = get_config_version(osinstance_status.sysconfigs.as_ref());
                } else {
                    status_config_version = get_config_version(None);
                }
                configs = osinstance.spec.sysconfigs.clone();
            },
        }
        debug!(
            "osinstance spec config version is {}, status config version is {}",
            spec_config_version, status_config_version
        );
        if spec_config_version != status_config_version && osinstance.spec.nodestatus != NODE_STATUS_IDLE {
            return ConfigInfo { need_config: true, configs };
        }
        ConfigInfo { need_config: false, configs: None }
    }
    pub fn set_osi_status_config(&self, osinstance: &mut OSInstance) {
        match self {
            ConfigType::UpgradeConfig => {
                if let Some(osi_status) = &mut osinstance.status {
                    osi_status.upgradeconfigs = osinstance.spec.upgradeconfigs.clone();
                } else {
                    osinstance.status = Some(OSInstanceStatus {
                        upgradeconfigs: osinstance.spec.upgradeconfigs.clone(),
                        sysconfigs: None,
                    })
                }
            },
            ConfigType::SysConfig => {
                if let Some(osi_status) = &mut osinstance.status {
                    osi_status.sysconfigs = osinstance.spec.sysconfigs.clone();
                } else {
                    osinstance.status =
                        Some(OSInstanceStatus { upgradeconfigs: None, sysconfigs: osinstance.spec.sysconfigs.clone() })
                }
            },
        }
    }
}

pub fn check_version(version_a: &str, version_b: &str) -> bool {
    version_a.eq(version_b)
}

pub fn get_config_version(configs: Option<&Configs>) -> String {
    if let Some(configs) = configs {
        if let Some(version) = configs.version.as_ref() {
            return version.to_string();
        }
    };
    String::from("")
}
