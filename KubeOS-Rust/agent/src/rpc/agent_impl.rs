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
use log::{debug, info};
use manager::{
    api::{AgentStatus, ConfigureRequest, ImageType, Response, UpgradeRequest},
    sys_mgmt::{CtrImageHandler, DiskImageHandler, DockerImageHandler, CONFIG_TEMPLATE, DEFAULT_GRUBENV_PATH},
    utils::{get_partition_info, is_dmv_mode, switch_boot_menuentry, CommandExecutor, RealCommandExecutor},
};
use nix::{sys::reboot::RebootMode, unistd::sync};

use super::{
    agent::Agent,
    function::{RpcFunction, RpcResult},
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

    fn configure(&self, req: ConfigureRequest) -> RpcResult<Response> {
        RpcFunction::call(|| self.configure_impl(req))
    }

    fn rollback(&self) -> RpcResult<Response> {
        RpcFunction::call(|| self.rollback_impl())
    }
}

impl Default for AgentImpl {
    fn default() -> Self {
        Self { mutex: Mutex::new(()), disable_reboot: false }
    }
}

impl AgentImpl {
    fn prepare_upgrade_impl(&self, req: UpgradeRequest) -> Result<Response> {
        let lock = self.mutex.try_lock();
        if lock.is_err() {
            bail!("os-agent is processing another request");
        }
        debug!("Received an 'prepare upgrade' request: {:?}", req);
        info!("Start preparing for upgrading to version: {}", req.version);

        let dmv_mode = is_dmv_mode(&RealCommandExecutor {});
        info!("dm-verity mode: {}", dmv_mode);
        let handler: Box<ImageType<RealCommandExecutor>> = match req.image_type.as_str() {
            "containerd" => Box::new(ImageType::Containerd(CtrImageHandler { dmv: dmv_mode, ..Default::default() })),
            "docker" => Box::new(ImageType::Docker(DockerImageHandler { dmv: dmv_mode, ..Default::default() })),
            "disk" => Box::new(ImageType::Disk(DiskImageHandler { dmv: dmv_mode, ..Default::default() })),
            _ => bail!("Invalid image type \"{}\"", req.image_type),
        };

        let image_manager = handler.download_image(&req)?;
        info!("Ready to install image: {:?}", image_manager.paths.image_path.display());
        image_manager.install()?;

        Ok(Response { status: AgentStatus::UpgradeReady })
    }

    fn upgrade_impl(&self) -> Result<Response> {
        let lock = self.mutex.try_lock();
        if lock.is_err() {
            bail!("os-agent is processing another request");
        }
        info!("Start to upgrade");
        let command_executor = RealCommandExecutor {};
        let dmv_mode = is_dmv_mode(&command_executor);
        info!("dm-verity mode: {}", dmv_mode);
        if dmv_mode {
            command_executor.run_command("/usr/bin/kubeos-dmv", &["switch"])?;
            info!("Switch to next boot partition and reboot");
            self.reboot()?;
            return Ok(Response { status: AgentStatus::Upgraded });
        }
        let (_, next_partition_info) = get_partition_info(&command_executor)?;

        // based on boot mode use different command to switch boot partition
        let device = next_partition_info.device.as_str();
        let menuentry = next_partition_info.menuentry.as_str();
        switch_boot_menuentry(&command_executor, DEFAULT_GRUBENV_PATH, menuentry)?;
        info!("Switch to boot partition: {}, device: {}", menuentry, device);
        self.reboot()?;
        Ok(Response { status: AgentStatus::Upgraded })
    }

    fn configure_impl(&self, mut req: ConfigureRequest) -> Result<Response> {
        let lock = self.mutex.try_lock();
        if lock.is_err() {
            bail!("os-agent is processing another request");
        }
        debug!("Received a 'configure' request: {:?}", req);
        info!("Start to configure");
        let config_map = &*CONFIG_TEMPLATE;
        for config in req.configs.iter_mut() {
            let config_type = &config.model;
            if let Some(configuration) = config_map.get(config_type) {
                debug!("Found configuration type: \"{}\"", config_type);
                configuration.set_config(config)?;
            } else {
                bail!("Unknown configuration type: \"{}\"", config_type);
            }
        }
        Ok(Response { status: AgentStatus::Configured })
    }

    fn rollback_impl(&self) -> Result<Response> {
        let lock = self.mutex.try_lock();
        if lock.is_err() {
            bail!("os-agent is processing another request");
        }
        info!("Start to rollback");
        let command_executor = RealCommandExecutor {};
        let dmv_mode = is_dmv_mode(&command_executor);
        info!("dm-verity mode: {}", dmv_mode);
        if dmv_mode {
            command_executor.run_command("/usr/bin/kubeos-dmv", &["switch"])?;
            info!("Switch to next boot partition and reboot");
            self.reboot()?;
            return Ok(Response { status: AgentStatus::Upgraded });
        }
        let (_, next_partition_info) = get_partition_info(&command_executor)?;
        switch_boot_menuentry(
            &command_executor,
            manager::sys_mgmt::DEFAULT_GRUBENV_PATH,
            &next_partition_info.menuentry,
        )?;
        info!("Switch to boot partition: {}, device: {}", next_partition_info.menuentry, next_partition_info.device);
        self.reboot()?;
        Ok(Response { status: AgentStatus::Rollbacked })
    }

    fn reboot(&self) -> Result<()> {
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
    use std::collections::HashMap;

    use manager::api::{CertsInfo, Sysconfig};

    use super::*;

    #[test]
    fn test_reboot() {
        let mut agent = AgentImpl::default();
        agent.disable_reboot = true;
        let res = agent.reboot();
        assert!(res.is_ok());
    }

    #[test]
    fn test_configure() {
        let agent = AgentImpl::default();
        let req = ConfigureRequest {
            configs: vec![Sysconfig {
                model: "kernel.sysctl".to_string(),
                config_path: "".to_string(),
                contents: HashMap::new(),
            }],
        };
        let res = agent.configure(req).unwrap();
        assert_eq!(res, Response { status: AgentStatus::Configured });

        let req = ConfigureRequest {
            configs: vec![Sysconfig {
                model: "invalid".to_string(),
                config_path: "".to_string(),
                contents: HashMap::new(),
            }],
        };
        let res = agent.configure(req);
        assert!(res.is_err());

        // test lock
        let _lock = agent.mutex.lock().unwrap();
        let req = ConfigureRequest {
            configs: vec![Sysconfig {
                model: "kernel.sysctl".to_string(),
                config_path: "".to_string(),
                contents: HashMap::new(),
            }],
        };
        let res = agent.configure(req);
        assert!(res.is_err());
    }

    #[test]
    fn test_prepare_upgrade() {
        let agent = AgentImpl::default();
        let req = UpgradeRequest {
            version: "v2".into(),
            check_sum: "xxx".into(),
            image_type: "xxx".into(),
            container_image: "xxx".into(),
            image_url: "".to_string(),
            flag_safe: false,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        let res = agent.prepare_upgrade(req);
        assert!(res.is_err());
    }
}
