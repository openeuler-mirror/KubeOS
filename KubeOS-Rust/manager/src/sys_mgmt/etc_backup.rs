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
    os::unix::fs::symlink,
    path::Path,
};

use anyhow::{Context, Result};
use log::info;

pub fn backup_etc_overlay(target_menuentry: &str) -> Result<()> {
    let src = Path::new("/persist/etc");
    let dst_path = format!("/persist/etc-{}", target_menuentry);
    let dst = Path::new(&dst_path);
    if !src.exists() {
        info!("No /persist/etc to backup, skipping");
        return Ok(());
    }
    if dst.exists() {
        if dst.is_symlink() || dst.is_file() {
            fs::remove_file(dst)
                .with_context(|| format!("Failed to remove old etc backup file {}", dst.display()))?;
        } else {
            fs::remove_dir_all(dst)
                .with_context(|| format!("Failed to remove old etc backup {}", dst.display()))?;
        }
    }
    copy_dir_recursive(src, dst)?;
    info!("Backed up /persist/etc to {}", dst.display());
    Ok(())
}

pub fn restore_etc_overlay(target_menuentry: &str) -> Result<()> {
    let backup_path = format!("/persist/etc-{}", target_menuentry);
    let backup = Path::new(&backup_path);
    let persist_etc = Path::new("/persist/etc");
    if !backup.exists() {
        info!("No etc backup found at {}, skipping restore", backup.display());
        return Ok(());
    }
    if persist_etc.exists() {
        fs::remove_dir_all(persist_etc)
            .with_context(|| format!("Failed to remove {}", persist_etc.display()))?;
    }
    copy_dir_recursive(backup, persist_etc)?;
    info!("Restored /persist/etc from {}", backup.display());
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)
        .with_context(|| format!("Failed to read directory {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_symlink() {
            let target = fs::read_link(&src_path)
                .with_context(|| format!("Failed to read symlink {}", src_path.display()))?;
            symlink(&target, &dst_path)
                .with_context(|| format!("Failed to create symlink {}", dst_path.display()))?;
        } else if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
