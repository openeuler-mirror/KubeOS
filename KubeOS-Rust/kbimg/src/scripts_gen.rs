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

use std::{fs::File, io::Write, path::PathBuf};

use anyhow::{bail, Ok, Result};

use crate::{commands::*, values::SCRIPTS_DIR};

/* copyright */
pub(crate) fn gen_copyright(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"## Copyright (c) Huawei Technologies Co., Ltd. 2022. All rights reserved.
# KubeOS is licensed under the Mulan PSL v2.
# You can use this software according to the terms and conditions of the Mulan PSL v2.
# You may obtain a copy of Mulan PSL v2 at:
#     http://license.coscl.org.cn/MulanPSL2
# THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
# PURPOSE.
## See the Mulan PSL v2 for more details.
"#
    )?;

    Ok(())
}

/* region: kbimg.sh */
pub(crate) fn gen_global_vars(file: &mut File) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;

    writeln!(
        file,
        r#"set -e

SCRIPTS_DIR=$(cd "$(dirname "$0")" && pwd)
LOCK="${{SCRIPTS_DIR}}"/test.lock
RPM_ROOT="${{SCRIPTS_DIR}}"/rootfs
TMP_MOUNT_PATH="${{SCRIPTS_DIR}}"/mnt
"#
    )?;
    Ok(())
}

pub(crate) fn gen_global_func(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function delete_dir() {{
	local ret=0
	local dir="$1"
	unmount_dir "${{dir}}"
	ret=$?
	if [ "${{ret}}" -eq 0 ]; then
        rm -rf "${{dir}}"
        return 0
	else
        log_error_print "${{dir}} is failed to unmount , can not delete ${{dir}}."
        return 1
	fi
}}

function delete_file() {{
	local file="$1"
	if [ ! -e "${{file}}" ]; then
        return 0
	fi

	if [ ! -f "${{file}}" ]; then
        log_error_print "${{file}} is not a file."
        return 1
	fi

	rm -f "${{file}}"
	return 0
}}

function clean_space() {{
    delete_dir "${{RPM_ROOT}}"
    delete_dir "${{TMP_MOUNT_PATH}}"
    delete_file "${{SCRIPTS_DIR}}"/os.tar
    rm -rf "${{LOCK}}"
    delete_file "${{ADMIN_CONTAINER_DIR}}"/hostshell
}}

function clean_img() {{
    delete_file "${{SCRIPTS_DIR}}"/system.img
    delete_file "${{SCRIPTS_DIR}}"/update.img
    delete_file "${{SCRIPTS_DIR}}"/initramfs.img
    delete_file "${{SCRIPTS_DIR}}"/kubeos.tar
}}

function file_lock() {{
    local lock_file=$1
    exec {{lock_fd}}>"${{lock_file}}"
    flock -xn "${{lock_fd}}"
}}

function test_lock() {{
    file_lock "${{LOCK}}"
    local status=$?
    if [ $status -ne 0 ]; then
        log_error_print "There is already an generate process running."
        exit 203
    fi
}}

function log_error_print() {{
    local logmsg
    logmsg="[ ERROR ] - ""$(date "+%b %d %Y %H:%M:%S")"" $1"
    echo "$logmsg"
}}

function log_info_print() {{
    local logmsg
    logmsg="[ INFO ] - ""$(date "+%b %d %Y %H:%M:%S")"" $1"
    echo "$logmsg"
}}
"#
    )?;
    Ok(())
}

pub(crate) fn gen_mount_proc_dev_sys(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function mount_proc_dev_sys() {{
	local tmp_root=$1
	mount -t proc none "${{tmp_root}}"/proc
	mount --bind /dev "${{tmp_root}}"/dev
	mount --bind /dev/pts "${{tmp_root}}"/dev/pts
	mount -t sysfs none "${{tmp_root}}"/sys
}}
"#
    )?;
    Ok(())
}

pub(crate) fn gen_unmount_dir(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function unmount_dir() {{
	local dir=$1

	if [ -L "${{dir}}" ] || [ -f "${{dir}}" ]; then
        log_error_print "${{dir}} is not a directory, please check it."
        return 1
	fi

	if [ ! -d "${{dir}}" ]; then
        return 0
	fi

	local real_dir
	real_dir=$(readlink -e "${{dir}}")
	local mnts
	mnts=$(awk '{{print $2}}' < /proc/mounts | grep "^${{real_dir}}" | sort -r)
	for m in ${{mnts}}; do
        log_info_print "Unmount ${{m}}"
        umount -f "${{m}}" || true
	done

	return 0
}}
"#
    )?;
    Ok(())
}

pub(crate) fn gen_init_part(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function init_part() {{
	local offset
	offset=$(fdisk -l "${{SCRIPTS_DIR}}"/system.img | grep "$1" | awk '{{print $2}}')
	local sizelimit
	sizelimit=$(fdisk -l "${{SCRIPTS_DIR}}"/system.img | grep "$1" | awk '{{print $3}}')
	sizelimit=$(echo "($sizelimit - $offset)*512" | bc)
	offset=$(echo "${{offset}}*512" | bc)
	local loop
	loop=$(losetup -f)
	losetup -o "${{offset}}" --sizelimit "${{sizelimit}}" "${{loop}}" "${{SCRIPTS_DIR}}"/system.img
	if [ "$2" == "BOOT" ];then
        mkfs.vfat -n "$2" "${{loop}}"
        mount -t vfat "${{loop}}" "$3"
	else
        mkfs.ext4 -L "$2" "${{loop}}"
        mount -t ext4 "${{loop}}" "$3"
        rm -rf "$3/lost+found"
	fi
}}
"#
    )?;
    Ok(())
}

// repo
pub(crate) fn gen_repo_vars(file: &mut File, info: &RepoInfo) -> Result<()> {
    writeln!(
        file,
        r#"REPO_PATH="{}"
VERSION="{}"
AGENT_PATH="{}"
ROOT_PASSWD='{}'
BOOT_MODE="{}"
"#,
        info.repo_path.to_str().unwrap(),
        &info.version,
        info.agent_path.to_str().unwrap(),
        &info.root_passwd,
        if info.legacy_bios { "legacy" } else { "efi" }
    )?;
    if let Some(docker_img) = &info.docker_img {
        writeln!(file, "DOCKER_IMG=\"{}\"\n", docker_img)?;
    }
    Ok(())
}

pub(crate) fn gen_prepare_yum(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function prepare_yum() {{
    # init rpmdb
    rpm --root "${{RPM_ROOT}}" --initdb
    mkdir -p "${{RPM_ROOT}}"{{/etc/yum.repos.d,/persist,/proc,/dev/pts,/sys}}
    mount_proc_dev_sys "${{RPM_ROOT}}"
    # init yum repo
    local iso_repo="${{RPM_ROOT}}"/etc/yum.repos.d/iso.repo
    cat "${{REPO_PATH}}" > "$iso_repo"
}}
"#
    )?;
    Ok(())
}

pub(crate) fn gen_install_packages(file: &mut File, arch: &str, legacy_bios: bool) -> Result<()> {
    writeln!(
        file,
        r#"function install_packages() {{
    prepare_yum "${{REPO_PATH}}"

    echo "install package.."

    local filesize
    filesize=$(stat -c "%s" "${{SCRIPTS_DIR}}"/rpmlist)
    local maxsize=$((1024*1024))
    if [ "${{filesize}}" -gt "${{maxsize}}" ]; then
        echo "please check if rpmlist is too big or something wrong"
        exit 7
    fi

    local rpms_name
    rpms_name=$(tr "\n" " " < "${{SCRIPTS_DIR}}"/rpmlist)
    old_ifs="$IFS"
    IFS=' '"#
    )?;

    if arch == "x86_64" {
        if legacy_bios {
            writeln!(file, "\trpms_name+=\" grub2\"")?;
        } else {
            writeln!(file, "\trpms_name+=\" grub2-efi grub2-tools grub2-efi-x64-modules grub2-pc-modules\"")?;
        }
        writeln!(
            file,
            r#"    read -ra rpms <<< "${{rpms_name}}"
    IFS="$old_ifs"
    yum -y --installroot="${{RPM_ROOT}}" install --nogpgcheck --setopt install_weak_deps=False "${{rpms[@]}}""#
        )?;
    } else if arch == "aarch64" {
        writeln!(
            file,
            r#"        read -ra rpms <<< "${{rpms_name}}"
        IFS="$old_ifs"
        yum -y --installroot="${{RPM_ROOT}}" install --nogpgcheck --setopt install_weak_deps=False "${{rpms[@]}}" grub2-efi grub2-tools grub2-efi-aa64-modules"#
        )?;
    }
    writeln!(
        file,
        r#"    yum -y --installroot="${{RPM_ROOT}}" clean all
}}
"#
    )?;
    Ok(())
}

pub(crate) fn gen_copy_files(file: &mut File, copy_files: &Vec<CopyFile>) -> Result<()> {
    writeln!(file, "function copy_files() {{")?;
    for copy_file in copy_files {
        let dst = format!("{}/rootfs{}", SCRIPTS_DIR, &copy_file.dst);
        let dst = PathBuf::from(dst);
        if !dst.exists() {
            writeln!(file, "\tmkdir -p \"${{RPM_ROOT}}{}\"", &copy_file.dst)?;
        }
        let src = PathBuf::from(&copy_file.src);
        if src.is_dir() {
            writeln!(file, "\tcp -r {} \"${{RPM_ROOT}}{}\"", &copy_file.src, &copy_file.dst)?;
        } else {
            writeln!(file, "\tcp {} \"${{RPM_ROOT}}{}\"", &copy_file.src, &copy_file.dst)?;
        }
    }
    writeln!(file, "}}\n")?;
    Ok(())
}

pub(crate) fn gen_grub_config(file: &mut File, legacy_bios: bool, grub: &Grub) -> Result<()> {
    writeln!(
        file,
        r#"function grub_config() {{
    local GRUB_PATH"#
    )?;
    if legacy_bios {
        writeln!(file, "\tGRUB_PATH=\"${{RPM_ROOT}}\"/boot/grub2")?;
    } else {
        writeln!(file, "\tGRUB_PATH=\"${{RPM_ROOT}}\"/boot/efi/EFI/openEuler")?;
    }
    if let Some(grub_passwd) = &grub.passwd {
        writeln!(
            file,
            r#"    local GRUB_PASSWD
    GRUB_PASSWD=$(echo -e "{}\n{}" | grub2-mkpasswd-pbkdf2 | grep PBKDF2 | awk '{{print $7}}')
    echo "GRUB2_PASSWD=${{GRUB_PASSWD}}" > "${{GRUB_PATH}}"/user.cfg
    chmod 600 "${{GRUB_PATH}}"/user.cfg
}}
"#,
            grub_passwd, grub_passwd
        )?;
    }
    Ok(())
}

pub(crate) fn gen_chroot_script(file: &mut File, chroot_script: &ChrootScript) -> Result<()> {
    let script_path = PathBuf::from(&chroot_script.path);
    match script_path.canonicalize() {
        core::result::Result::Ok(absolute_path) => {
            if let Some(script_name) = absolute_path.file_name() {
                writeln!(
                    file,
                    r#"function chroot_script() {{
    cp "{}" "${{RPM_ROOT}}"
    chroot "${{RPM_ROOT}}" bash /{}
}}
"#,
                    absolute_path.as_path().to_str().unwrap(),
                    script_name.to_str().unwrap()
                )?;
            }
            Ok(())
        },
        Err(e) => bail!(e),
    }
}

pub(crate) fn gen_install_misc(file: &mut File, legacy_bios: bool, config: &Config) -> Result<()> {
    if let Some(copy_files) = &config.copy_files {
        gen_copy_files(file, &copy_files)?;
    }
    if let Some(grub) = &config.grub {
        gen_grub_config(file, legacy_bios, &grub)?;
    }
    if let Some(chroot_script) = &config.chroot_script {
        gen_chroot_script(file, &chroot_script)?;
    }

    writeln!(
        file,
        r#"function install_misc() {{
    cp "${{SCRIPTS_DIR}}"/misc-files/*mount "${{SCRIPTS_DIR}}"/misc-files/os-agent.service "${{RPM_ROOT}}"/usr/lib/systemd/system/
    cp "${{SCRIPTS_DIR}}"/misc-files/os-release "${{RPM_ROOT}}"/usr/lib/
    cp "${{AGENT_PATH}}" "${{RPM_ROOT}}"/usr/bin
    rm "${{RPM_ROOT}}"/etc/os-release

    cat <<EOF > "${{RPM_ROOT}}"/usr/lib/os-release
NAME=${{NAME}}
ID=${{NAME}}
EOF
    echo "PRETTY_NAME=\"${{NAME}} ${{VERSION}}\"" >> "${{RPM_ROOT}}"/usr/lib/os-release
    echo "VERSION_ID=${{VERSION}}" >> "${{RPM_ROOT}}"/usr/lib/os-release
    mv "${{RPM_ROOT}}"/boot/vmlinuz* "${{RPM_ROOT}}"/boot/vmlinuz
    mv "${{RPM_ROOT}}"/boot/initramfs* "${{RPM_ROOT}}"/boot/initramfs.img"#
    )?;

    if legacy_bios {
        writeln!(
            file,
            r#"    cp "${{SCRIPTS_DIR}}"/grub.cfg "${{RPM_ROOT}}"/boot/grub2
    sed -i "s/insmod part_gpt/insmod part_msdos/g; \
s/set root='hd0,gpt2'/set root='hd0,msdos2'/g; \
s/set root='hd0,gpt3'/set root='hd0,msdos3'/g" \
"${{RPM_ROOT}}"/boot/grub2/grub.cfg"#
        )?;
    } else {
        writeln!(file, "\tcp \"${{SCRIPTS_DIR}}\"/grub.cfg \"${{RPM_ROOT}}\"/boot/efi/EFI/openEuler")?;
    }

    writeln!(
        file,
        r#"    cp -r "${{SCRIPTS_DIR}}"/00bootup "${{RPM_ROOT}}"/usr/lib/dracut/modules.d/ 
    cp "${{SCRIPTS_DIR}}"/set_in_chroot.sh "${{RPM_ROOT}}"
    
    # (optional) custom config"#
    )?;

    if let Some(_) = &config.copy_files {
        writeln!(file, "\tcopy_files")?;
    }
    if let Some(_) = &config.grub {
        writeln!(file, "\tgrub_config")?;
    }
    if let Some(_) = &config.chroot_script {
        writeln!(file, "\tchroot_script")?;
    }

    writeln!(
        file,
        r#"
    ROOT_PASSWD="${{ROOT_PASSWD}}" BOOT_MODE="${{BOOT_MODE}}" chroot "${{RPM_ROOT}}" bash /set_in_chroot.sh
    rm "${{RPM_ROOT}}/set_in_chroot.sh"
}}
"#
    )?;
    Ok(())
}

pub(crate) fn gen_create_os_tar_from_repo(file: &mut File, info: &RepoInfo, config: &Config) -> Result<()> {
    gen_prepare_yum(file)?;
    gen_install_packages(file, info.arch.as_ref().unwrap(), info.legacy_bios)?;
    gen_install_misc(file, info.legacy_bios, config)?;

    writeln!(
        file,
        r#"function create_os_tar_from_repo() {{
    install_packages
    install_misc
    unmount_dir "${{RPM_ROOT}}"
    tar -C "${{RPM_ROOT}}" -cf "${{SCRIPTS_DIR}}"/os.tar .
}}
"#
    )?;
    Ok(())
}

pub(crate) fn gen_create_img(file: &mut File, legacy_bios: bool, config: &Config) -> Result<()> {
    let (first, second, third, img_size) = if let Some(disk_partition) = &config.disk_partition {
        let first = disk_partition.first;
        let second = disk_partition.second;
        let third = disk_partition.third;
        let img_size = disk_partition.img_size;
        if first + second + third + 2100 > img_size * 1024 {
            bail!("Image size({}G) is not enough for partitions, please check input", img_size)
        }
        (first, first + second, first + second + third, img_size)
    } else {
        (60, 2160, 4260, 20)
    };

    writeln!(
        file,
        r#"function create_img() {{
    rm -f "${{SCRIPTS_DIR}}"/system.img "${{SCRIPTS_DIR}}/update.img"
    qemu-img create "${{SCRIPTS_DIR}}/system.img" {}G"#,
        img_size
    )?;

    if legacy_bios {
        writeln!(
            file,
            r#"    local BOOT_PATH=${{TMP_MOUNT_PATH}}/boot/grub2
    parted "${{SCRIPTS_DIR}}/system.img" -s mklabel msdos
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary ext4 1MiB {}MiB"#,
            first
        )?;
    } else {
        writeln!(
            file,
            r#"    local BOOT_PATH=${{TMP_MOUNT_PATH}}/boot/efi
    parted "${{SCRIPTS_DIR}}/system.img" -s mklabel gpt
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary fat32 1MiB {}MiB"#,
            first
        )?;
    }

    writeln!(
        file,
        r#"    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary ext4 {}MiB {}MiB
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary ext4 {}MiB {}MiB
    parted "${{SCRIPTS_DIR}}/system.img" -s mkpart primary ext4 {}MiB 100%"#,
        first, second, second, third, third
    )?;

    writeln!(
        file,
        r#"    local device
    device=$(losetup -f)
    losetup "${{device}}" "${{SCRIPTS_DIR}}"/system.img

    mkdir -p "${{TMP_MOUNT_PATH}}"

    init_part "${{SCRIPTS_DIR}}"/system.img2 ROOT-A "${{TMP_MOUNT_PATH}}"
    
    mkdir -p "${{BOOT_PATH}}"
    chmod 755 "${{BOOT_PATH}}""#
    )?;

    if legacy_bios {
        writeln!(
            file,
            r#"    init_part "${{SCRIPTS_DIR}}"/system.img1 GRUB2 "${{BOOT_PATH}}"
    tar -x -C "${{TMP_MOUNT_PATH}}" -f "${{SCRIPTS_DIR}}"/os.tar
    sed -i "s/insmod part_gpt/insmod part_msdos/g; \
s/set root='hd0,gpt2'/set root='hd0,msdos2'/g; \
s/set root='hd0,gpt3'/set root='hd0,msdos3'/g" \
"${{TMP_MOUNT_PATH}}"/boot/grub2/grub.cfg"#
        )?;
    } else {
        writeln!(
            file,
            r#"    init_part "${{SCRIPTS_DIR}}"/system.img1 BOOT "${{BOOT_PATH}}"
    tar -x -C "${{TMP_MOUNT_PATH}}" -f "${{SCRIPTS_DIR}}"/os.tar"#
        )?;
    }

    writeln!(
        file,
        r#"    sync
    cp "${{SCRIPTS_DIR}}"/bootloader.sh "${{TMP_MOUNT_PATH}}"
    mount_proc_dev_sys "${{TMP_MOUNT_PATH}}"
    DEVICE="${{device}}" BOOT_MODE="${{BOOT_MODE}}" chroot "${{TMP_MOUNT_PATH}}" bash bootloader.sh
    rm -rf "${{TMP_MOUNT_PATH}}"/bootloader.sh
    sync

    dd if=/dev/disk/by-label/ROOT-A of="${{SCRIPTS_DIR}}"/update.img bs=8M
    sync
    unmount_dir "${{TMP_MOUNT_PATH}}"
    init_part "${{SCRIPTS_DIR}}"/system.img3 ROOT-B "${{TMP_MOUNT_PATH}}"
    umount "${{TMP_MOUNT_PATH}}"

    init_part "${{SCRIPTS_DIR}}"/system.img4 PERSIST "${{TMP_MOUNT_PATH}}"
    mkdir "${{TMP_MOUNT_PATH}}"/{{var,etc,etcwork}}"#
    )?;

    if let Some(persist_mkdir) = &config.persist_mkdir {
        for name in &persist_mkdir.name {
            writeln!(file, "\tmkdir \"${{TMP_MOUNT_PATH}}\"/{}", name)?;
        }
    }

    writeln!(
        file,
        r#"    mkdir -p "${{TMP_MOUNT_PATH}}"/etc/KubeOS/certs
    umount "${{TMP_MOUNT_PATH}}"

    losetup -D
    parted "${{SCRIPTS_DIR}}"/system.img -- set 1 boot on
    qemu-img convert "${{SCRIPTS_DIR}}"/system.img -O qcow2 "${{SCRIPTS_DIR}}"/system.qcow2
}}
"#
    )?;
    Ok(())
}

pub(crate) fn gen_create_vm_repo_img(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function create_vm_repo_img() {{
	create_os_tar_from_repo
	create_img
}}
    
test_lock
trap clean_space EXIT
trap clean_img ERR

create_vm_repo_img"#
    )?;
    Ok(())
}

pub(crate) fn gen_create_pxe_repo_img(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function create_pxe_repo_img() {{
    rm -rf "${{SCRIPTS_DIR}}"/initramfs.img "${{SCRIPTS_DIR}}"/kubeos.tar
	create_os_tar_from_repo
	tar -xvf "${{SCRIPTS_DIR}}"/os.tar "${{SCRIPTS_DIR}}"/initramfs.img
  	mv "${{SCRIPTS_DIR}}"/os.tar "${{SCRIPTS_DIR}}"/kubeos.tar
}}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_pxe_repo_img"#
    )?;
    Ok(())
}

pub(crate) fn gen_create_docker_img(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function create_docker_img() {{
	create_os_tar_from_repo
	docker build -t "${{DOCKER_IMG}}" -f "${{SCRIPTS_DIR}}"/Dockerfile .
}}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_docker_img"#
    )?;
    Ok(())
}

// docker
pub(crate) fn gen_docker_vars(file: &mut File, image_name: &str) -> Result<()> {
    writeln!(
        file,
        r#"
IMAGE_NAME="{}"
BOOT_MODE=efi
"#,
        image_name
    )?;
    Ok(())
}

pub(crate) fn gen_create_os_tar_from_docker(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function create_os_tar_from_docker() {{
    container_id=$(docker create "${{DOCKER_IMG}}")
    echo "$container_id"
    docker cp "$container_id":/os.tar "${{SCRIPTS_DIR}}"
    docker rm "$container_id"
}}
"#
    )?;
    Ok(())
}

pub(crate) fn gen_create_vm_docker_img(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function create_vm_docker_img() {{
    create_os_tar_from_docker
    create_img
}}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_vm_docker_img"#
    )?;
    Ok(())
}

pub(crate) fn gen_create_pxe_docker_img(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function create_pxe_docker_img() {{
    rm -rf "${{SCRIPTS_DIR}}"/initramfs.img "${{SCRIPTS_DIR}}"/kubeos.tar
    create_os_tar_from_docker
    tar -xvf "${{SCRIPTS_DIR}}"/os.tar "${{SCRIPTS_DIR}}"/initramfs.img
    mv "${{SCRIPTS_DIR}}"/os.tar "${{SCRIPTS_DIR}}"/kubeos.tar
}}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_pxe_docker_img"#
    )?;
    Ok(())
}

// admin
pub(crate) fn gen_admin_vars(file: &mut File, docker_img: &str, dockerfile: &PathBuf) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;

    writeln!(
        file,
        r#"set -e

DOCKER_IMG={}
DOCKERFILE={}
ADMIN_CONTAINER_DIR="${{SCRIPTS_DIR}}"/admin-container
"#,
        dockerfile.to_str().unwrap(),
        docker_img
    )?;
    Ok(())
}

pub(crate) fn gen_create_admin_img(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"function create_admin_img() {{
    local kubeos_root_dir=$(dirname $(dirname $(dirname "${{SCRIPTS_DIR}}")))
    cp "${{kubeos_root_dir}}"/bin/hostshell "${{ADMIN_CONTAINER_DIR}}"
    docker build -t "${{DOCKER_IMG}}" -f "${{DOCKERFILE}}" "${{ADMIN_CONTAINER_DIR}}"
    rm -rf "${{ADMIN_CONTAINER_DIR}}"/hostshell
}}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_admin_img"#
    )?;
    Ok(())
}
/* endregion */

/* region: set_in_chroot.sh */
pub(crate) fn gen_add_users(file: &mut File, users: &Vec<User>) -> Result<()> {
    writeln!(file, "# add users")?;
    for user in users {
        let name = &user.name;
        let passwd = &user.passwd;
        let groups = match user.groups.clone() {
            Some(groups) => groups,
            None => vec![name.clone()],
        };
        for group in &groups {
            writeln!(
                file,
                r#"if ! getent group "{}" > /dev/null 2>&1; then
        groupadd "{}"
fi"#,
                group, group
            )?;
        }
        write!(file, "useradd -m -g {}", &groups[0])?;
        if groups.len() > 1 {
            let additional_groups = &groups[1..].join(",");
            write!(file, " -G {}", additional_groups)?;
        }
        writeln!(file, " -s /bin/bash \"{}\"", &name)?;
        writeln!(file, "echo \"{}:{}\" | chpasswd", name, passwd)?;
        if let Some(sudo) = &user.sudo {
            writeln!(
                file,
                r#"if visudo -c; then
    echo -e "{}    {}" | tee -a /etc/sudoers
else
    echo "Sudoers file syntax check failed. Please fix the sudoers file manually."
    exit 5
fi"#,
                name, sudo
            )?;
        }
    }
    Ok(())
}

pub(crate) fn gen_systemd_services(file: &mut File, systemd_services: &SystemdService) -> Result<()> {
    writeln!(file, "# systemd")?;
    for service_name in &systemd_services.name {
        writeln!(file, "systemctl enable {}", service_name)?;
    }
    Ok(())
}

pub(crate) fn gen_set_in_chroot(file: &mut File, legacy_bios: bool, config: &Config) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;

    writeln!(
        file,
        r#"ln -s /usr/lib/systemd/system/os-agent.service /usr/lib/systemd/system/multi-user.target.wants/os-agent.service
ln -s /usr/lib/systemd/system/kubelet.service /usr/lib/systemd/system/multi-user.target.wants/kubelet.service"#
    )?;
    if legacy_bios {
        writeln!(
            file,
            "ln -s /usr/lib/systemd/system/boot-grub2.mount /lib/systemd/system/local-fs.target.wants/boot-grub2.mount"
        )?;
    } else {
        writeln!(
            file,
            "ln -s /usr/lib/systemd/system/boot-efi.mount /lib/systemd/system/local-fs.target.wants/boot-efi.mount"
        )?;
    }
    writeln!(file, r#"ln -s /usr/lib/systemd/system/etc.mount /lib/systemd/system/local-fs.target.wants/etc.mount"#)?;

    if let Some(users) = &config.users {
        gen_add_users(file, users)?;
    }
    if let Some(systemd_services) = &config.systemd_service {
        gen_systemd_services(file, systemd_services)?;
    }

    writeln!(
        file,
        r#"
str=$(sed -n '/^root:/p' /etc/shadow | awk -F "root:" '{{print $2}}')
umask 0666
mv /etc/shadow /etc/shadow_bak
sed -i '/^root:/d' /etc/shadow_bak
echo "root:""${{ROOT_PASSWD}}""${{str:1}}" > /etc/shadow
cat /etc/shadow_bak >> /etc/shadow
rm -rf /etc/shadow_bak

dracut -f -v --add bootup /initramfs.img --kver "$(ls /lib/modules)"
rm -rf /usr/lib/dracut/modules.d/00bootup"#
    )?;

    Ok(())
}
/* endregion */

/* region: bootloader.sh */
pub(crate) fn gen_bootloader(file: &mut File, arch: &str, legacy_bios: bool) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;

    writeln!(
        file,
        r#"set -eu
set -o pipefail
set -x

function install_grub2 () {{"#
    )?;

    if arch == "aarch64" || (arch == "x86_64" && !legacy_bios) {
        writeln!(
            file,
            r#"    cp -r /usr/lib/grub/x86_64-efi boot/efi/EFI/openEuler
    eval "grub2-mkimage -d /usr/lib/grub/x86_64-efi -O x86_64-efi --output=/boot/efi/EFI/openEuler/grubx64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"

    mkdir -p /boot/efi/EFI/BOOT/
    cp -f /boot/efi/EFI/openEuler/grubx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI
}}    
"#
        )?;
    } else {
        writeln!(
            file,
            r#"    GRUBNAME=$(which grub2-install)
    echo "Installing GRUB2..."
    FORCE_OPT=${{FORCE_OPT:-"--force"}}
    TARGET_OPT=${{TARGET_OPT:-"--target=i386-pc"}}

    $GRUBNAME --modules="biosdisk part_msdos" "$FORCE_OPT" "$TARGET_OPT" "$DEVICE"
}}
"#
        )?;
    }

    writeln!(
        file,
        r#"install_grub2
"#
    )?;
    Ok(())
}
/* endregion */

/* region: rpmlist */
pub(crate) fn gen_rpm_list(file: &mut File, rpmlist: &Vec<String>) -> Result<()> {
    for rpm in rpmlist {
        writeln!(file, "{}", rpm)?;
    }
    Ok(())
}
/* endregion */

/* region: grub.cfg */
pub(crate) fn gen_grub_cfg(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"set pager=1

if [ -f ${{config_directory}}/grubenv ]; then
  load_env -f ${{config_directory}}/grubenv
elif [ -s $prefix/grubenv ]; then
  load_env
fi
if [ "${{next_entry}}" ] ; then
   set default="${{next_entry}}"
   set next_entry=
   save_env next_entry
   set boot_once=true
else
   set default="${{saved_entry}}"
fi

if [ x"${{feature_menuentry_id}}" = xy ]; then
  menuentry_id_option="--id"
else
  menuentry_id_option=""
fi

export menuentry_id_option

if [ "${{prev_saved_entry}}" ]; then
  set saved_entry="${{prev_saved_entry}}"
  save_env saved_entry
  set prev_saved_entry=
  save_env prev_saved_entry
  set boot_once=true
fi

function savedefault {{
  if [ -z "${{boot_once}}" ]; then
    saved_entry="${{chosen}}"
    save_env saved_entry
  fi
}}

function load_video {{
  if [ x$feature_all_video_module = xy ]; then
    insmod all_video
  else
    insmod efi_gop
    insmod efi_uga
    insmod ieee1275_fb
    insmod vbe
    insmod vga
    insmod video_bochs
    insmod video_cirrus
  fi
}}

terminal_output console
if [ x$feature_timeout_style = xy ] ; then
  set timeout_style=menu
  set timeout=5
# Fallback normal timeout code in case the timeout_style feature is
# unavailable.
else
  set timeout=5
fi
set superusers="root"
### END /etc/grub.d/00_header ###

### BEGIN /etc/grub.d/01_users ###
if [ -f ${{prefix}}/user.cfg ]; then
  source ${{prefix}}/user.cfg
  if [ -n "${{GRUB2_PASSWORD}}" ]; then
    set superusers="root"
    export superusers
    password_pbkdf2 root ${{GRUB2_PASSWORD}}
  fi
fi
### END /etc/grub.d/01_users ###

### BEGIN /etc/grub.d/10_linux ###
menuentry 'A' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-A' {{
	load_video
	set gfxpayload=keep
	insmod gzio
	insmod part_gpt
	insmod ext2
	set root='hd0,gpt2'
	linux   /boot/vmlinuz root=/dev/vda2 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3
	initrd  /boot/initramfs.img
}}

menuentry 'B' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-B' {{
	load_video
	set gfxpayload=keep
	insmod gzio
	insmod part_gpt
	insmod ext2
	set root='hd0,gpt3'
	linux   /boot/vmlinuz root=/dev/vda3 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3
	initrd  /boot/initramfs.img
}}

### END /etc/grub.d/10_linux ###

### BEGIN /etc/grub.d/10_reset_boot_success ###
# Hiding the menu is ok if last boot was ok or if this is a first boot attempt to boot the entry
if [ "${{boot_success}}" = "1" -o "${{boot_indeterminate}}" = "1" ]; then
  set menu_hide_ok=1
else
  set menu_hide_ok=0
fi
# Reset boot_indeterminate after a successful boot
if [ "${{boot_success}}" = "1" ] ; then
  set boot_indeterminate=0
# Avoid boot_indeterminate causing the menu to be hidden more then once
elif [ "${{boot_indeterminate}}" = "1" ]; then
  set boot_indeterminate=2
fi
# Reset boot_success for current boot
set boot_success=0
save_env boot_success boot_indeterminate
### END /etc/grub.d/10_reset_boot_success ###

### BEGIN /etc/grub.d/12_menu_auto_hide ###
if [ x$feature_timeout_style = xy ] ; then
  if [ "${{menu_show_once}}" ]; then
    unset menu_show_once
    save_env menu_show_once
    set timeout_style=menu
    set timeout=60
  elif [ "${{menu_auto_hide}}" -a "${{menu_hide_ok}}" = "1" ]; then
    set orig_timeout_style=${{timeout_style}}
    set orig_timeout=${{timeout}}
    if [ "${{fastboot}}" = "1" ]; then
      # timeout_style=menu + timeout=0 avoids the countdown code keypress check
      set timeout_style=menu
      set timeout=0
    else
      set timeout_style=hidden
      set timeout=1
    fi
  fi
fi
### END /etc/grub.d/12_menu_auto_hide ###

### BEGIN /etc/grub.d/20_linux_xen ###
### END /etc/grub.d/20_linux_xen ###

### BEGIN /etc/grub.d/20_ppc_terminfo ###
### END /etc/grub.d/20_ppc_terminfo ###

### BEGIN /etc/grub.d/30_uefi-firmware ###
### END /etc/grub.d/30_uefi-firmware ###

### BEGIN /etc/grub.d/40_custom ###
# This file provides an easy way to add custom menu entries.  Simply type the
# menu entries you want to add after this comment.  Be careful not to change
# the 'exec tail' line above.
### END /etc/grub.d/40_custom ###

### BEGIN /etc/grub.d/41_custom ###
if [ -f  ${{config_directory}}/custom.cfg ]; then
  source ${{config_directory}}/custom.cfg
elif [ -z "${{config_directory}}" -a -f  $prefix/custom.cfg ]; then
  source $prefix/custom.cfg;
fi
### END /etc/grub.d/41_custom ###
"#
    )?;
    Ok(())
}
/* endregion */

/* region: 00bootup */
// 00bootup/global.cfg
pub(crate) fn gen_global_cfg(file: &mut File, pxe_config: &PxeConfig) -> Result<()> {
    writeln!(
        file,
        r#"# rootfs file name
rootfs_name={}

# select the target disk to install kubeOS
disk={}

# pxe server ip address where stores the rootfs on the http server
server_ip={}
# target machine ip
local_ip={}
# target machine route
route_ip={}
# target machine netmask
netmask={}
# target machine netDevice name
net_name={}
"#,
        pxe_config.rootfs_name,
        pxe_config.disk,
        pxe_config.server_ip,
        pxe_config.local_ip,
        pxe_config.route_ip,
        pxe_config.netmask,
        pxe_config.net_name
    )?;
    Ok(())
}

// 00bootup/module-setup.sh
pub(crate) fn gen_module_setup(file: &mut File) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;

    writeln!(
        file,
        r#"check() {{
    return 0
}}

depends() {{
    echo systemd
}}

install() {{
    inst_multiple -o grub2-mkimage mkfs.ext4 mkfs.vfat lsblk tar cpio gunzip lspci parted dhclient ifconfig curl hwinfo head tee arch df awk route 
    inst_hook mount 00 "$moddir/mount.sh"
    inst_simple "$moddir/mount.sh" "/mount.sh"
    inst_simple "$moddir/Global.cfg" "/Global.cfg"
}}

installkernel() {{
    hostonly=''
    instmods='drivers/ata drivers/nvme drivers/scsi drivers/net fs/fat fs/nls'
}}
"#
    )?;
    Ok(())
}

// 00bootup/mount.sh
pub(crate) fn gen_mount(file: &mut File) -> Result<()> {
    writeln!(file, "#!/bin/bash")?;
    gen_copyright(file)?;

    writeln!(
        file,
        r#"arch=$(arch)
min_size=8
log=/install.log

source ./Global.cfg

function CheckSpace() {{
    local disk_ava
    disk_ava="$(parted -l | grep "${{disk}}" | awk '{{print $3}}')"
    if echo "${{disk_ava}}" | grep "[GT]B$"; then
        if echo "${{disk_ava}}" | grep GB$; then
            disk_ava="$(echo "${{disk_ava}}" | awk -F G '{{print $1}}' | awk -F . '{{print $1}}')"
            if [ "${{disk_ava}}" -lt ${{min_size}} ]; then
                echo "The available disk space is not enough, at least ${{min_size}}GB." | tee -a ${{log}}
                return 1
            fi
        fi
    else
        echo "The available disk space is not enough, at least ${{min_size}}G." | tee -a ${{log}}
        return 1
    fi

    return 0
}}

function mount_proc_dev_sys() {{
    local tmp_root=$1
    mount -t proc none "${{tmp_root}}/proc"
    mount --bind /dev "${{tmp_root}}/dev"
    mount --bind /dev/pts "${{tmp_root}}/dev/pts"
    mount -t sysfs none "${{tmp_root}}/sys"
}}

function GetDisk() {{
    mapfile -t disks < <(hwinfo --disk --short 2>&1 | grep -vi "^disk" | awk '{{print $1}}')
    if [ ${{#disks[*]}} -gt 0 ]; then
        if [ -n "${{disk}}" ] && echo "${{disks[@]}}" | grep -wq "${{disk}}" ; then
            echo "${{disk}} exists, start partition"  | tee -a ${{log}}
        else
            echo "disk not exist, please choose correct disk"  | tee -a ${{log}}
        fi
    else
        echo "no disk found" | tee -a ${{log}}
        return 1
    fi
    CheckSpace
    local status=$?
    if [ $status -ne 0 ]; then
        echo "no enough space on ${{disk}}" | tee -a ${{log}}
        return 1
    fi

    return 0
}}

function PartitionAndFormatting() {{
    echo "Partitioning and formatting disk $disk..."
    # partition and format
    parted "${{disk}}" -s mklabel gpt >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "partition failed" | tee -a ${{log}}
        return 1
    fi

    parted "${{disk}}" -s mkpart primary fat16 1M 100M >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "partition failed" | tee -a ${{log}}
        return 1
    fi

    parted "${{disk}}" -s mkpart primary ext4 100M 2600M >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "partition failed" | tee -a ${{log}}
        return 1
    fi

    parted "${{disk}}" -s mkpart primary ext4 2600M 5100M >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "partition failed" | tee -a ${{log}}
        return 1
    fi

    parted "${{disk}}" -s mkpart primary ext4 5100M 100% >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "partition failed" | tee -a ${{log}}
        return 1
    fi

    parted "${{disk}}" -s set 1 boot on >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "partition failed" | tee -a ${{log}}
        return 1
    fi

    mkfs.vfat -n "BOOT" "${{disk}}"1 >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "format failed" | tee -a ${{log}}
        return 1
    fi

    mkfs.ext4 -L "ROOT-A" "${{disk}}"2 >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "format failed" | tee -a ${{log}}
        return 1
    fi

    mkfs.ext4 -L "ROOT-B" "${{disk}}"3 >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "format failed" | tee -a ${{log}}
        return 1
    fi

    mkfs.ext4 -L "PERSIST" "${{disk}}"4 >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "format failed" | tee -a ${{log}}
        return 1
    fi

    return 0
}}

function InitNetwork() {{
    echo "Initializing network..."
    mapfile -t netNames < <(ifconfig -a | awk '{{print $1}}' | grep : | grep '^e' | awk -F: '{{print $1}}')
    if [ ${{#netNames[*]}} -gt 0 ]; then
        if [ -n "${{net_name}}" ] && echo "${{netNames[@]}}" | grep -wq "${{net_name}}" ; then
            echo "${{net_name}} exists, start set ip"  | tee -a ${{log}}
        else
            echo "net_name not exist, choose default net"  | tee -a ${{log}}
            net_name=${{netNames[0]}}
        fi
    else
        echo "no net Device found" | tee -a ${{log}}
        return 1
    fi

    ifconfig "${{net_name}}" up
    local status=$?
    if [ $status -ne 0 ]; then
        echo "load net card failed" | tee -a ${{log}}
        return 1
    fi
    sleep 3

    ifconfig "${{net_name}}" "${{local_ip}}" netmask "${{netmask}}"  >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "ip set failed" | tee -a ${{log}}
        return 1
    fi
    sleep 3

    route add default gw "${{route_ip}}" >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "add route failed" | tee -a ${{log}}
        return 1
    fi
    sleep 3
    return 0
}}

function MountRoot() {{
    echo "Mounting rootfs..."
    # mount rootfs
    mount "${{disk}}"2 /sysroot >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "mount rootfs failed" | tee -a ${{log}}
        return 1
    fi

    return 0
}}

function MountPersist() {{
    echo "Mounting persist"
    mount "${{disk}}"4 /sysroot/persist >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "mount persist failed" | tee -a ${{log}}
        return 1
    fi
    mkdir /sysroot/persist/{{var,etc,etcwork}}
    mkdir -p /sysroot/persist/etc/KubeOS/certs
    return 0
}}

function MountBoot() {{
    echo "Mounting boot"
    mkdir -p /sysroot/boot/efi
    mount "${{disk}}"1 /sysroot/boot/efi >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "mount boot failed" | tee -a ${{log}}
        return 1
    fi
    return 0
}}

function GetRootfs() {{
    echo "Downloading rootfs..."

    curl -o /"${{rootfs_name}}" http://"${{server_ip}}"/"${{rootfs_name}}"
    if [ ! -e "/${{rootfs_name}}" ]; then
        echo "download rootfs failed" | tee -a ${{log}}
        return 1
    fi

    tar -xf /"${{rootfs_name}}" -C /sysroot
    local status=$?
    if [ $status -ne 0 ]; then
        echo "decompose rootfs failed" | tee -a ${{log}}
        return 1
    fi

    rm -rf "${{rootfs_name:?}}"
    mount -o remount,ro "${{disk}}"2 /sysroot  >> ${{log}} 2>&1
    return 0
}}

function Inst_Grub2_x86() {{
    # copy the files that boot need
    cp -r /sysroot/usr/lib/grub/x86_64-efi /sysroot/boot/efi/EFI/openEuler
    eval "grub2-mkimage -d /sysroot/usr/lib/grub/x86_64-efi -O x86_64-efi --output=/sysroot/boot/efi/EFI/openEuler/grubx64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"  >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "grub2-mkimage on x86 failed" | tee -a ${{log}}
        return 1
    fi
    
    mkdir -p /sysroot/boot/efi/EFI/BOOT/
    cp -f /sysroot/boot/efi/EFI/openEuler/grubx64.efi /sysroot/boot/efi/EFI/BOOT/BOOTX64.EFI

    return 0
}}

function Inst_Grub2_aarch64() {{
    cp -r /sysroot/usr/lib/grub/arm64-efi /sysroot/boot/efi/EFI/openEuler/
    eval "grub2-mkimage -d /sysroot/usr/lib/grub/arm64-efi -O arm64-efi --output=/sysroot/boot/efi/EFI/openEuler/grubaa64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"  >> ${{log}} 2>&1
    local status=$?
    if [ $status -ne 0 ]; then
        echo "grub2-mkimage on aarch64 failed" | tee -a ${{log}}
        return 1
    fi
    
    mkdir -p /sysroot/boot/efi/EFI/BOOT/
    cp -f /sysroot/boot/efi/EFI/openEuler/grubaa64.efi /sysroot/boot/efi/EFI/BOOT/BOOTAA64.EFI

    return 0
}}

function SetBoot() {{
    # mount boot
    echo "Setting boot"

    if [ "$arch" == "x86_64" ];   then
        Inst_Grub2_x86
        local status=$?
        if [ $status -ne 0 ]; then
            echo "install grub on x86 failed" | tee -a ${{log}}
            return 1
        fi
    fi

    if [ "$arch" == "aarch64" ]; then
        Inst_Grub2_aarch64
        local status=$?
        if [ $status -ne 0 ]; then
            echo "install grub on aarch64 failed" | tee -a ${{log}}
            return 1
        fi
    fi
    sed -i 's#/dev/sda#'"${{disk}}"'#g' /sysroot/boot/efi/EFI/openEuler/grub.cfg

    return 0
}}

function Bootup_Main() {{
    # get disk
    echo "Checking disk info..." | tee -a ${{log}}
    GetDisk
    local status=$?
    if [ $status -ne 0 ]; then
        echo "Checking disk info failed" | tee -a ${{log}}
        return 1
    fi

    # partition and format disk
    echo "Partion and formatting..." | tee -a ${{log}}
    PartitionAndFormatting
    local status=$?
    if [ $status -ne 0 ]; then
        echo "Partition and formatting disk failed" | tee -a ${{log}}
        return 1
    fi

    # init network
    echo "Initializing network..." | tee -a ${{log}}
    InitNetwork
    local status=$?
    if [ $status -ne 0 ]; then
        echo "Initializing network failed" | tee -a ${{log}}
        return 1
    fi

    # mount partitions
    
    # mount boot
    echo "Mounting root..." | tee -a ${{log}}
    MountRoot
    local status=$?
    if [ $status -ne 0 ]; then
        echo "Mounting root failed" | tee -a ${{log}}
        return 1
    fi

    echo "Mounting boot..." | tee -a ${{log}}
    MountBoot
    local status=$?
    if [ $status -ne 0 ]; then
        echo "Mounting boot failed" | tee -a ${{log}}
        return 1
    fi

    # download rootfs
    echo "Downloading rootfs..." | tee -a ${{log}}
    GetRootfs
    local status=$?
    if [ $status -ne 0 ]; then
        echo "Downloading rootfs failed" | tee -a ${{log}}
        return 1
    fi
    mount_proc_dev_sys /sysroot
    # set boot
    echo "Setting boot..." | tee -a ${{log}}
    SetBoot
    local status=$?
    if [ $status -ne 0 ]; then
        echo "Setting boot failed" | tee -a ${{log}}
        return 1
    fi
    # mount persist
    echo "Mounting persist..." | tee -a ${{log}}
    MountPersist
    local status=$?
    if [ $status -ne 0 ]; then
        echo "Mounting persist failed" | tee -a ${{log}}
        return 1
    fi
    return 0
}}

Bootup_Main
ret=$?
if [ ${{ret}} -eq 0 ]; then
    echo "kubeOS install success! switch to root" | tee -a ${{log}}
    cp ${{log}} /sysroot/persist
else
    echo "kubeOS install failed, see install.log" | tee -a ${{log}}
fi

"#
    )?;
    Ok(())
}
/* endregion */

/* region: dockerfile */
pub(crate) fn gen_dockerfile(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"FROM scratch
COPY os.tar /
CMD ["/bin/sh"]
"#
    )?;
    Ok(())
}
/* endregion */

/* region: admin-container */
// admin-container/dockerfile
pub(crate) fn gen_admin_dockerfile(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"FROM openeuler-22.03-lts
MAINTAINER <shenyangyang4@huawei.com>

RUN yum -y install openssh-clients util-linux

ADD ./sysmaster-0.2.3-1.oe2203.aarch64.rpm /home
RUN rpm -ivh  /home/sysmaster-0.2.3-1.oe2203.aarch64.rpm

COPY ./hostshell /usr/bin/
COPY ./set-ssh-pub-key.sh /usr/local/bin
COPY ./set-ssh-pub-key.service /usr/lib/sysmaster

EXPOSE 22
# set sshd.service and set-ssh-pub-key.service pulled up by default
RUN sed -i 's/sysinit.target/sysinit.target;sshd.service;set-ssh-pub-key.service/g' /usr/lib/sysmaster/basic.target

CMD ["/usr/lib/sysmaster/init"]
"#
    )?;
    Ok(())
}

// admin-container/set-ssh-pub-key.service
pub(crate) fn gen_set_ssh_pub_key_service(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"[Unit]
Description="set ssh authorized keys according to the secret which is set by user"

[Service]
ExecStart="/usr/local/bin/set-ssh-pub-key.sh"
"#
    )?;
    Ok(())
}

// admin-container/set-ssh-pub-key.sh
pub(crate) fn gen_set_ssh_pub_key(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"ssh_pub=$(cat /etc/secret-volume/ssh-pub-key)
ssh_dir="/root/.ssh"
authorized_file="$ssh_dir/authorized_keys"

if [ ! -d "$ssh_dir" ]; then
    mkdir "$ssh_dir"
    chmod 700 "$ssh_dir"
fi

if [ ! -f "$authorized_file" ]; then
    touch "$authorized_file"
    chmod 600 "$authorized_file"
fi

echo "$ssh_pub" >> "$authorized_file"
"#
    )?;
    Ok(())
}
/* endregion */

/* region: misc-files */
// misc-files/boot-efi.mount
pub(crate) fn gen_boot_efi_mount(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"[Unit]
Description=grub2 Dir
DefaultDependencies=no
Conflicts=umount.target
Before=local-fs.target umount.target

[Mount]
What=/dev/disk/by-label/BOOT
Where=/boot/efi
Type=vfat
Options=defaults

[Install]
WantedBy=local-fs.target
"#
    )?;
    Ok(())
}

// misc-files/boot-grub2.mount
pub(crate) fn gen_boot_grub2_mount(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"[Unit]
Description=grub2 Dir
DefaultDependencies=no
Conflicts=umount.target
Before=local-fs.target umount.target

[Mount]
What=/dev/disk/by-label/GRUB2
Where=/boot/grub2
Type=ext4
Options=defaults

[Install]
WantedBy=local-fs.target
"#
    )?;
    Ok(())
}

// misc-files/etc.mount
pub(crate) fn gen_etc_mount(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"[Unit]
Description=etc Dir
DefaultDependencies=no
Conflicts=umount.target
Before=local-fs.target umount.target
Wants=persist.mount
After=persist.mount

[Mount]
What=overlay
Where=/etc
Type=overlay
Options=upperdir=/persist/etc,lowerdir=/etc,workdir=/persist/etcwork

[Install]
WantedBy=local-fs.target
"#
    )?;
    Ok(())
}

// misc-files/os-agent.service
pub(crate) fn gen_os_agent_service(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"[Unit]
Description=Agent For KubeOS

[Service]
Environment=GOTRACEBACK=crash
ExecStart=/usr/bin/os-agent
KillMode=process
Restart=on-failure

[Install]
WantedBy=multi-user.target
"#
    )?;
    Ok(())
}

// misc-files/os-release
pub(crate) fn gen_os_release(file: &mut File) -> Result<()> {
    writeln!(
        file,
        r#"NAME=KubeOS
ID=KubeOS
"#
    )?;
    Ok(())
}

// misc-files/persist.mount
pub(crate) fn gen_persist_mount(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"[Unit]
Description=PERSIST Dir (/persist)
DefaultDependencies=no
Conflicts=umount.target
Before=local-fs.target umount.target

[Mount]
What=/dev/disk/by-label/PERSIST
Where=/persist
Type=ext4
Options=defaults

[Install]
WantedBy=local-fs.target
"#
    )?;
    Ok(())
}

// misc-files/var.mount
pub(crate) fn gen_var_mount(file: &mut File) -> Result<()> {
    gen_copyright(file)?;

    writeln!(
        file,
        r#"[Unit]
Description=var Dir
DefaultDependencies=no
Conflicts=umount.target
Before=local-fs.target umount.target
Wants=persist.mount
After=persist.mount

[Mount]
What=/persist/var
Where=/var
Type=node
Options=bind

[Install]
WantedBy=local-fs.target
"#
    )?;
    Ok(())
}
/* endregion */
