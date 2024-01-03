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

pub const KERNEL_SYSCTL: &str = "kernel.sysctl";
pub const KERNEL_SYSCTL_PERSIST: &str = "kernel.sysctl.persist";
pub const GRUB_CMDLINE_CURRENT: &str = "grub.cmdline.current";
pub const GRUB_CMDLINE_NEXT: &str = "grub.cmdline.next";

pub const DEFAULT_PROC_PATH: &str = "/proc/sys/";
pub const DEFAULT_KERNEL_CONFIG_PATH: &str = "/etc/sysctl.conf";
pub const DEFAULT_GRUB_CFG_PATH: &str = "/boot/efi/EFI/openEuler/grub.cfg";
pub const DEFAULT_GRUBENV_PATH: &str = "/boot/efi/EFI/openEuler/grubenv";

pub const PERSIST_DIR: &str = "/persist";
pub const ROOTFS_ARCHIVE: &str = "os.tar";
pub const UPDATE_DIR: &str = "KubeOS-Update";
pub const MOUNT_DIR: &str = "kubeos-update";
pub const OS_IMAGE_NAME: &str = "update.img";
pub const CERTS_PATH: &str = "/etc/KubeOS/certs/";

pub const DEFAULT_KERNEL_CONFIG_PERM: u32 = 0o644;
pub const DEFAULT_GRUB_CFG_PERM: u32 = 0o751;
pub const IMAGE_PERMISSION: u32 = 0o600;

pub const ONLY_KEY: usize = 1;
pub const KV_PAIR: usize = 2;
pub const NEED_BYTES: i64 = 3 * 1024 * 1024 * 1024;
