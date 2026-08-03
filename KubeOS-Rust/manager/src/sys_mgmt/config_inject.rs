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

pub fn inject_cloud_init(mount_path: &Path, src: &str, skip_tls: bool) -> Result<()> {
    let seed_dir = mount_path.join("var/lib/cloud/seed/nocloud-net");
    fs::create_dir_all(&seed_dir)
        .with_context(|| format!("Failed to create cloud-init seed dir {}", seed_dir.display()))?;
    copy_or_download(src, &seed_dir.join("user-data"), skip_tls)?;
    fs::write(seed_dir.join("meta-data"), "instance-id: KubeOS\n")?;
    info!("Cloud-init config injected to {}", seed_dir.display());
    Ok(())
}

pub fn inject_ignition(mount_path: &Path, src: &str, skip_tls: bool) -> Result<()> {
    let ign_dir = mount_path.join("usr/lib/dracut/modules.d/30ignition");
    fs::create_dir_all(&ign_dir)
        .with_context(|| format!("Failed to create ignition dir {}", ign_dir.display()))?;
    copy_or_download(src, &ign_dir.join("config.ign"), skip_tls)?;
    info!("Ignition config injected to {}", ign_dir.display());
    Ok(())
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
