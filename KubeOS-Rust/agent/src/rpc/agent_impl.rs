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

use std::{sync::Mutex, thread, time::Duration};

use anyhow::{bail, Result};
use log::{debug, error, info};
use nix::{sys::reboot::RebootMode, unistd::sync};

use super::{
    agent::Agent,
    function::{RpcFunction, RpcResult},
};
use manager::{
    api::{AgentStatus, ConfigureRequest, ImageType, Response, UpgradeRequest},
    sys_mgmt::{
        CtrImageHandler, DiskImageHandler, DockerImageHandler, CONFIG_TEMPLATE,
        DEFAULT_GRUBENV_PATH,
    },
    utils::{
        clean_env, get_partition_info, switch_boot_menuentry, PreparePath, RealCommandExecutor,
    },
};

pub struct AgentImpl {
    mutex: Mutex<()>,
    disable_reboot: bool,
}

impl Agent for AgentImpl {
    fn prepare_upgrade(&self, req: UpgradeRequest) -> RpcResult<Response> {
        RpcFunction::call(|| self.prepare_upgrade_impl(req))
    }

    fn upgrade(&self) -> RpcResult<Response> {
        RpcFunction::call(|| self.upgrade_impl())
    }

    fn cleanup(&self) -> RpcResult<Response> {
        RpcFunction::call(|| self.cleanup_impl())
    }

    fn configure(&self, req: ConfigureRequest) -> RpcResult<Response> {
        RpcFunction::call(|| self.configure_impl(req))
    }

    fn rollback(&self) -> RpcResult<Response> {
        RpcFunction::call(|| self.rollback_impl())
    }
}

impl Default for AgentImpl {
    fn default() -> Self {
        Self {
            mutex: Mutex::new(()),
            disable_reboot: false,
        }
    }
}

impl AgentImpl {
    pub fn prepare_upgrade_impl(&self, req: UpgradeRequest) -> Result<Response> {
        let _lock = self.mutex.lock().unwrap();
        debug!("Received an 'prepare upgrade' request: {:?}", req);
        info!("Start preparing for upgrading to version: {}", req.version);

        let handler: Box<ImageType<RealCommandExecutor>> = match req.image_type.as_str() {
            "containerd" => Box::new(ImageType::Containerd(CtrImageHandler::default())),
            "docker" => Box::new(ImageType::Docker(DockerImageHandler::default())),
            "disk" => Box::new(ImageType::Disk(DiskImageHandler::default())),
            _ => bail!("Invalid image type \"{}\"", req.image_type),
        };

        let image_manager = handler.download_image(&req)?;
        info!(
            "Ready to install image: {:?}",
            image_manager.paths.image_path.display()
        );
        image_manager.install()?;

        Ok(Response {
            status: AgentStatus::UpgradeReady,
        })
    }

    pub fn upgrade_impl(&self) -> Result<Response> {
        let _lock = self.mutex.lock().unwrap();
        info!("Start to upgrade");
        let command_executor = RealCommandExecutor {};
        let (_, next_partition_info) = get_partition_info(&command_executor)?;

        // based on boot mode use different command to switch boot partition
        let device = next_partition_info.device.as_str();
        let menuentry = next_partition_info.menuentry.as_str();
        switch_boot_menuentry(&command_executor, DEFAULT_GRUBENV_PATH, menuentry)?;
        info!(
            "Switch to boot partition: {}, device: {}",
            menuentry, device
        );
        self.reboot()?;
        Ok(Response {
            status: AgentStatus::Upgraded,
        })
    }

    pub fn cleanup_impl(&self) -> Result<Response> {
        let _lock = self.mutex.lock().unwrap();
        info!("Start to cleanup");
        let paths = PreparePath::default();
        clean_env(paths.update_path, paths.mount_path, paths.image_path)?;
        Ok(Response {
            status: AgentStatus::CleanedUp,
        })
    }

    pub fn configure_impl(&self, mut req: ConfigureRequest) -> Result<Response> {
        let _lock = self.mutex.lock().unwrap();
        debug!("Received a 'configure' request: {:?}", req);
        info!("Start to configure");
        let config_map = &*CONFIG_TEMPLATE;
        for config in req.configs.iter_mut() {
            let config_type = &config.model;
            if let Some(configuration) = config_map.get(config_type) {
                debug!("Found configuration type: \"{}\"", config_type);
                configuration.set_config(config)?;
            } else {
                error!("Unknown configuration type: \"{}\"", config_type);
                bail!("Unknown configuration type: \"{}\"", config_type);
            }
        }
        Ok(Response {
            status: AgentStatus::Configured,
        })
    }

    pub fn rollback_impl(&self) -> Result<Response> {
        let _lock = self.mutex.lock().unwrap();
        info!("Start to rollback");
        let command_executor = RealCommandExecutor {};
        let (_, next_partition_info) = get_partition_info(&command_executor)?;
        switch_boot_menuentry(
            &command_executor,
            manager::sys_mgmt::DEFAULT_GRUBENV_PATH,
            &next_partition_info.menuentry,
        )?;
        info!(
            "Switch to boot partition: {}, device: {}",
            next_partition_info.menuentry, next_partition_info.device
        );
        self.reboot()?;
        Ok(Response {
            status: AgentStatus::Rollbacked,
        })
    }

    pub fn reboot(&self) -> Result<()> {
        info!("Wait to reboot");
        thread::sleep(Duration::from_secs(1));
        sync();
        if self.disable_reboot {
            return Ok(());
        }
        nix::sys::reboot::reboot(RebootMode::RB_AUTOBOOT)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use manager::api::{CertsInfo, Sysconfig};
    use std::collections::HashMap;

    #[test]
    fn configure_impl_tests() {
        let agent = AgentImpl::default();
        let req = ConfigureRequest {
            configs: vec![Sysconfig {
                model: "kernel.sysctl".to_string(),
                config_path: "".to_string(),
                contents: HashMap::new(),
            }],
        };
        let res = agent.configure_impl(req).unwrap();
        assert_eq!(
            res,
            Response {
                status: AgentStatus::Configured,
            }
        );

        let req = ConfigureRequest {
            configs: vec![Sysconfig {
                model: "invalid".to_string(),
                config_path: "".to_string(),
                contents: HashMap::new(),
            }],
        };
        let res = agent.configure_impl(req);
        assert!(res.is_err());
    }

    #[test]
    fn upgrade_impl_tests() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
        let agent = AgentImpl::default();
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "xxx".into(),
            image_type: "xxx".into(),
            container_image: "xxx".into(),
            image_url: "".to_string(),
            flag_safe: false,
            mtls: false,
            certs: CertsInfo {
                ca_cert: "".to_string(),
                client_cert: "".to_string(),
                client_key: "".to_string(),
            },
        };
        let res = agent.prepare_upgrade_impl(req);
        assert!(res.is_err());
    }
}
