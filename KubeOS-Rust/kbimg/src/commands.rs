/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2024. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use serde::Deserialize;

#[derive(Parser)]
#[clap(name = "kbimg")]
#[clap(author, version, about)]
#[clap(about = "CLI tool for generating various types of image for KubeOS")]
pub struct Cli {
    /// Enable debug mode, generate the scripts without execution
    #[clap(short, long, action)]
    pub debug: bool,
    #[clap(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Create a new KubeOS image
    Create {
        #[arg(value_enum)]
        image_type: CreateType,
        /// Path to the configuration file
        #[arg(short, long, value_parser)]
        file: PathBuf,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum CreateType {
    #[clap(name = "vm-img")]
    VM,
    #[clap(name = "pxe-img")]
    PXE,
    #[clap(name = "upgrade-img")]
    Upgrade,
    #[clap(name = "admin-container")]
    AdminContainer,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RepoInfo {
    /// Required: KubeOS version
    pub version: String,
    /// Required: Repo path for installing packages
    pub repo_path: PathBuf,
    /// Required: Path to the os-agent binary
    pub agent_path: PathBuf,
    /// Required: Encrypted password for root user
    pub root_passwd: String,
    /// Required for creating upgrade docker image
    pub upgrade_img: Option<String>,
    /// Required: RPM packages
    pub rpmlist: Vec<String>,
    /// Optional: boot mode, default is uefi, enable this flag for legacy bios
    pub legacy_bios: bool,
    pub image_type: Option<ImageType>,
    pub arch: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DockerImgInfo {
    /// Required: Name of the container image
    pub upgrade_img: String,
    /// Optional: boot mode, default is uefi, enable this flag for legacy bios
    pub legacy_bios: bool,
    pub image_type: Option<ImageType>,
    pub arch: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AdminContainerInfo {
    /// Required: Name of the container image
    pub img_name: String,
    /// Required: Path to the hostshell binary
    pub hostshell: PathBuf,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    pub from_repo: Option<RepoInfo>,
    pub from_dockerimg: Option<DockerImgInfo>,
    pub admin_container: Option<AdminContainerInfo>,
    pub pxe_config: Option<PxeConfig>,
    pub users: Option<Vec<User>>,
    pub copy_files: Option<Vec<CopyFile>>,
    pub grub: Option<Grub>,
    pub systemd_service: Option<SystemdService>,
    pub chroot_script: Option<ChrootScript>,
    pub disk_partition: Option<DiskPartition>,
    pub persist_mkdir: Option<PersistMkdir>,
    pub dm_verity: Option<DmVerity>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    pub name: String,
    pub passwd: String,
    pub primary_group: Option<String>,
    pub groups: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CopyFile {
    pub src: String,
    pub dst: String,
    pub create_dir: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Grub {
    pub passwd: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SystemdService {
    pub name: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChrootScript {
    pub path: PathBuf,
    pub rm: Option<bool>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DiskPartition {
    pub root: u32,
    pub img_size: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PersistMkdir {
    pub name: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PxeConfig {
    pub rootfs_name: String,
    pub disk: String,
    pub server_ip: String,
    pub local_ip: Option<String>,
    pub route_ip: String,
    pub netmask: Option<String>,
    pub net_name: Option<String>,
    pub dhcp: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DmVerity {
    pub efi_key: String,
    pub grub_key: String,
    pub keys_dir: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
pub enum ImageType {
    #[default]
    #[serde(rename = "vm-repo")]
    VMRepo,
    #[serde(rename = "vm-docker")]
    VMDocker,
    #[serde(rename = "pxe-repo")]
    PxeRepo,
    #[serde(rename = "pxe-docker")]
    PxeDocker,
    #[serde(rename = "admin-container")]
    AdminContainer,
    #[serde(rename = "upgrade")]
    UpgradeImage,
}

impl From<&str> for ImageType {
    fn from(input: &str) -> Self {
        match input {
            "vm-repo" => ImageType::VMRepo,
            "vm-docker" => ImageType::VMDocker,
            "pxe-repo" => ImageType::PxeRepo,
            "pxe-docker" => ImageType::PxeDocker,
            "admin-container" => ImageType::AdminContainer,
            "upgrade" => ImageType::UpgradeImage,
            _ => ImageType::VMRepo,
        }
    }
}
