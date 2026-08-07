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

use std::{
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use log::info;

pub fn inject_config(mount_path: &Path, src: &str, dst: &str, skip_tls: bool) -> Result<()> {
    let dst_path = mount_path.join(dst);
    if let Some(parent) = dst_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dir {}", parent.display()))?;
    }
    copy_or_download(src, &dst_path, skip_tls)?;
    info!("Config injected: {} -> {}", src, dst_path.display());
    Ok(())
}

pub fn run_security_scripts(mount_path: &Path) -> Result<()> {
    let sec_script = mount_path.join("usr/local/kubeos/security-tools.sh");
    if !sec_script.exists() {
        return Ok(());
    }
    info!("Security hardening enabled, running scripts...");
    mount_proc_dev_sys(mount_path);
    let result = run_in_chroot(mount_path);
    unmount_dir(mount_path);
    result
}

fn mount_proc_dev_sys(target: &Path) {
    let target_str = target.to_str().unwrap_or("");
    let _ = std::process::Command::new("mount")
        .args(["-t", "proc", "none", &format!("{}/proc", target_str)])
        .status();
    let _ = std::process::Command::new("mount")
        .args(["--bind", "/dev", &format!("{}/dev", target_str)])
        .status();
    let _ = std::process::Command::new("mount")
        .args(["-t", "sysfs", "none", &format!("{}/sys", target_str)])
        .status();
}

fn run_in_chroot(mount_path: &Path) -> Result<()> {
    let path = mount_path.to_str().context("Failed to convert mount path")?;
    let script = "if [ -f /usr/local/kubeos/security-tools.sh ]; then bash /usr/local/kubeos/security-tools.sh; fi\n\
                  if [ -f /usr/local/kubeos/modify-stig-dynamic.sh ]; then bash /usr/local/kubeos/modify-stig-dynamic.sh; fi";
    let output = std::process::Command::new("chroot")
        .args([path, "bash", "-c", script])
        .status()
        .with_context(|| "Failed to run security scripts in chroot")?;
    if !output.success() {
        anyhow::bail!("Security scripts execution failed");
    }
    Ok(())
}

fn unmount_dir(target: &Path) {
    let target_str = target.to_str().unwrap_or("");
    let _ = std::process::Command::new("umount")
        .arg(&format!("{}/proc", target_str))
        .status();
    let _ = std::process::Command::new("umount")
        .arg(&format!("{}/dev", target_str))
        .status();
    let _ = std::process::Command::new("umount")
        .arg(&format!("{}/sys", target_str))
        .status();
}

fn copy_or_download(src: &str, dest: &Path, skip_tls: bool) -> Result<()> {
    if src.starts_with("http://") || src.starts_with("https://") {
        info!("Downloading config from URL: {}", src);
        let mut args = vec![
            "-sSL",
            "--fail",
            "-o",
            dest.to_str().context("Failed to convert destination path")?,
        ];
        if skip_tls {
            args.push("--insecure");
        }
        args.push(src);
        std::process::Command::new("curl")
            .args(&args)
            .status()
            .with_context(|| format!("Failed to execute curl for {}", src))?;
    } else {
        info!("Copying config from file: {}", src);
        fs::copy(src, dest)
            .with_context(|| format!("Failed to copy {} to {}", src, dest.display()))?;
    }
    Ok(())
}
