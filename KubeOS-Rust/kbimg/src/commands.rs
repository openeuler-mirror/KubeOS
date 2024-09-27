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

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use serde::Deserialize;

#[derive(Parser)]
#[clap(name = "kbimg")]
#[clap(author, version, about)]
#[clap(long_about = "A tool for creating KubeOS images.")]
pub struct Cli {
    /// Path to the detailed configuration toml file
    #[clap(short, long, value_parser)]
    pub config: Option<PathBuf>,
    /// Enable debug mode, keep the scripts after execution
    #[clap(short, long, action)]
    pub debug: bool,
    #[clap(subcommand)]
    pub commands: Option<Commands>,
}

#[derive(Subcommand, Debug, Deserialize)]
pub enum Commands {
    /// Create a new container image for upgrading KubeOS
    #[clap(name = "upgrade")]
    UpgradeImage(RepoInfo),
    /// Create a new KubeOS vm image from repo
    #[clap(name = "vm-repo")]
    VMRepo(RepoInfo),
    /// Create a new KubeOS vm image from docker image
    #[clap(name = "vm-docker")]
    VMDocker(DockerInfo),
    /// Create a new KubeOS pxe image from repo
    #[clap(name = "pxe-repo")]
    PxeRepo(RepoInfo),
    /// Create a new KubeOS pxe image from docker image
    #[clap(name = "pxe-docker")]
    PxeDocker(DockerInfo),
    /// Create a KubeOS admin-container image
    #[clap(name = "admin-container")]
    AdminContainer(AdminContainerInfo),
}

#[derive(Args, Debug, Deserialize, Clone)]
pub struct RepoInfo {
    /// Required: KubeOS version
    #[clap(short, long, value_parser)]
    pub version: String,
    /// Required: Repo path for installing packages
    #[clap(short = 'p', long, value_parser)]
    pub repo_path: PathBuf,
    /// Required: Path to the agent binary
    #[clap(short = 'b', long, value_parser)]
    pub agent_path: PathBuf,
    /// Required: Encrypted password for root user
    #[clap(short = 'e', long, value_parser)]
    pub root_passwd: String,
    /// Required for upgrade
    #[clap(short = 'd', long, value_parser)]
    pub docker_img: Option<String>,
    /// Required: RPM packages
    #[clap(short = 'r', long, value_parser)]
    pub rpmlist: Vec<String>,
    /// Optional: boot mode, default is uefi, enable this flag for legacy bios
    #[clap(short, long, value_parser)]
    pub legacy_bios: bool,
    #[clap(skip)]
    pub image_type: String,
    #[clap(skip)]
    pub arch: Option<String>,
}

#[derive(Args, Debug, Deserialize, Clone)]
pub struct DockerInfo {
    /// Required: Name of the container image
    #[clap(short, long, value_parser)]
    pub docker_img: String,
    #[clap(skip)]
    pub image_type: String,
}

#[derive(Args, Debug, Deserialize, Clone)]
pub struct AdminContainerInfo {
    /// Required: Name of the container image
    #[clap(short, long, value_parser)]
    pub docker_img: String,
    /// Required: Path to the Dockerfile
    #[clap(short, long, value_parser)]
    pub dockerfile: PathBuf,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    pub from_repo: Option<RepoInfo>,
    pub from_dockerimg: Option<DockerInfo>,
    pub admin_container: Option<AdminContainerInfo>,
    pub users: Option<Vec<User>>,
    pub copy_files: Option<Vec<CopyFile>>,
    pub grub: Option<Grub>,
    pub systemd_service: Option<SystemdService>,
    pub chroot_script: Option<ChrootScript>,
    pub disk_partition: Option<DiskPartition>,
    pub persist_mkdir: Option<PersistMkdir>,
    pub pxe_config: Option<PxeConfig>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    pub name: String,
    pub passwd: String,
    pub groups: Option<Vec<String>>,
    pub sudo: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CopyFile {
    pub src: String,
    pub dst: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Grub {
    pub passwd: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SystemdService {
    pub name: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChrootScript {
    pub path: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DiskPartition {
    pub first: u32,
    pub second: u32,
    pub third: u32,
    pub img_size: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PersistMkdir {
    pub name: Vec<String>,
}

// pxe config
#[derive(Debug, Deserialize, Clone)]
pub struct PxeConfig {
    pub rootfs_name: String,
    pub disk: String,
    pub server_ip: String,
    pub local_ip: String,
    pub route_ip: String,
    pub netmask: String,
    pub net_name: String,
}
