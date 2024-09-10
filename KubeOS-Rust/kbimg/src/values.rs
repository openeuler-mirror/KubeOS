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

pub(crate) const SCRIPTS_DIR: &str = "./scripts-auto";
pub(crate) const KBIMG_SH: &str = "kbimg.sh";
pub(crate) const BOOTLOADER_SH: &str = "bootloader.sh";
pub(crate) const SET_IN_CHROOT_SH: &str = "set_in_chroot.sh";
pub(crate) const GRUB_CFG: &str = "grub.cfg";
pub(crate) const RPMLIST: &str = "rpmlist";
pub(crate) const DOCKERFILE: &str = "Dockerfile";

pub(crate) const BOOTUP_DIR: &str = "./scripts-auto/00bootup";
pub(crate) const BOOTUP_GLOBAL_CFG: &str = "Global.cfg";
pub(crate) const BOOTUP_MODULE_SETUP_SH: &str = "module-setup.sh";
pub(crate) const BOOTUP_MOUNT_SH: &str = "mount.sh";

pub(crate) const ADMIN_CONTAINER_DIR: &str = "./scripts-auto/admin-container";
pub(crate) const ADMIN_DOCKERFILE: &str = "Dockerfile";
pub(crate) const ADMIN_SET_SSH_PUB_KEY_SERVICE: &str = "set-ssh-pub-key.service";
pub(crate) const ADMIN_SET_SSH_PUB_KEY_SH: &str = "set-ssh-pub-key.sh";

pub(crate) const MISC_FILES_DIR: &str = "./scripts-auto/misc-files";
pub(crate) const MISC_BOOT_EFI_MOUNT: &str = "boot-efi.mount";
pub(crate) const MISC_BOOT_GRUB2_MOUNT: &str = "boot-grub2.mount";
pub(crate) const MISC_ETC_MOUNT: &str = "etc.mount";
pub(crate) const MISC_OS_AGENT_SERVICE: &str = "os-agent.service";
pub(crate) const MISC_OS_RELEASE: &str = "os-release";
pub(crate) const MISC_PERSIST_MOUNT: &str = "persist.mount";
pub(crate) const MISC_VAR_MOUNT: &str = "var.mount";

// permissions
pub(crate) const CONFIG_PERMISSION: u32 = 0o640;
pub(crate) const EXEC_PERMISSION: u32 = 0o550;
