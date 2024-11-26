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

use std::{collections::HashMap, io::Write, path::PathBuf};

use anyhow::{anyhow, bail, Result};
use strfmt::strfmt;

use crate::{commands::*, utils, values::*};

pub fn base_gen(file: &mut dyn Write, content: &str, sh: bool) -> Result<()> {
    if sh {
        writeln!(file, "#!/bin/bash")?;
    }
    gen_copyright(file)?;
    writeln!(file, "{}", content)?;
    Ok(())
}

/* copyright */
pub(crate) fn gen_copyright(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{COPYRIGHT}")?;

    Ok(())
}

/* region: kbimg.sh */
pub(crate) fn gen_test_lock(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{TEST_LOCK}")?;
    Ok(())
}

pub(crate) fn gen_cleanup(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{CLEANUP}")?;
    Ok(())
}

pub(crate) fn gen_global_func(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{DELETE_DIR}")?;
    writeln!(file, "{DELETE_FILE}")?;
    writeln!(file, "{LOG}")?;
    gen_test_lock(file)?;
    gen_cleanup(file)?;
    Ok(())
}

pub(crate) fn gen_mount_proc_dev_sys(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{MOUNT_PROC_DEV_SYS}")?;
    Ok(())
}

pub(crate) fn gen_unmount_dir(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{UNMOUNT_DIR}")?;
    Ok(())
}

pub(crate) fn gen_init_partition(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{INIT_PARTITION}")?;
    Ok(())
}

pub(crate) fn write_bootloader(arch: &str, legacy_bios: bool) -> Result<()> {
    let bootloader_path = format!("{}/{}", SCRIPTS_DIR, BOOTLOADER_SH);
    let mut bootloader = std::fs::File::create(&bootloader_path)?;
    gen_bootloader(&mut bootloader, arch, legacy_bios)?;
    utils::set_permissions(&bootloader_path, EXEC_PERMISSION)?;
    Ok(())
}

// repo
pub(crate) fn gen_repo_vars(
    file: &mut dyn Write,
    info: &RepoInfo,
    dm_verity: &Option<DmVerity>,
    grub: &Option<Grub>,
) -> Result<()> {
    writeln!(
        file,
        r#"REPO_PATH="{}"
VERSION="{}"
AGENT_PATH="{}"
# shellcheck disable=SC2016
ROOT_PASSWD='{}'
"#,
        info.repo_path.to_str().unwrap(),
        &info.version,
        info.agent_path.to_str().unwrap(),
        &info.root_passwd,
    )?;
    if info.image_type == Some(ImageType::UpgradeImage) {
        writeln!(file, "DOCKER_IMG=\"{}\"\n", info.upgrade_img.as_ref().unwrap())?;
    }
    if let Some(dm_verity) = dm_verity {
        writeln!(
            file,
            r#"RSApw='{}'
GPGpw='{}'
GRUBpw='{}'
source "${{SCRIPTS_DIR}}"/dm-verity/dm_verity.sh &>/dev/null
"#,
            dm_verity.efi_key,
            dm_verity.grub_key,
            grub.as_ref().unwrap().passwd,
        )?;
    }
    Ok(())
}

pub(crate) fn gen_prepare_yum(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{PREPARE_YUM}")?;
    Ok(())
}

pub(crate) fn gen_install_packages(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{INSTALL_PACKAGES}")?;
    Ok(())
}

pub(crate) fn gen_copy_files(file: &mut dyn Write, copy_files: &Vec<CopyFile>) -> Result<()> {
    writeln!(file, "function copy_files() {{")?;
    for copy_file in copy_files {
        copy_file.gen_copy_files(file)?;
    }
    writeln!(file, "}}\n")?;
    Ok(())
}

pub(crate) fn gen_grub_config(file: &mut dyn Write, legacy_bios: bool, arch: &str, grub: &Grub) -> Result<()> {
    grub.gen_grub_config(file, legacy_bios, arch)?;
    Ok(())
}

pub(crate) fn gen_chroot_script(file: &mut dyn Write, chroot_script: &ChrootScript) -> Result<()> {
    chroot_script.gen_chroot_script(file)?;
    Ok(())
}

pub(crate) fn gen_install_misc(
    file: &mut dyn Write,
    legacy_bios: bool,
    arch: &str,
    image_type: &ImageType,
    config: &Config,
) -> Result<()> {
    if let Some(copy_files) = &config.copy_files {
        gen_copy_files(file, &copy_files)?;
    }
    if let Some(grub) = &config.grub {
        if config.dm_verity.is_none() {
            gen_grub_config(file, legacy_bios, arch, &grub)?;
        }
    }
    if let Some(chroot_script) = &config.chroot_script {
        gen_chroot_script(file, &chroot_script)?;
    }

    let mut vars = HashMap::new();
    if legacy_bios && arch == "x86_64" {
        vars.insert(
            "COPY_GRUB_CFG".to_string(),
            r#"cp "${SCRIPTS_DIR}"/grub.cfg "${RPM_ROOT}"/boot/grub2
    sed -i "s/insmod part_gpt/insmod part_msdos/g; \
    s/set root='hd0,gpt2'/set root='hd0,msdos2'/g; \
    s/set root='hd0,gpt3'/set root='hd0,msdos3'/g" \
    "${RPM_ROOT}"/boot/grub2/grub.cfg"#
                .to_string(),
        );
    } else {
        vars.insert(
            "COPY_GRUB_CFG".to_string(),
            r#"cp "${SCRIPTS_DIR}"/grub.cfg "${RPM_ROOT}"/boot/efi/EFI/openEuler"#.to_string(),
        );
    }

    let mut pxe_bootup_files = String::new();
    if image_type == &ImageType::PxeRepo {
        pxe_bootup_files.push_str(r#"cp -r "${SCRIPTS_DIR}"/00bootup "${RPM_ROOT}"/usr/lib/dracut/modules.d/"#);
    }
    vars.insert("PXE_BOOTUP_FILES".to_string(), pxe_bootup_files);

    let mut dm_verity_files = String::new();
    if config.dm_verity.is_some() {
        dm_verity_files.push_str(
            format!(
                "cp -r \"${{SCRIPTS_DIR}}\"/05dmverity \"${{RPM_ROOT}}\"/usr/lib/dracut/modules.d/\n    cp \"${{SCRIPTS_DIR}}\"/dm-verity/{} \"${{RPM_ROOT}}\"/usr/bin",
                DMV_UPGRADE_ROLLBACK
            )
            .as_str(),
        );
    }
    vars.insert("DM_VERITY_FILES".to_string(), dm_verity_files);

    let mut custom_script = String::new();
    if let Some(_) = &config.copy_files {
        custom_script.push_str("    copy_files\n");
    }
    if let Some(_) = &config.grub {
        if config.dm_verity.is_none() {
            custom_script.push_str("    grub_config\n");
        }
    }
    if let Some(_) = &config.chroot_script {
        custom_script.push_str("    chroot_script\n");
    }
    vars.insert("CUSTOM_SCRIPT".to_string(), custom_script);

    let dynamic_script = strfmt(INSTALL_MISC, &vars)?;
    writeln!(file, "{dynamic_script}")?;
    Ok(())
}

pub(crate) fn gen_create_os_tar_from_repo(file: &mut dyn Write, info: &RepoInfo, config: &Config) -> Result<()> {
    let arch = info.arch.clone().ok_or_else(|| anyhow!("arch is None"))?;
    gen_prepare_yum(file)?;
    gen_install_packages(file)?;
    gen_install_misc(file, info.legacy_bios, &arch, &info.image_type.clone().unwrap(), config)?;

    writeln!(file, "{CREATE_OS_TAR_FROM_REPO}")?;
    Ok(())
}

pub(crate) fn gen_set_partuuid(file: &mut dyn Write, legacy_bios: bool, dm_verity: bool) -> Result<()> {
    if dm_verity {
        return Ok(());
    }
    let mut vars = HashMap::new();
    let (grub_path, root_partuuid): (String, String);
    if legacy_bios {
        grub_path = "/boot/grub2/grub.cfg".to_string();
        root_partuuid = SET_PARTUUID_LEGACY.to_string();
    } else {
        grub_path = "/boot/efi/EFI/openEuler/grub.cfg".to_string();
        root_partuuid = SET_PARTUUID_EFI.to_string();
    }
    vars.insert("GRUB_PATH".to_string(), grub_path);
    vars.insert("ROOT_PARTUUID".to_string(), root_partuuid);
    let dynamic_script = strfmt(SET_PARTUUID, &vars)?;
    writeln!(file, "{dynamic_script}")?;
    Ok(())
}

// gen_create_img create image file from os.tar.
// For legacy bios, use msdos partition table, and for uefi, use gpt partition table
// In the case of dm_verity, there are 7 partitions which are boot1, root1, hash1, boot2, root2, hash2 and persist.
// The partion size relationship between root and hash is 20:1.
// In the case of no dm_verity, there are 4 partitions which are boot, root1, root2 and persist.
pub(crate) fn gen_create_img(file: &mut dyn Write, legacy_bios: bool, config: &Config) -> Result<()> {
    let (img_size, init_boot, partition_info) =
        gen_partition(legacy_bios, config.dm_verity.is_some(), &config.disk_partition)?;
    let mut vars = HashMap::new();

    let mut mkdir_persist: String = String::new();
    if let Some(persist_mkdir) = &config.persist_mkdir {
        for name in &persist_mkdir.name {
            mkdir_persist.push_str(&format!("    mkdir -p \"${{TMP_MOUNT_PATH}}\"/{}\n", name));
        }
    }
    let init_rootb = format!(
        "init_part \"${{SCRIPTS_DIR}}\"/system.img{} ROOT-B \"${{TMP_MOUNT_PATH}}\"",
        if config.dm_verity.is_some() { "5" } else { "3" }
    );
    let init_persist = format!(
        "init_part \"${{SCRIPTS_DIR}}\"/system.img{} PERSIST \"${{TMP_MOUNT_PATH}}\"",
        if config.dm_verity.is_some() { "7" } else { "4" }
    );
    let mut dmv_main = String::new();
    let mut set_partuuid = String::from(r#"set_partuuid "${TMP_MOUNT_PATH}""#);
    if config.dm_verity.is_some() {
        let keys_dir = if let Some(p) = &config.dm_verity.as_ref().unwrap().keys_dir {
            if !p.exists() {
                bail!("dm_verity keys_dir does not exist: {}", p.to_str().unwrap());
            }
            let canonical_path = p.as_path().canonicalize()?;
            let canonical_str =
                canonical_path.to_str().ok_or_else(|| anyhow!("Failed to convert canonicalized path to string"))?;
            canonical_str.to_string()
        } else {
            String::new()
        };
        dmv_main = format!(
            r#"rm -f "${{SCRIPTS_DIR}}"/update.img
    {}dmvmain "${{RSApw}}" "${{GPGpw}}" "${{GRUBpw}}""#,
            if keys_dir.is_empty() {
                "".to_string()
            } else {
                format!("KEYDIR={} CERTDB={}/certdb ", keys_dir, keys_dir)
            }
        );
        set_partuuid = String::new();
    }

    vars.insert("IMG_SIZE".to_string(), img_size.to_string());
    vars.insert("PARTITIONS".to_string(), partition_info);
    vars.insert("INIT_BOOT".to_string(), init_boot);
    vars.insert("SET_PARTUUID".to_string(), set_partuuid);
    vars.insert("INIT_ROOTB".to_string(), init_rootb);
    vars.insert("INIT_PERSIST".to_string(), init_persist);
    vars.insert("MKDIR_PERSIST".to_string(), mkdir_persist);
    vars.insert("DMV_MAIN".to_string(), dmv_main);
    let dynamic_script = strfmt(CREATE_IMAGE, &vars)?;

    writeln!(file, "{dynamic_script}")?;
    Ok(())
}

pub(crate) fn gen_partition(
    legacy_bios: bool,
    dm_verity: bool,
    disk_partition: &Option<DiskPartition>,
) -> Result<(u32, String, String)> {
    let img_size = disk_partition.as_ref().and_then(|dp| dp.img_size).unwrap_or(IMAGE_SIZE);

    let init_boot = setup_init_boot(dm_verity, legacy_bios);
    let partition_info = if dm_verity {
        create_dm_verity_partitions(disk_partition, img_size)?
    } else {
        create_standard_partitions(legacy_bios, disk_partition, img_size)?
    };
    Ok((img_size, init_boot, partition_info))
}

fn setup_init_boot(dm_verity: bool, legacy_bios: bool) -> String {
    if dm_verity || !legacy_bios {
        r#"init_part "${SCRIPTS_DIR}"/system.img1 BOOT "${BOOT_PATH}""#.to_string()
    } else {
        r#"init_part "${SCRIPTS_DIR}"/system.img1 GRUB2 "${BOOT_PATH}""#.to_string()
    }
}

fn create_dm_verity_partitions(disk_partition: &Option<DiskPartition>, img_size: u32) -> Result<String> {
    let sizes = calculate_dm_verity_partition_sizes(disk_partition, img_size)?;
    let mut partition_info = format!(
        r#"local BOOT_PATH=${{TMP_MOUNT_PATH}}/boot/efi
    parted "${{SCRIPTS_DIR}}/system.img" -s mklabel gpt
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary fat32 1MiB {}MiB"#,
        sizes[0]
    );
    for i in 0..(sizes.len() - 1) {
        partition_info.push_str(&format!(
            r#"
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary {} {}MiB {}MiB"#,
            if i == 2 { "fat32" } else { "ext4" },
            sizes[i],
            sizes[i + 1]
        ));
    }
    partition_info.push_str(&format!(
        r#"
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary ext4 {}MiB 100%"#,
        sizes[sizes.len() - 1]
    ));
    Ok(partition_info)
}

fn create_standard_partitions(
    legacy_bios: bool,
    disk_partition: &Option<DiskPartition>,
    img_size: u32,
) -> Result<String> {
    let sizes = calculate_standard_partition_sizes(disk_partition, img_size)?;
    let label = if legacy_bios { "msdos" } else { "gpt" };
    let boot_path = if legacy_bios { "grub2" } else { "efi" };

    let mut partition_info = format!(
        r#"local BOOT_PATH=${{TMP_MOUNT_PATH}}/boot/{}
    parted "${{SCRIPTS_DIR}}/system.img" -s mklabel {}
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary {} 1MiB {}MiB"#,
        boot_path,
        label,
        if legacy_bios { "ext4" } else { "fat32" },
        sizes[0]
    );
    for i in 0..(sizes.len() - 1) {
        partition_info.push_str(&format!(
            r#"
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary ext4 {}MiB {}MiB"#,
            sizes[i],
            sizes[i + 1]
        ));
    }
    partition_info.push_str(&format!(
        r#"
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary ext4 {}MiB 100%"#,
        sizes[sizes.len() - 1]
    ));
    Ok(partition_info)
}

fn calculate_dm_verity_partition_sizes(disk_partition: &Option<DiskPartition>, img_size: u32) -> Result<Vec<u32>> {
    let base_sizes = if let Some(p) = disk_partition {
        vec![BOOT_SIZE, p.root, p.root / 20, BOOT_SIZE, p.root, p.root / 20]
    } else {
        vec![BOOT_SIZE, ROOT_SIZE, HASH_SIZE, BOOT_SIZE, ROOT_SIZE, HASH_SIZE]
    };

    let cumulative_sizes = compute_cumulative_sizes(&base_sizes, img_size)?;
    Ok(cumulative_sizes)
}

fn calculate_standard_partition_sizes(disk_partition: &Option<DiskPartition>, img_size: u32) -> Result<Vec<u32>> {
    let base_sizes = if let Some(p) = disk_partition {
        vec![BOOT_SIZE, p.root, p.root]
    } else {
        vec![BOOT_SIZE, ROOT_SIZE, ROOT_SIZE]
    };

    let cumulative_sizes = compute_cumulative_sizes(&base_sizes, img_size)?;
    Ok(cumulative_sizes)
}

fn compute_cumulative_sizes(base_sizes: &[u32], img_size: u32) -> Result<Vec<u32>> {
    let cumulative_sizes: Vec<u32> = base_sizes
        .iter()
        .scan(0, |acc, &p_size| {
            *acc += p_size;
            Some(*acc)
        })
        .collect();

    if cumulative_sizes.last().unwrap_or(&0) + PERSIST_SIZE > img_size * 1024 {
        bail!("Image size({}G) is not enough for partitions, please check input", img_size);
    }
    Ok(cumulative_sizes)
}

pub(crate) fn gen_create_vm_repo_img(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{CREATE_VM_REPO_IMAGE}")?;
    Ok(())
}

pub(crate) fn gen_create_pxe_repo_img(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{CREATE_PXE_REPO_IMAGE}")?;
    Ok(())
}

pub(crate) fn gen_create_docker_img(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{CREATE_DOCKER_IMAGE}")?;
    Ok(())
}

// docker
pub(crate) fn gen_docker_vars(file: &mut dyn Write, image_name: &str) -> Result<()> {
    writeln!(
        file,
        r#"
DOCKER_IMG="{}"
"#,
        image_name
    )?;
    Ok(())
}

pub(crate) fn gen_create_os_tar_from_docker(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{CREATE_OS_TAR_FROM_DOCKER}")?;
    Ok(())
}

pub(crate) fn gen_create_vm_docker_img(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{CREATE_VM_DOCKER_IMAGE}")?;
    Ok(())
}

pub(crate) fn gen_create_pxe_docker_img(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{CREATE_PXE_DOCKER_IMAGE}")?;
    Ok(())
}

// admin
pub(crate) fn gen_admin_vars(file: &mut dyn Write, docker_img: &str, hostshell: &PathBuf) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;

    writeln!(
        file,
        r#"set -eux

SCRIPTS_DIR=$(dirname "$0")
LOCK="${{SCRIPTS_DIR}}"/test.lock
ADMIN_CONTAINER_DIR="${{SCRIPTS_DIR}}"/admin-container
DOCKER_IMG={}
DOCKERFILE="${{ADMIN_CONTAINER_DIR}}"/Dockerfile
HOSTSHELL={}
"#,
        docker_img,
        hostshell.to_str().unwrap()
    )?;
    Ok(())
}

pub(crate) fn gen_create_admin_img(file: &mut dyn Write) -> Result<()> {
    writeln!(file, "{CREATE_ADMIN_IMAGE}")?;
    Ok(())
}
/* endregion */

/* region: set_in_chroot.sh */
pub(crate) fn gen_add_users(file: &mut dyn Write, users: &Vec<User>) -> Result<()> {
    writeln!(file, "# add users")?;
    writeln!(file, r#"sed -i 's/^CREATE_MAIL_SPOOL=yes/CREATE_MAIL_SPOOL=no/' /etc/default/useradd"#)?;
    for user in users {
        user.gen_add_users(file)?;
    }
    Ok(())
}

pub(crate) fn gen_systemd_services(file: &mut dyn Write, systemd_services: &SystemdService) -> Result<()> {
    writeln!(file, "# systemd")?;
    for service_name in &systemd_services.name {
        writeln!(file, "systemctl enable {}", service_name)?;
    }
    Ok(())
}

pub(crate) fn gen_set_in_chroot(
    file: &mut dyn Write,
    legacy_bios: bool,
    arch: &str,
    image_type: &ImageType,
    config: &Config,
) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;

    let mut vars = HashMap::new();
    if legacy_bios && arch == "x86_64" {
        vars.insert(
            "BOOT_MOUNT_ENABLE".to_string(),
            "ln -s /usr/lib/systemd/system/boot-grub2.mount /lib/systemd/system/local-fs.target.wants/boot-grub2.mount"
                .to_string(),
        );
    } else {
        vars.insert(
            "BOOT_MOUNT_ENABLE".to_string(),
            "ln -s /usr/lib/systemd/system/boot-efi.mount /lib/systemd/system/local-fs.target.wants/boot-efi.mount"
                .to_string(),
        );
    }
    let mut pxe_dracut = String::new();
    if image_type == &ImageType::PxeRepo {
        pxe_dracut = r#"dracut -f -v --add bootup /initramfs.img --kver "$(ls /lib/modules)"  # added in pxe case
rm -rf /usr/lib/dracut/modules.d/00bootup"#
            .to_string();
    }
    vars.insert("PXE_DRACUT".into(), pxe_dracut);

    let mut dm_verity_dracut = String::new();
    if config.dm_verity.is_some() {
        dm_verity_dracut = r#"if [ -d "/usr/lib/dracut/modules.d/05dmverity" ]; then
    dracut -f -v --add dmverity /boot/initramfs-verity.img --kver "$(ls /lib/modules)"
    rm -rf /usr/lib/dracut/modules.d/05dmverity
fi
grub2-editenv /boot/efi/EFI/openEuler/grubenv create"#
            .to_string();
    }
    vars.insert("DM_VERITY_DRACUT".into(), dm_verity_dracut);
    let dynamic_script = strfmt(SET_IN_CHROOT, &vars)?;
    writeln!(file, "{dynamic_script}")?;

    if let Some(users) = &config.users {
        gen_add_users(file, users)?;
    }
    if let Some(systemd_services) = &config.systemd_service {
        gen_systemd_services(file, systemd_services)?;
    }
    Ok(())
}
/* endregion */

/* region: bootloader.sh */
pub(crate) fn gen_bootloader(file: &mut dyn Write, arch: &str, legacy_bios: bool) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;

    writeln!(
        file,
        r#"set -eux
set -o pipefail"#
    )?;
    match arch {
        "x86_64" => {
            if legacy_bios {
                writeln!(file, "{BOOT_LOADER_LEGACY}")?;
            } else {
                writeln!(file, "{BOOT_LOADER_X86_UEFI}")?;
            }
        },
        "aarch64" => {
            writeln!(file, "{BOOT_LOADER_AARCH64}")?;
        },
        _ => bail!("Unsupported architecture: {}", arch),
    }

    Ok(())
}
/* endregion */

/* region: rpmlist */
pub(crate) fn gen_rpm_list(
    file: &mut dyn Write,
    rpmlist: &Vec<String>,
    arch: &str,
    legacy_bios: bool,
    dm_verity: bool,
) -> Result<()> {
    for rpm in rpmlist {
        writeln!(file, "{}", rpm)?;
    }
    match arch {
        "x86_64" => {
            if legacy_bios {
                writeln!(file, "grub2")?;
            } else {
                writeln!(file, "grub2-efi\ngrub2-tools\ngrub2-efi-x64-modules\ngrub2-pc-modules")?;
                if dm_verity {
                    writeln!(file, "efibootmgr\nveritysetup\nshim\nmokutil\ngrub2-efi-x64")?;
                }
            }
        },
        "aarch64" => {
            writeln!(file, "grub2-efi\ngrub2-tools\ngrub2-efi-aa64-modules")?;
            if dm_verity {
                writeln!(file, "efibootmgr\nveritysetup\nshim\nmokutil\ngrub2-efi-aa64")?;
            }
        },
        _ => bail!("Unsupported architecture: {}", arch),
    }
    Ok(())
}
/* endregion */

/* region: 00bootup */
// 00bootup/global.cfg
pub(crate) fn gen_global_cfg(file: &mut dyn Write, config: &PxeConfig) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;
    writeln!(
        file,
        r#"rootfs_name={}
# select the target disk to install kubeOS
disk={}
# pxe server ip address where stores the rootfs on the http server
server_ip={}
route_ip={}"#,
        config.rootfs_name, config.disk, config.server_ip, config.route_ip,
    )?;
    if config.dhcp.unwrap_or(false) {
        writeln!(file, "dhcs=/dhclient-script",)?;
    } else {
        writeln!(
            file,
            "local_ip={}\nnet_name={}\nnetmask={}\n",
            config.local_ip.as_ref().unwrap(),
            config.net_name.as_ref().unwrap(),
            config.netmask.as_ref().unwrap(),
        )?;
    }
    Ok(())
}

// 00bootup/mount.sh
pub(crate) fn gen_mount(file: &mut dyn Write, config: &Config) -> Result<()> {
    let mut mkdir_args = String::from("mkdir /sysroot/persist/{var,etc,etcwork,opt,optwork");
    if let Some(persist_mkdir) = &config.persist_mkdir {
        for name in &persist_mkdir.name {
            mkdir_args.push_str(&format!(",{}", name));
        }
    }
    mkdir_args.push('}');

    let (first, second, third) = if let Some(disk_partition) = &config.disk_partition {
        let first = BOOT_SIZE;
        let second = disk_partition.root;
        let third = disk_partition.root;
        (first, first + second, first + second + third)
    } else {
        (BOOT_SIZE, BOOT_SIZE + ROOT_SIZE, BOOT_SIZE + 2 * ROOT_SIZE)
    };

    let mut vars = HashMap::new();
    vars.insert("MKDIR_COMMAND".to_string(), mkdir_args);
    vars.insert("PARTITION1_SIZE".to_string(), first.to_string());
    vars.insert("PARTITION2_SIZE".to_string(), second.to_string());
    vars.insert("PARTITION3_SIZE".to_string(), third.to_string());

    if config.pxe_config.as_ref().unwrap().dhcp.unwrap_or(false) {
        vars.insert("SET_IP".to_string(), DHCP_SET_IP.to_string());
        vars.insert("MANUAL_GET_IF_NAME".to_string(), "".to_string());
    } else {
        vars.insert("SET_IP".to_string(), MANUAL_SET_IP.to_string());
        vars.insert("MANUAL_GET_IF_NAME".to_string(), MANUAL_GET_IF_NAME.to_string());
    }
    let dynamic_script = strfmt(INIT_NETWORK_PARTITION, &vars)?;

    writeln!(file, "{BOOTUP_MOUNT_1}")?;
    writeln!(file, "{dynamic_script}")?;
    writeln!(file, "{BOOTUP_MOUNT_2}")?;
    Ok(())
}
/* endregion */

/* region: misc-files */
// misc-files/os-release
pub(crate) fn gen_os_release(file: &mut dyn Write) -> Result<()> {
    writeln!(
        file,
        r#"NAME=KubeOS
ID=KubeOS
"#
    )?;
    Ok(())
}
/* endregion */

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use strfmt::strfmt;

    #[test]
    fn test_strfmt() {
        let mystring = r#"
        function {{
            Hello {name}, {age} years old.
        }}
        "#;
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "John".to_string());
        vars.insert("age".to_string(), "30".to_string());
        let result = strfmt(mystring, &vars).unwrap();
        println!("{}", result);
    }
}
