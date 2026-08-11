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

use anyhow::{bail, Context, Result};
use log::info;

pub fn inject_config(mount_path: &Path, src: &str, dst: &str, skip_tls: bool) -> Result<()> {
    let dst = dst.trim_start_matches('/');
    let dst_path = mount_path.join(dst);
    if let Some(parent) = dst_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create dir {}", parent.display()))?;
    }
    copy_or_download(src, &dst_path, skip_tls)?;
    info!("Config injected: {} -> {}", src, dst_path.display());
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
        let status = std::process::Command::new("curl")
            .args(&args)
            .status()
            .with_context(|| format!("Failed to execute curl for {}", src))?;
        if !status.success() {
            bail!("Failed to download config from {}: curl exited with status {}", src, status);
        }
    } else {
        info!("Copying config from file: {}", src);
        fs::copy(src, dest)
            .with_context(|| format!("Failed to copy {} to {}", src, dest.display()))?;
    }
    Ok(())
}
