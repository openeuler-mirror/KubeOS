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

use std::{fs, path::{Path, PathBuf}};

use anyhow::{bail, Context, Result};
use log::{debug, info};

use crate::{
    api::CmdRequest,
    sys_mgmt::{IMAGE_PERMISSION, NEED_BYTES},
    utils::*,
};

pub struct SkopeoImageHandler<T: CommandExecutor> {
    pub paths: PreparePath,
    pub executor: T,
}

impl<T: CommandExecutor> SkopeoImageHandler<T> {
    /// Downloads the OCI image and creates the OS image, leaving it mounted
    /// so the caller can inject configs before calling finish() → install().
    pub fn download_image(&self, req: &CmdRequest) -> Result<UpgradeImageManager<T>> {
        perpare_env(&self.paths, NEED_BYTES, IMAGE_PERMISSION)?;

        info!("Start pulling OCI image via skopeo: {}", req.oci_image);
        let oci_dir = self.paths.update_path.join("oci-layout");
        self.executor.run_command(
            "skopeo",
            &[
                "copy",
                &req.oci_image,
                &format!("oci:{}:latest", oci_dir.to_str().context("Failed to convert oci dir path")?),
            ],
        )?;

        self.extract_oci_layers(&oci_dir, &self.paths.tar_path)?;

        let (_, next_partition_info) = get_partition_info(&self.executor)?;
        let img_manager = UpgradeImageManager::new(
            self.paths.clone(),
            next_partition_info,
            self.executor.clone(),
            false,
        );
        // Individual steps: create image, format, mount, extract tar.
        // Do NOT call create_os_image (which includes clean_env) so the
        // caller can inject configs into the still-mounted image.
        img_manager.create_image_file(IMAGE_PERMISSION)?;
        img_manager.format_image()?;
        img_manager.mount_image()?;
        img_manager.extract_tar_to_image()?;
        Ok(img_manager)
    }

    /// Cleans up the work directory and unmounts the image.
    /// Must be called after config injection and before install().
    pub fn finish(&self) -> Result<()> {
        clean_env(&self.paths.update_path, &self.paths.mount_path, &PathBuf::new())
    }

    fn extract_oci_layers(&self, oci_dir: &Path, tar_path: &Path) -> Result<()> {
        let index_path = oci_dir.join("index.json");
        if !index_path.exists() {
            bail!("OCI index.json not found at {}", index_path.display());
        }
        let index_content = fs::read_to_string(&index_path)
            .with_context(|| format!("Failed to read {}", index_path.display()))?;

        let manifest_hash = index_content
            .lines()
            .find_map(|line| {
                line.find("\"digest\":\"sha256:")
                    .map(|_| line)
            })
            .and_then(|line| {
                let start = line.find("sha256:")?;
                let hash = &line[start..];
                let end = hash.find('"').unwrap_or(hash.len());
                Some(hash[..end].replace(':', "/"))
            })
            .ok_or_else(|| anyhow::anyhow!("Failed to parse manifest digest from index.json"))?;

        let manifest_file = oci_dir.join("blobs").join(&manifest_hash);
        if !manifest_file.exists() {
            bail!("Manifest file not found at {}", manifest_file.display());
        }
        let manifest_content = fs::read_to_string(&manifest_file)
            .with_context(|| format!("Failed to read {}", manifest_file.display()))?;

        let layers_section = manifest_content
            .find("\"layers\":[")
            .and_then(|start| manifest_content[start..].find(']').map(|end| &manifest_content[start..start + end + 1]))
            .unwrap_or("");

        let mut layer_digests: Vec<String> = Vec::new();
        let mut remaining = layers_section;
        while let Some(digest_start) = remaining.find("sha256:") {
            let digest_str = &remaining[digest_start..];
            let digest_end = digest_str.find('"').unwrap_or(digest_str.len());
            let digest = digest_str[..digest_end].replace(':', "/");
            layer_digests.push(digest);
            remaining = &remaining[digest_start + digest_end..];
        }

        if layer_digests.is_empty() {
            bail!("No layers found in OCI manifest");
        }
        info!("Extracting {} OCI layers to {}", layer_digests.len(), tar_path.display());

        let tar_str = tar_path.to_str().context("Failed to convert tar path")?;
        for layer_digest in layer_digests {
            let layer_file = oci_dir.join("blobs").join(&layer_digest);
            if !layer_file.exists() {
                bail!("Layer file not found at {}", layer_file.display());
            }
            debug!("Extracting layer: {}", layer_file.display());
            let is_gzip = std::process::Command::new("file")
                .arg(layer_file.to_str().unwrap())
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase().contains("gzip"))
                .unwrap_or(false);
            if is_gzip {
                self.executor.run_command(
                    "bash",
                    &["-c", &format!("zcat {} > {}", layer_file.to_str().unwrap(), tar_str)],
                )?;
            } else {
                self.executor.run_command(
                    "bash",
                    &["-c", &format!("cat {} > {}", layer_file.to_str().unwrap(), tar_str)],
                )?;
            }
        }

        fs::remove_dir_all(oci_dir)?;
        info!("OCI layers extracted successfully");
        Ok(())
    }
}
