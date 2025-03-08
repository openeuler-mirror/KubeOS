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

pub(crate) const SCRIPTS_DIR: &str = "./scripts-auto";
pub(crate) const KBIMG_SH: &str = "kbimg.sh";
pub(crate) const BOOTLOADER_SH: &str = "bootloader.sh";
pub(crate) const SET_IN_CHROOT_SH: &str = "set_in_chroot.sh";
pub(crate) const GRUB_CFG: &str = "grub.cfg";
pub(crate) const RPMLIST: &str = "rpmlist";
pub(crate) const DOCKERFILE: &str = "Dockerfile";

pub(crate) const BOOTUP_DIR: &str = "./scripts-auto/00bootup";
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
pub(crate) const MISC_OPT_CNI_MOUNT: &str = "opt-cni.mount";
pub(crate) const MISC_OS_AGENT_SERVICE: &str = "os-agent.service";
pub(crate) const MISC_OS_RELEASE: &str = "os-release";
pub(crate) const MISC_PERSIST_MOUNT: &str = "persist.mount";
pub(crate) const MISC_VAR_MOUNT: &str = "var.mount";

pub(crate) const DMV_DIR: &str = "./scripts-auto/dm-verity";
pub(crate) const DMV_CHROOT: &str = "chroot_new_grub.sh";
pub(crate) const DMV_MAIN: &str = "dm_verity.sh";
pub(crate) const DMV_DRACUT_DIR: &str = "./scripts-auto/05dmverity";
pub(crate) const DMV_DRACUT_MOUNT: &str = "dmv-mount.sh";
pub(crate) const DMV_DRACUT_MODULE: &str = "module-setup.sh";
pub(crate) const DMV_UPGRADE_ROLLBACK: &str = "kubeos-dmv";

// permissions
pub(crate) const CONFIG_PERMISSION: u32 = 0o640;
pub(crate) const EXEC_PERMISSION: u32 = 0o550;
pub(crate) const DIR_PERMISSION: u32 = 0o750;

// KubeOS image(GB) and partition(MiB) size
pub(crate) const BOOT_SIZE: u32 = 60;
pub(crate) const ROOT_SIZE: u32 = 2560;
pub(crate) const HASH_SIZE: u32 = 128;
pub(crate) const PERSIST_SIZE: u32 = 2100;
pub(crate) const IMAGE_SIZE: u32 = 20;

pub const COPYRIGHT: &str = r#"# Copyright (c) Huawei Technologies Co., Ltd. 2024. All rights reserved.
# KubeOS is licensed under the Mulan PSL v2.
# You can use this software according to the terms and conditions of the Mulan PSL v2.
# You may obtain a copy of Mulan PSL v2 at:
#     http://license.coscl.org.cn/MulanPSL2
# THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
# PURPOSE.
# See the Mulan PSL v2 for more details.
"#;

pub const GLOBAL_VARS: &str = r#"set -eux

umask 022
NAME=KubeOS
ID=kubeos
SCRIPTS_DIR=$(cd "$(dirname "$0")" && pwd)
LOCK="${SCRIPTS_DIR}"/test.lock
RPM_ROOT="${SCRIPTS_DIR}"/rootfs
TMP_MOUNT_PATH="${SCRIPTS_DIR}"/mnt"#;

pub const TEST_LOCK: &str = r#"function file_lock() {
    local lock_file=$1
    exec {lock_fd}>"${lock_file}"
    flock -xn "${lock_fd}"
}

function test_lock() {
    file_lock "${LOCK}"
    local status=$?
    if [ $status -ne 0 ]; then
        log_error_print "There is already an generate process running."
        exit 203
    fi
}
"#;

pub const CLEANUP: &str = r#"function clean_space() {
    delete_dir "${RPM_ROOT}"
    delete_dir "${TMP_MOUNT_PATH}"
    delete_file "${SCRIPTS_DIR}"/os.tar
    rm -rf "${LOCK}"
    delete_dir "${SCRIPTS_DIR}/dm-verity/tmp"
}

function clean_img() {
    losetup -D
    delete_file "${SCRIPTS_DIR}"/system.img
    delete_file "${SCRIPTS_DIR}"/update.img
    delete_file "${SCRIPTS_DIR}"/initramfs.img
    delete_file "${SCRIPTS_DIR}"/kubeos.tar
    delete_file "${SCRIPTS_DIR}"/update-root.img
    delete_file "${SCRIPTS_DIR}"/update-boot.img
    delete_file "${SCRIPTS_DIR}"/update-hash.img
    delete_file "${SCRIPTS_DIR}"/update-roothash
}
"#;

pub const DELETE_DIR: &str = r#"function delete_dir() {
    local ret=0
    local dir="$1"
    unmount_dir "${dir}"
    ret=$?
    if [ "${ret}" -eq 0 ]; then
        rm -rf "${dir}"
        return 0
    else
        log_error_print "${dir} is failed to unmount , can not delete ${dir}."
        return 1
    fi
}
"#;

pub const DELETE_FILE: &str = r#"function delete_file() {
    local file="$1"
    if [ ! -e "${file}" ]; then
        return 0
    fi

    if [ ! -f "${file}" ]; then
        log_error_print "${file} is not a file."
        return 1
    fi

    rm -f "${file}"
    return 0
}
"#;

pub const LOG: &str = r#"function log_error_print() {
    local logmsg
    logmsg="[ ERROR ] - ""$(date "+%b %d %Y %H:%M:%S")"" $1"
    echo "$logmsg"
}

function log_info_print() {
    local logmsg
    logmsg="[ INFO ] - ""$(date "+%b %d %Y %H:%M:%S")"" $1"
    echo "$logmsg"
}
"#;

pub const MOUNT_PROC_DEV_SYS: &str = r#"function mount_proc_dev_sys() {
    local tmp_root=$1
    mount -t proc none "${tmp_root}"/proc
    mount --bind /dev "${tmp_root}"/dev
    mount --bind /dev/pts "${tmp_root}"/dev/pts
    mount -t sysfs none "${tmp_root}"/sys
}
"#;

pub const UNMOUNT_DIR: &str = r#"function unmount_dir() {
    local dir=$1

    if [ -L "${dir}" ] || [ -f "${dir}" ]; then
        log_error_print "${dir} is not a directory, please check it."
        return 1
    fi

    if [ ! -d "${dir}" ]; then
        return 0
    fi

    local real_dir
    real_dir=$(readlink -e "${dir}")
    local mnts
    mnts=$(awk '{print $2}' </proc/mounts | grep "^${real_dir}" | sort -r)
    for m in ${mnts}; do
        log_info_print "Unmount ${m}"
        umount -f "${m}" || true
    done

    return 0
}
"#;

pub const INIT_PARTITION: &str = r#"function init_part() {
    local offset
    offset=$(fdisk -l "${SCRIPTS_DIR}"/system.img | grep "$1" | awk '{print $2}')
    local sizelimit
    sizelimit=$(fdisk -l "${SCRIPTS_DIR}"/system.img | grep "$1" | awk '{print $3}')
    sizelimit=$(echo "($sizelimit - $offset)*512" | bc)
    offset=$(echo "${offset}*512" | bc)
    local loop
    loop=$(losetup -f)
    losetup -o "${offset}" --sizelimit "${sizelimit}" "${loop}" "${SCRIPTS_DIR}"/system.img
    if [ "$2" == "BOOT" ]; then
        mkfs.vfat -n "$2" "${loop}"
        mount -t vfat "${loop}" "$3"
    else
        mkfs.ext4 -L "$2" "${loop}"
        mount -t ext4 "${loop}" "$3"
        rm -rf "$3/lost+found"
    fi
}
"#;

pub const PREPARE_YUM: &str = r#"function prepare_yum() {
    # init rpmdb
    rpm --root "${RPM_ROOT}" --initdb
    mkdir -p "${RPM_ROOT}"{/etc/yum.repos.d,/persist,/proc,/dev/pts,/sys}
    mount_proc_dev_sys "${RPM_ROOT}"
    # init yum repo
    local iso_repo="${RPM_ROOT}"/etc/yum.repos.d/iso.repo
    cat "${REPO_PATH}" >"$iso_repo"
}
"#;

pub const INSTALL_PACKAGES: &str = r#"function install_packages() {
    prepare_yum "${REPO_PATH}"

    echo "install package.."

    local filesize
    filesize=$(stat -c "%s" "${SCRIPTS_DIR}"/rpmlist)
    local maxsize=$((1024 * 1024))
    if [ "${filesize}" -gt "${maxsize}" ]; then
        echo "please check if rpmlist is too big or something wrong"
        exit 7
    fi

    local rpms_name
    rpms_name=$(tr "\n" " " <"${SCRIPTS_DIR}"/rpmlist)
    read -ra rpms <<<"${rpms_name}"
    yum -y --installroot="${RPM_ROOT}" install --nogpgcheck --setopt install_weak_deps=False "${rpms[@]}"
    yum -y --installroot="${RPM_ROOT}" clean all
}
"#;

pub const INSTALL_MISC: &str = r#"function install_misc() {{
    cp "${{SCRIPTS_DIR}}"/misc-files/*mount "${{SCRIPTS_DIR}}"/misc-files/os-agent.service "${{RPM_ROOT}}"/usr/lib/systemd/system/
    cp "${{SCRIPTS_DIR}}"/misc-files/os-release "${{RPM_ROOT}}"/usr/lib/
    cp "${{AGENT_PATH}}" "${{RPM_ROOT}}"/usr/bin
    rm "${{RPM_ROOT}}"/etc/os-release

    cat <<EOF >"${{RPM_ROOT}}"/usr/lib/os-release
NAME="${{NAME}}"
ID=${{ID}}
PRETTY_NAME="${{NAME}} ${{VERSION}}"
VERSION_ID=${{VERSION}}
EOF
    mv "${{RPM_ROOT}}"/boot/vmlinuz* "${{RPM_ROOT}}"/boot/vmlinuz
    mv "${{RPM_ROOT}}"/boot/initramfs* "${{RPM_ROOT}}"/boot/initramfs.img
    {COPY_GRUB_CFG}
    {PXE_BOOTUP_FILES}
    {DM_VERITY_FILES}
    # custom config
{CUSTOM_SCRIPT}

    cp "${{SCRIPTS_DIR}}"/set_in_chroot.sh "${{RPM_ROOT}}"
    ROOT_PASSWD="${{ROOT_PASSWD}}" chroot "${{RPM_ROOT}}" bash /set_in_chroot.sh
    rm "${{RPM_ROOT}}/set_in_chroot.sh"
}}
"#;

pub const SET_IN_CHROOT: &str = r#"set -eux
ln -s /usr/lib/systemd/system/os-agent.service /usr/lib/systemd/system/multi-user.target.wants/os-agent.service
ln -s /usr/lib/systemd/system/kubelet.service /usr/lib/systemd/system/multi-user.target.wants/kubelet.service
ln -s /usr/lib/systemd/system/etc.mount /lib/systemd/system/local-fs.target.wants/etc.mount
ln -s /usr/lib/systemd/system/opt-cni.mount /lib/systemd/system/local-fs.target.wants/opt-cni.mount
mkdir -p /opt/cni
{BOOT_MOUNT_ENABLE}

str=$(sed -n '/^root:/p' /etc/shadow | awk -F "root:" '{{print $2}}')
umask 0666
mv /etc/shadow /etc/shadow_bak
sed -i '/^root:/d' /etc/shadow_bak
echo "root:""${{ROOT_PASSWD}}""${{str:1}}" >/etc/shadow
cat /etc/shadow_bak >>/etc/shadow
rm -rf /etc/shadow_bak
{PXE_DRACUT}
{DM_VERITY_DRACUT}"#;

pub const SET_PARTUUID: &str = r#"function set_partuuid() {{
    root_path=$1
    grub_path="$root_path{GRUB_PATH}"
    {ROOT_PARTUUID}

    sed -i "s/vmlinuz root=\/dev\/vda2/vmlinuz root=PARTUUID=$ROOTA_PARTUUID/g" "$grub_path"
    sed -i "s/vmlinuz root=\/dev\/vda3/vmlinuz root=PARTUUID=$ROOTB_PARTUUID/g" "$grub_path"
}}
"#;

pub const SET_PARTUUID_LEGACY: &str = r#"DISK_ID=$(sfdisk --disk-id "${SCRIPTS_DIR}"/system.img | tr '[:upper:]' '[:lower:]' | cut -c3-)
    ROOTA_PARTUUID="${DISK_ID}-02"
    ROOTB_PARTUUID="${DISK_ID}-03""#;

pub const SET_PARTUUID_EFI: &str = r#"ROOTA_PARTUUID=$(sfdisk --part-uuid "${SCRIPTS_DIR}"/system.img 2 | tr '[:upper:]' '[:lower:]')
    ROOTB_PARTUUID=$(sfdisk --part-uuid "${SCRIPTS_DIR}"/system.img 3 | tr '[:upper:]' '[:lower:]')"#;

pub const CREATE_IMAGE: &str = r#"function create_img() {{
    rm -f "${{SCRIPTS_DIR}}"/system.img
    qemu-img create "${{SCRIPTS_DIR}}/system.img" {IMG_SIZE}G
    {PARTITIONS}
    local device
    device=$(losetup -f)
    losetup "${{device}}" "${{SCRIPTS_DIR}}"/system.img
    mkdir -p "${{TMP_MOUNT_PATH}}"

    init_part "${{SCRIPTS_DIR}}"/system.img2 ROOT-A "${{TMP_MOUNT_PATH}}"
    mkdir -p "${{BOOT_PATH}}"
    chmod 755 "${{BOOT_PATH}}"
    {INIT_BOOT}
    tar -x -C "${{TMP_MOUNT_PATH}}" -f "${{SCRIPTS_DIR}}"/os.tar
    {SET_PARTUUID}
    sync
    cp "${{SCRIPTS_DIR}}"/bootloader.sh "${{TMP_MOUNT_PATH}}"
    mount_proc_dev_sys "${{TMP_MOUNT_PATH}}"
    DEVICE="${{device}}" chroot "${{TMP_MOUNT_PATH}}" bash bootloader.sh
    rm -rf "${{TMP_MOUNT_PATH}}"/bootloader.sh
    sync
    unmount_dir "${{TMP_MOUNT_PATH}}"

    {INIT_ROOTB}
    umount "${{TMP_MOUNT_PATH}}"

    {INIT_PERSIST}
{MKDIR_PERSIST}
    mkdir "${{TMP_MOUNT_PATH}}"/{{var,etc,etcwork,opt,optwork}}
    mkdir -p "${{TMP_MOUNT_PATH}}"/etc/KubeOS/certs
    umount "${{TMP_MOUNT_PATH}}"

    losetup -D
    parted "${{SCRIPTS_DIR}}"/system.img -- set 1 boot on
    {DMV_MAIN}
    qemu-img convert "${{SCRIPTS_DIR}}"/system.img -O qcow2 "${{SCRIPTS_DIR}}"/system.qcow2
}}
"#;

pub const CREATE_OS_TAR_FROM_REPO: &str = r#"function create_os_tar_from_repo() {
    install_packages
    install_misc
    unmount_dir "${RPM_ROOT}"
    tar -C "${RPM_ROOT}" -cf "${SCRIPTS_DIR}"/os.tar .
    cp "${SCRIPTS_DIR}"/os.tar "${SCRIPTS_DIR}"/kubeos.tar 
}
"#;

pub const CREATE_OS_TAR_FROM_DOCKER: &str = r#"function create_os_tar_from_docker() {
    container_id=$(docker create "${DOCKER_IMG}")
    echo "$container_id"
    docker cp "$container_id":/os.tar "${SCRIPTS_DIR}"
    docker rm "$container_id"
}
"#;

pub const CREATE_VM_REPO_IMAGE: &str = r#"function create_vm_repo_img() {
    create_os_tar_from_repo
    create_img
}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_vm_repo_img"#;

pub const CREATE_PXE_REPO_IMAGE: &str = r#"function create_pxe_repo_img() {
    rm -rf "${SCRIPTS_DIR}"/initramfs.img "${SCRIPTS_DIR}"/kubeos.tar
    create_os_tar_from_repo
    tar -xvf "${SCRIPTS_DIR}"/os.tar -C "${SCRIPTS_DIR}" ./initramfs.img
    mv "${SCRIPTS_DIR}"/os.tar "${SCRIPTS_DIR}"/kubeos.tar
}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_pxe_repo_img"#;

pub const CREATE_DOCKER_IMAGE: &str = r#"function create_docker_img() {
    create_os_tar_from_repo
    docker build -t "${DOCKER_IMG}" -f "${SCRIPTS_DIR}"/Dockerfile "${SCRIPTS_DIR}"
}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_docker_img"#;

pub const CREATE_VM_DOCKER_IMAGE: &str = r#"function create_vm_docker_img() {
    create_os_tar_from_docker
    create_img
}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_vm_docker_img"#;

pub const CREATE_PXE_DOCKER_IMAGE: &str = r#"function create_pxe_docker_img() {
    rm -rf "${SCRIPTS_DIR}"/initramfs.img "${SCRIPTS_DIR}"/kubeos.tar
    create_os_tar_from_docker
    tar -xvf "${SCRIPTS_DIR}"/os.tar -C "${SCRIPTS_DIR}" ./initramfs.img
    mv "${SCRIPTS_DIR}"/os.tar "${SCRIPTS_DIR}"/kubeos.tar
}

test_lock
trap clean_space EXIT
trap clean_img ERR

create_pxe_docker_img"#;

pub const CREATE_ADMIN_IMAGE: &str = r#"function create_admin_img() {
    cp "${HOSTSHELL}" "${ADMIN_CONTAINER_DIR}"/hostshell
    docker build -t "${DOCKER_IMG}" -f "${DOCKERFILE}" "${ADMIN_CONTAINER_DIR}"
}

test_lock
trap 'rm -f "${ADMIN_CONTAINER_DIR}"/hostshell;rm -f "${LOCK}"' EXIT
trap 'rm -f "${ADMIN_CONTAINER_DIR}"/hostshell;rm -f "${LOCK}"' ERR

create_admin_img"#;

pub const BOOT_LOADER_LEGACY: &str = r#"
GRUBNAME=$(which grub2-install)
echo "Installing GRUB2..."
GRUB_OPTS=${GRUB_OPTS:-"--force"}
GRUB_OPTS="$GRUB_OPTS --target=i386-pc"
# shellcheck disable=SC2086
$GRUBNAME --modules="biosdisk part_msdos" $GRUB_OPTS "$DEVICE""#;

pub const BOOT_LOADER_X86_UEFI: &str = r#"
cp -r /usr/lib/grub/x86_64-efi boot/efi/EFI/openEuler
eval "grub2-mkimage -d /usr/lib/grub/x86_64-efi -O x86_64-efi --output=/boot/efi/EFI/openEuler/grubx64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"
mkdir -p /boot/efi/EFI/BOOT/
cp -f /boot/efi/EFI/openEuler/grubx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI"#;

pub const BOOT_LOADER_AARCH64: &str = r#"
cp -r /usr/lib/grub/arm64-efi /boot/efi/EFI/openEuler/
eval "grub2-mkimage -d /usr/lib/grub/arm64-efi -O arm64-efi --output=/boot/efi/EFI/openEuler/grubaa64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"
mkdir -p /boot/efi/EFI/BOOT/
cp -f /boot/efi/EFI/openEuler/grubaa64.efi /boot/efi/EFI/BOOT/BOOTAA64.EFI"#;

pub const MODULE_SETUP: &str = r#"check() {
    return 0
}

depends() {
    echo systemd
}

install() {
    inst_multiple -o grub2-mkimage mkfs.ext4 mkfs.vfat lsblk tar cpio gunzip lspci parted dhclient ifconfig curl hwinfo head tee arch df awk route 
    inst_hook mount 00 "$moddir/mount.sh"
    inst_simple "$moddir/mount.sh" "/mount.sh"
}

installkernel() {
    hostonly='' instmods =drivers/ata =drivers/nvme =drivers/scsi =drivers/net =fs/fat =fs/nls
}"#;

pub const BOOTUP_MOUNT_1: &str = r#"arch=$(arch)
min_size=8
log=/install.log

function CheckSpace() {
    local disk_ava
    disk_ava="$(parted -l | grep "${disk}" | awk '{print $3}')"
    if echo "${disk_ava}" | grep "[GT]B$"; then
        if echo "${disk_ava}" | grep "GB$"; then
            disk_ava="$(echo "${disk_ava}" | awk -F G '{print $1}' | awk -F . '{print $1}')"
            if [ "${disk_ava}" -lt ${min_size} ]; then
                echo "The available disk space is not enough, at least ${min_size}GB." | tee -a ${log}
                return 1
            fi
        fi
    else
        echo "The available disk space is not enough, at least ${min_size}G." | tee -a ${log}
        return 1
    fi

    return 0
}

function mount_proc_dev_sys() {
    local tmp_root=$1
    mount -t proc none "${tmp_root}/proc"
    mount --bind /dev "${tmp_root}/dev"
    mount --bind /dev/pts "${tmp_root}/dev/pts"
    mount -t sysfs none "${tmp_root}/sys"
}

function GetDisk() {
    mapfile -t disks < <(hwinfo --disk --short 2>&1 | grep -vi "^disk" | awk '{print $1}')
    if [ ${#disks[*]} -gt 0 ]; then
        if [ -n "${disk}" ] && echo "${disks[@]}" | grep -wq "${disk}" ; then
            echo "${disk} exists, start partition"  | tee -a ${log}
        else
            echo "disk not exist, please choose correct disk"  | tee -a ${log}
            return 1
        fi
    else
        echo "no disk found" | tee -a ${log}
        return 1
    fi
    if ! CheckSpace; then
        echo "no enough space on ${disk}" | tee -a ${log}
        return 1
    fi

    return 0
}
"#;

pub const BOOTUP_MOUNT_2: &str = r#"function MountRoot() {
    echo "Mounting rootfs..."
    # mount rootfs
    mount "${disk}2" /sysroot >> "${log}" 2>&1
    if ! mount "${disk}2" /sysroot >> "${log}" 2>&1; then
        echo "mount rootfs failed" | tee -a "${log}"
        return 1
    fi

    return 0
}

function MountBoot() {
    echo "Mounting boot"
    mkdir -p /sysroot/boot/efi
    mount "${disk}1" /sysroot/boot/efi >> "${log}" 2>&1
    if ! mount "${disk}1" /sysroot/boot/efi >> "${log}" 2>&1; then
        echo "mount boot failed" | tee -a "${log}"
        return 1
    fi
    return 0
}

function GetRootfs() {
    echo "Downloading rootfs..."

    curl -o /${rootfs_name} http://${server_ip}/${rootfs_name}
    if [ ! -e "/${rootfs_name}" ]; then
        echo "download rootfs failed" | tee -a ${log}
        return 1
    fi

    if ! tar -xf /${rootfs_name} -C /sysroot; then
        echo "decompose rootfs failed" | tee -a ${log}
        return 1
    fi

    rm -rf "/${rootfs_name:?}"
    mount -o remount,ro ${disk}2 /sysroot  >> ${log} 2>&1
    return 0
}

function Inst_Grub2_x86() {
    # copy the files that boot need
    cp -r /sysroot/usr/lib/grub/x86_64-efi /sysroot/boot/efi/EFI/openEuler
    if ! eval "grub2-mkimage -d /sysroot/usr/lib/grub/x86_64-efi -O x86_64-efi --output=/sysroot/boot/efi/EFI/openEuler/grubx64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"  >> ${log} 2>&1; then
        echo "grub2-mkimage on x86 failed" | tee -a ${log}
        return 1
    fi
    
    mkdir -p /sysroot/boot/efi/EFI/BOOT/
    cp -f /sysroot/boot/efi/EFI/openEuler/grubx64.efi /sysroot/boot/efi/EFI/BOOT/BOOTX64.EFI

    return 0
}

function Inst_Grub2_aarch64() {
    cp -r /sysroot/usr/lib/grub/arm64-efi /sysroot/boot/efi/EFI/openEuler/
    eval "grub2-mkimage -d /sysroot/usr/lib/grub/arm64-efi -O arm64-efi --output=/sysroot/boot/efi/EFI/openEuler/grubaa64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"  >> ${log} 2>&1
    if ! eval "grub2-mkimage -d /sysroot/usr/lib/grub/arm64-efi -O arm64-efi --output=/sysroot/boot/efi/EFI/openEuler/grubaa64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"  >> ${log} 2>&1; then
        echo "grub2-mkimage on aarch64 failed" | tee -a ${log}
        return 1
    fi
    mkdir -p /sysroot/boot/efi/EFI/BOOT/
    cp -f /sysroot/boot/efi/EFI/openEuler/grubaa64.efi /sysroot/boot/efi/EFI/BOOT/BOOTAA64.EFI

    return 0
}

function set_partuuid() {
    grub_path="/sysroot/boot/efi/EFI/openEuler/grub.cfg"
    sed -i "s/vmlinuz root=\/dev\/vda2/vmlinuz root=PARTUUID=$ROOTA_PARTUUID/g" "$grub_path"
    sed -i "s/vmlinuz root=\/dev\/vda3/vmlinuz root=PARTUUID=$ROOTB_PARTUUID/g" "$grub_path"
    return 0
}

function SetBoot() {
    # mount boot
    echo "Setting boot"

    if ! set_partuuid; then
        echo "set partuuid failed" | tee -a "${log}"
        return 1
    fi

    if [ "$arch" == "x86_64" ]; then
        if ! Inst_Grub2_x86; then
            echo "install grub on x86 failed" | tee -a "${log}"
            return 1
        fi
    fi

    if [ "$arch" == "aarch64" ]; then
        if ! Inst_Grub2_aarch64; then
            echo "install grub on aarch64 failed" | tee -a "${log}"
            return 1
        fi
    fi

    return 0
}

function Bootup_Main() {
    # get disk
    echo "Checking disk info..." | tee -a "${log}"
    if ! GetDisk; then
        echo "Checking disk info failed" | tee -a "${log}"
        return 1
    fi

    # partition and format disk
    echo "Partion and formatting..." | tee -a "${log}"
    if ! PartitionAndFormatting; then
        echo "Partition and formatting disk failed" | tee -a "${log}"
        return 1
    fi

    # init network
    echo "Initializing network..." | tee -a "${log}"
    if ! InitNetwork; then
        echo "Initializing network failed" | tee -a "${log}"
        return 1
    fi
    
    # mount partitions
    echo "Mounting root..." | tee -a "${log}"
    if ! MountRoot; then
        echo "Mounting root failed" | tee -a "${log}"
        return 1
    fi

    # mount boot
    echo "Mounting boot..." | tee -a "${log}"
    if ! MountBoot; then
        echo "Mounting boot failed" | tee -a "${log}"
        return 1
    fi

    # download rootfs
    echo "Downloading rootfs..." | tee -a "${log}"
    if ! GetRootfs; then
        echo "Downloading rootfs failed" | tee -a "${log}"
        return 1
    fi
    mount_proc_dev_sys /sysroot
    # set boot
    echo "Setting boot..." | tee -a "${log}"
    if ! SetBoot; then
        echo "Setting boot failed" | tee -a "${log}"
        return 1
    fi
    # mount persist
    echo "Mounting persist..." | tee -a "${log}"
    if ! MountPersist; then
        echo "Mounting persist failed" | tee -a "${log}"
        return 1
    fi
    return 0
}

Bootup_Main
ret=$?
if [ ${ret} -eq 0 ]; then
    echo "KubeOS is installed successfully! Switch to root." | tee -a ${log}
    cp ${log} /sysroot/persist
else
    echo "Failed to install KubeOS, please check install.log." | tee -a ${log}
fi"#;

pub const INIT_NETWORK_PARTITION: &str = r#"function PartitionAndFormatting() {{
    echo "Partitioning and formatting disk $disk..."
    # partition and format
    parted "${{disk}}" -s mklabel gpt >> "${{log}}" 2>&1
    if ! parted "${{disk}}" -s mklabel gpt >> "${{log}}" 2>&1; then
        echo "partition failed" | tee -a "${{log}}"
        return 1
    fi

    if ! parted "${{disk}}" -s mkpart primary fat16 1MiB {PARTITION1_SIZE}MiB >> "${{log}}" 2>&1; then
        echo "partition failed" | tee -a "${{log}}"
        return 1
    fi

    if ! parted "${{disk}}" -s mkpart primary ext4 {PARTITION1_SIZE}MiB {PARTITION2_SIZE}MiB >> "${{log}}" 2>&1; then
        echo "partition failed" | tee -a "${{log}}"
        return 1
    fi

    if ! parted "${{disk}}" -s mkpart primary ext4 {PARTITION2_SIZE}MiB {PARTITION3_SIZE}MiB >> "${{log}}" 2>&1; then
        echo "partition failed" | tee -a "${{log}}"
        return 1
    fi

    if ! parted "${{disk}}" -s mkpart primary ext4 {PARTITION3_SIZE}MiB 100% >> "${{log}}" 2>&1; then
        echo "partition failed" | tee -a "${{log}}"
        return 1
    fi

    if ! parted "${{disk}}" -s set 1 boot on >> "${{log}}" 2>&1; then
        echo "partition failed" | tee -a "${{log}}"
        return 1
    fi

    if ! mkfs.vfat -n "BOOT" "${{disk}}1" >> "${{log}}" 2>&1; then
        echo "format failed" | tee -a "${{log}}"
        return 1
    fi

    if ! mkfs.ext4 -L "ROOT-A" "${{disk}}2" >> "${{log}}" 2>&1; then
        echo "format failed" | tee -a "${{log}}"
        return 1
    fi

    if ! mkfs.ext4 -L "ROOT-B" "${{disk}}3" >> "${{log}}" 2>&1; then
        echo "format failed" | tee -a "${{log}}"
        return 1
    fi

    if ! mkfs.ext4 -L "PERSIST" "${{disk}}4" >> "${{log}}" 2>&1; then
        echo "format failed" | tee -a "${{log}}"
        return 1
    fi

    ROOTA_PARTUUID=$(blkid "${{disk}}2" | awk -F 'PARTUUID="' '{{print $2}}' | awk -F '"' '{{print $1}}')
    ROOTB_PARTUUID=$(blkid "${{disk}}3" | awk -F 'PARTUUID="' '{{print $2}}' | awk -F '"' '{{print $1}}')

    return 0
}}

function InitNetwork() {{
    echo "Initializing network..."
    mapfile -t netNames < <(ifconfig -a | awk '{{print $1}}' | grep : | grep '^e' | awk -F: '{{print $1}}')
    {MANUAL_GET_IF_NAME}

    for netif in "${{netNames[@]}}";do
       echo "Setup ${{netif}} link up"
       if  ! ifconfig "${{netif}}" up; then
           echo "load ${{netif}} net card failed" | tee -a ${{log}}
           continue
       fi
    done
    sleep 3

    {SET_IP}
    sleep 3

    if ! route add default gw "${{route_ip}}" >> "${{log}}" 2>&1; then
        echo "add route failed" | tee -a "${{log}}"
        return 1
    fi
    sleep 3
    return 0
}}

function MountPersist() {{
    echo "Mounting persist"
    mount "${{disk}}4" /sysroot/persist >> "${{log}}" 2>&1
    if ! mount "${{disk}}4" /sysroot/persist >> "${{log}}" 2>&1; then
        echo "mount persist failed" | tee -a "${{log}}"
        return 1
    fi
    {MKDIR_COMMAND}
    mkdir -p /sysroot/persist/etc/KubeOS/certs
    return 0
}}
"#;

pub const MANUAL_GET_IF_NAME: &str = r#"
    if [ ${#netNames[*]} -gt 0 ]; then
        if [ -n "${net_name}" ] && echo "${netNames[@]}" | grep -wq "${net_name}" ; then
            echo "${net_name} exists, start set ip"  | tee -a "${log}"
        else
            echo "net_name not exist, choose default net"  | tee -a "${log}"
            net_name=${netNames[0]}
        fi
    else
        echo "no net Device found" | tee -a "${log}"
        return 1
    fi"#;

pub const DHCP_SET_IP: &str = r#"mkdir -p /var/lib/dhclient
    cat > "${dhcs}" <<EOF
#!/bin/sh
case "\${reason}" in

BOUND|RENEW|REBIND)
  echo "new ip:  \${new_ip_address}; \${new_subnet_mask}"
  ifconfig \${interface} \${new_ip_address} netmask \${new_subnet_mask}
  ;;
*)
  echo "cannot get new ip"
  ;;
esac
EOF
    echo "dhcp setup ip address" | tee -a "${log}"
    if [ -f "${dhcs}" ]; then
        tee -a "${log}" < "${dhcs}"
        chmod 755 "${dhcs}"
        if ! dhclient -sf "${dhcs}" -v >> "${log}" 2>&1; then
            echo "dhcp setup ip address failed" | tee -a "${log}"
            return 1
        fi
    fi
"#;

pub const MANUAL_SET_IP: &str = r#"
    if ! ifconfig "${net_name}" "${local_ip}" netmask "${netmask}" >> "${log}" 2>&1; then
        echo "ip set failed" | tee -a "${log}"
        return 1
    fi
"#;

pub const OS_TAR_DOCKERFILE: &str = r#"FROM scratch
COPY os.tar /
CMD ["/bin/sh"]"#;

pub const DMV_DOCKERFILE: &str = r#"FROM scratch
COPY ./update-boot.img ./update-hash.img ./update-root.img /
CMD ["/bin/sh"]"#;

pub const ADMIN_DOCKERFILE_CONTENT: &str = r#"FROM openeuler/openeuler:24.03-lts
RUN dnf upgrade -y && dnf -y install openssh-clients util-linux sysmaster
COPY ./set-ssh-pub-key.sh ./hostshell /usr/local/bin/
COPY ./set-ssh-pub-key.service /usr/lib/sysmaster/system
EXPOSE 22
RUN ln -s /usr/lib/sysmaster/system/set-ssh-pub-key.service /etc/sysmaster/system/multi-user.target.wants/set-ssh-pub-key.service
CMD ["/usr/lib/sysmaster/init"]"#;

pub const SET_SSH_PUB_KEY_SERVICE: &str = r#"[Unit]
Description=set ssh authorized keys according to the secret which is set by user

[Service]
ExecStart=/usr/local/bin/set-ssh-pub-key.sh"#;

pub const SET_SSH_PUB_KEY_SH: &str = r#"ssh_pub=$(cat /etc/secret-volume/ssh-pub-key)
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

echo "$ssh_pub" >> "$authorized_file""#;

pub const DMV_MOUNT_SH: &str = r#"set -x
export PATH=/bin:/sbin:/usr/bin:/usr/sbin

echo "create dm-verity device for rootfs..."

roothash=

function parse_kernel_args() {
	CMDLINE=$(cat /proc/cmdline)
	for param in $CMDLINE; do
		case "${param}" in
		root=*)
			root_device="${param}"
			echo "${root_device}"
			;;
		dmvroothash=*)
			roothash=$(echo "${param}" | cut -d'=' -f2 | tr -d '\n')
			echo "${roothash}"
			;;
		esac
	done
}

function try_another() {
	bootres=$(efibootmgr)
	bootcurrent=$(echo "$bootres" | grep "BootCurrent" | cut -d ':' -f2 | tr -d ' ')
	currentposition=$(echo "$bootres" | grep "Boot$bootcurrent" | cut -d '(' -f2 | cut -b 1)

	if [ "$currentposition" = "1" ]; then
		another=4
	elif [ "$currentposition" = "4" ]; then
		another=1
	else
		echo "reboot"
	fi

	exist=$(echo "$bootres" | grep "$another,GPT")
	if [ "$exist" = "" ]; then
		arch=$(arch)
		if [ "$arch" = "x86_64" ]; then
			efibootmgr -c -d "/dev/${DEVprefix}" -p "$another" -l "\EFI\openEuler\shimx64.efi"
		elif [ "$arch" = "aarch64" ]; then
			efibootmgr -c -d "/dev/${DEVprefix}" -p "$another" -l "\EFI\openEuler\shimaa64.efi"
		else
			echo "$arch not support"
			reboot
		fi
	fi
	anotherbootNum=$(efibootmgr | grep "$another,GPT" | cut -b 5-8)

	efibootmgr -o "$anotherbootNum,$bootcurrent"
	reboot
}

parse_kernel_args

root1=$(echo "${root_device}" | cut -d'=' -f2)
root2=$(echo "${root_device}" | cut -d'=' -f3)
final=""
DEVprefix="vda"
hwinfo --disk

if [ "${root2}" = "" ]; then
	final=${root1: -1}
	DEVprefix=$(echo "$root1" | cut -d'/' -f3 | sed 's/[^a-z]//g')
elif [ "${root1}" = "LABEL" ] && [ "${root2}" != "" ]; then
	root3=$(readlink "/dev/disk/by-label/$root2")
	final=${root3: -1}
	## shellcheck disable=SC2001
	DEVprefix=$(echo "$root3" | sed 's/[^a-z]//g')
elif [ "${root1}" = "UUID" ] && [ "${root2}" != "" ]; then
	root3=$(readlink "/dev/disk/by-uuid/$root2")
	final=${root3: -1}
	DEVprefix=$(echo "$root3" | sed 's/[^a-z]//g')
elif [ "${root1}" = "PARTUUID" ] && [ "${root2}" != "" ]; then
	root3=$(readlink "/dev/disk/by-partuuid/$root2")
	final=${root3: -1}
	DEVprefix=$(echo "$root3" | sed 's/[^a-z]//g')
else
	echo "${root_device}: root-device identifier error, parse failed"
fi

hashpart=$(echo "${final}+1" | bc)
veritysetup create kubeos-root "/dev/${DEVprefix}${final}" "/dev/${DEVprefix}${hashpart}" "${roothash}"
dmvstatus=$(veritysetup status kubeos-root | grep "status:" | cut -d : -f2 | tr -d " ")
if [ "$dmvstatus" = "verified" ]; then
	echo "dm-verity verify success! switch to kubeos-root...."
	mount /dev/mapper/kubeos-root /sysroot
else
	try_another
fi

if [ $? -ne 0 ]; then
	echo "mount rootfs failed"
	reboot
fi"#;

pub const DMV_MODULE_SETUP_SH: &str = r#"check() {
    return 0
}

install() {
    inst_multiple -o grub2-mkimage fdisk cpio veritysetup cut awk efibootmgr modprobe arch bc hwinfo partprobe
    inst_hook pre-mount 05 "$moddir/dmv-mount.sh"
}

installkernel() {
    hostonly='' \
        instmods \
        =drivers/scsi \
        =drivers/virtio \
        =drivers/block
}"#;

pub const DMV_CHROOT_NEW_GRUB_SH: &str = r#"set -x
WORKDIR="/tmp4grub"

function create_new_grubxx_efi() {

    ARCH=$(arch)
    if [ "$ARCH" == "x86_64" ]; then
        MODULES="all_video boot btrfs cat configfile cryptodisk echo efifwsetup efinet ext2 f2fs fat font gcry_rijndael gcry_rsa gcry_serpent gcry_sha256 gcry_sha512 gcry_twofish gcry_whirlpool gfxmenu gfxterm gzio halt hfsplus http iso9660 jpeg loadenv loopback linux lvm lsefi lsefimmap luks luks2 mdraid09 mdraid1x minicmd net normal part_apple part_msdos part_gpt password_pbkdf2 pgp png reboot regexp search search_fs_uuid search_fs_file search_label serial sleep syslinuxcfg test tftp video xfs zstd tpm backtrace chain usb usbserial_common usbserial_pl2303 usbserial_ftdi usbserial_usbdebug keylayouts at_keyboard"
        grub2-mkimage -d /usr/lib/grub/x86_64-efi -O x86_64-efi -p /EFI/openEuler --pubkey "${WORKDIR}/gpg.key" --output "${WORKDIR}/grubx64.efi" -c "${WORKDIR}"/grub.init.cfg --sbat "${WORKDIR}/sbat.csv" $MODULES 2>&1 | tee "${WORKDIR}/mkimage.log"
    fi

    if [ "$ARCH" == "aarch64" ]; then
        MODULES="all_video boot btrfs cat configfile cryptodisk echo efifwsetup efinet ext2 f2fs fat font gcry_rijndael gcry_rsa gcry_serpent gcry_sha256 gcry_sha512 gcry_twofish gcry_whirlpool gfxmenu gfxterm gzio halt hfsplus http iso9660 jpeg loadenv loopback linux lvm lsefi lsefimmap luks luks2 mdraid09 mdraid1x minicmd net normal part_apple part_msdos part_gpt password_pbkdf2 pgp png reboot regexp search search_fs_uuid search_fs_file search_label serial sleep syslinuxcfg test tftp video xfs zstd tpm"
        grub2-mkimage -d /usr/lib/grub/arm64-efi -O arm64-efi -p /EFI/openEuler --pubkey "${WORKDIR}/gpg.key" --output "${WORKDIR}/grubaa64.efi" -c "${WORKDIR}/grub.init.cfg" --sbat "${WORKDIR}/sbat.csv" $MODULES 2>&1 | tee "${WORKDIR}/mkimage.log"
    fi

    if [ $? -ne 0 ]; then
        echo "create grubxx.efi failed"
        return 7
    fi
}

create_new_grubxx_efi"#;

pub const DMV_MAIN_SH: &str = r#"set -e

CURDIR=$(cd "$(dirname $0)";pwd)
PWDDD=$(cd "$CURDIR/"; pwd)
KEYDIR="$PWDDD/dm-verity/keys"
CERTDB="$KEYDIR/certdb"
BIOSkeyname="rsa4BIOS"
WORKDIR="$PWDDD/dm-verity/tmp"
GPGkeyid="gpgKey4kubeos"
GPG_KEY=""

function keys_exist() {
    keyExist=True
    if [ ! -d "${KEYDIR}" ]; then
        keyExist=False
        mkdir -p "${KEYDIR}"
    else
        for file in "${BIOSkeyname}.der" "gpg.key" "gpg.log"
        do
            if [ ! "$(find "${KEYDIR}" -name "${file}")" ]; then
                keyExist=False
                break
            fi
        done
        if [ ! -d "${CERTDB}" ]; then
            keyExist=False
        fi
    fi
    echo ${keyExist}
}

function gpg_key_gen() {

    GPG_PASSWORD=$1

    id=$(gpg --list-keys | grep "${GPGkeyid}" | awk '{ print $3 }')
    if [ "$id" == ${GPGkeyid} ];then
		fgpt=$(gpg --with-colons --fingerprint gpgKey4kubeos | grep -m 1 "^fpr" | sed -n 's/^fpr:::::::::\([[:alnum:]]\+\):/\1/p')
        gpg --batch --yes --delete-secret-keys "$fgpt"
        gpg --batch --yes --delete-keys "$fgpt"
    fi

    cat > "${KEYDIR}/gpg.batch.file" << EOF
Key-Type: RSA
Key-Length: 4096
Subkey-Type: RSA
Subkey-Length: 4096
Name-Real: ${GPGkeyid}
Expire-Date: 0
Passphrase: ${GPG_PASSWORD}
EOF

    gpg --batch --gen-key "${KEYDIR}/gpg.batch.file"
	gpg --list-keys --keyid-format LONG ${GPGkeyid} | grep pub > "${KEYDIR}/gpg.log"
	GPG_KEY=$(gpg --list-keys --keyid-format LONG ${GPGkeyid} | grep pub | awk -F 'rsa4096/' '{print $2}' | cut -b 1-16)
	if [ "$GPG_KEY" = "" ]; then
		echo "GPG-key-gen ID failed"
		return 7
	fi
	gpg --export "$GPG_KEY" > "${KEYDIR}/gpg.key"
    rm -f "${KEYDIR}/gpg.batch.file"
    if [ $? -ne 0 ]; then
        echo "GPG-key-gen failed"
        return 7
    fi
}

function BIOS_key_gen() {

    PIN_PASSWORD=$1
    keyname=$BIOSkeyname

    if [ -d "${CERTDB}" ]; then
        rm -rf "${CERTDB}"
	fi
    mkdir -p "${CERTDB}"
    cat > "${KEYDIR}/pinfile" << EOF
$PIN_PASSWORD
EOF

    openssl genrsa -out "${KEYDIR}/${keyname}.key" 4096
    openssl req -new -key "${KEYDIR}/${keyname}.key" -out "${KEYDIR}/${keyname}.csr" -subj '/C=AA/ST=BB/O=CC/OU=DD/CN=BIOS-cert-for-kubeos-secure-boot'
    openssl x509 -req -days 365 -in "${KEYDIR}/${keyname}.csr" -signkey "${KEYDIR}/${keyname}.key" -out "${KEYDIR}/${keyname}.crt"
    openssl x509 -in "${KEYDIR}/${keyname}.crt" -out "${KEYDIR}/${keyname}.der" -outform der

    certutil -N -d "${CERTDB}" -f "${KEYDIR}/pinfile"
    certutil -A -n ${keyname} -d "${CERTDB}" -t CT,CT,CT -i "${KEYDIR}/${keyname}.crt" -f "${KEYDIR}/pinfile"
    openssl pkcs12 -export -out "${KEYDIR}/${keyname}.p12" -inkey "${KEYDIR}/${keyname}.key" -in "${KEYDIR}/${keyname}.crt" -password pass:"${PIN_PASSWORD}"
    pk12util -d "${CERTDB}" -i "${KEYDIR}/${keyname}.p12" -w "${KEYDIR}/pinfile" -k "${KEYDIR}/pinfile"

    rm -f "${KEYDIR}/pinfile"
    rm -f "${KEYDIR}/${keyname}.p12"
    rm -f "${KEYDIR}/${keyname}.crt"
    rm -f "${KEYDIR}/${keyname}.csr"
    rm -f "${KEYDIR}/${keyname}.key"

    if [ $? -ne 0 ]; then
        echo "BIOS-key-gen failed"
        return 7
    fi
}

function create_new_grubxx_efi() {

    GRUB_PASSWORD=$2
    GPG_PASSWORD=$1
    GRUB_version=2.06

    GRUB_PASSWORD_HASH=$(echo -e "$GRUB_PASSWORD\n$GRUB_PASSWORD" | grub2-mkpasswd-pbkdf2 | grep -o "grub.*")

	loopN=$(losetup -f)
	losetup -P "$loopN" "${PWDDD}/system.img"
	bootUUID=$(blkid | grep "${loopN}p1" | awk -F ' UUID=' '{print $2}' | cut -b 2-10)
	losetup -d "${loopN}"

	cat > "${WORKDIR}/grub.init.cfg" << EOF
#set debug=linux,linuxefi,crypt
#export debug
set check_signatures=enforce
export check_signatures
set superusers=root
export superusers
set prefix='/EFI/openEuler'
export prefix
password_pbkdf2 root $GRUB_PASSWORD_HASH
set root='hd0,gpt1'
search --no-floppy --fs-uuid --set=root $bootUUID
echo "now in grub-init......1....."
configfile /EFI/openEuler/grub.cfg
echo /EFI/openEuler/grub.cfg did not boot the system, rebooting the system in 10 seconds..
sleep 10
reboot
EOF

    cat > "${WORKDIR}/sbat.csv" << EOF
sbat,1,SBAT Version,sbat,1,https://github.com/rhboot/shim/blob/main/SBAT.md
grub,4,Free Software Foundation,grub,$GRUB_version,https//www.gnu.org/software/grub/
grub.openeuler,1,The openEuler Project,grub2,$GRUB_version-0,https://gitee.com/src-openeuler/grub2
EOF

    gpg --pinentry-mode=loopback --passphrase "${GPG_PASSWORD}" --default-key "$GPG_KEY" --detach-sign "${WORKDIR}/grub.init.cfg"

	if [ $? -ne 0 ]; then
		echo "prepare new grub files failed"
		return 7
	fi
}

function sign_efi_imgs() {

    PIN_PASSWORD=$1
    GRUB_PASSWORD2=$3
    GPG_PASSWORD=$2
    cat > "${WORKDIR}/pinfile" << EOF
$PIN_PASSWORD
EOF

    tmpRoot="${WORKDIR}/tmproot"
    tmpBoot="${WORKDIR}/tmpboot"
    mkdir -p "${tmpRoot}"
    mkdir -p "${tmpBoot}"

    loopX=$(losetup -f)
    losetup -P "${loopX}" "$PWDDD/system.img"
    mount "${loopX}p1" "$tmpBoot"
    mount "${loopX}p2" "$tmpRoot"

	ARCH=$(arch)
    suffix=
    if [ "$ARCH" == "x86_64" ]; then
        suffix="x64.efi"
    elif [ "$ARCH" == "aarch64" ]; then
        suffix="aa64.efi"
    else
        echo "ARCH $ARCH not support currently"
	return 7
	fi

	mkdir -p "$tmpRoot/tmp4grub"
	cp "${WORKDIR}/grub.init.cfg" "$tmpRoot/tmp4grub/grub.init.cfg"
	cp "${WORKDIR}/grub.init.cfg.sig" "$tmpRoot/tmp4grub/grub.init.cfg.sig"
	cp "${WORKDIR}/sbat.csv" "$tmpRoot/tmp4grub/sbat.csv"
	cp "${KEYDIR}/gpg.key" "$tmpRoot/tmp4grub/gpg.key"
	cp "${PWDDD}/dm-verity/chroot_new_grub.sh" "$tmpRoot/chroot_new_grub.sh"
	chroot "$tmpRoot" bash /chroot_new_grub.sh
	cp "$tmpRoot/tmp4grub/grub$suffix" "${WORKDIR}"
	rm -rf "$tmpRoot/tmp4grub"
	rm -f "$tmpRoot/chroot_new_grub.sh"

	bootuuid=$(blkid | grep "${loopX}p1" | awk -F ' UUID=' '{print $2}' | cut -b 2-10)
	sed -i "s/What=\/dev\/disk\/by-label\/BOOT/What=\/dev\/disk\/by-uuid\/$bootuuid/g" "$tmpRoot/usr/lib/systemd/system/boot-efi.mount"

	if [ $? -ne 0 ]; then
		echo "create grubxx.efi failed"
		return 7
	fi

    IMGs="shim fb mm"
    for img in $IMGs
    do
        /bin/cp "$tmpBoot/EFI/openEuler/$img$suffix" "${WORKDIR}"
    done

    IMGs="$IMGs grub"
    for img in $IMGs
    do
        pesign -n "${CERTDB}" -c ${BIOSkeyname} --pinfile "${WORKDIR}/pinfile" -s -i "$WORKDIR/$img$suffix" -o "${WORKDIR}/$img$suffix.signed"
        /bin/cp "${WORKDIR}/$img$suffix.signed" "$tmpBoot/EFI/openEuler/$img$suffix"
    done

    if [ $? -ne 0 ]; then
        echo "pesign efi failed"
        return 7
    fi

    /bin/cp "$PWDDD/grub.cfg" "${WORKDIR}"
    if [ "$ARCH" == "x86_64" ]; then
        /bin/cp "$tmpRoot/boot/vmlinuz" "${WORKDIR}"
        /bin/cp "$tmpRoot/boot/initramfs-verity.img" "${WORKDIR}"
    elif [ "$ARCH" == "aarch64" ]; then
        /bin/cp "$tmpRoot/boot/vmlinuz" "${WORKDIR}/vmlinuz.gz"
        /bin/cp "$tmpRoot/boot/initramfs-verity.img" "${WORKDIR}/initramfs-verity.img"
        gzip -d "${WORKDIR}/vmlinuz.gz"
    else
        echo "ARCH $ARCH not support currently"
    fi

    if [ $? -ne 0 ]; then
        echo "copy/gzip -d failed"
        return 7
    fi

    pesign -n "${CERTDB}" -c ${BIOSkeyname} --pinfile "${WORKDIR}/pinfile" -s -i "$WORKDIR/vmlinuz" -o "${WORKDIR}/vmlinuz.signed"
    gpg --pinentry-mode=loopback --passphrase "${GPG_PASSWORD}" --default-key "$GPG_KEY" --detach-sign "${WORKDIR}/vmlinuz.signed"
    gpg --pinentry-mode=loopback --passphrase "${GPG_PASSWORD}" --default-key "$GPG_KEY" --detach-sign "${WORKDIR}/initramfs-verity.img"

    if [ $? -ne 0 ]; then
        echo "gpg sign failed"
        return 7
    fi

    if [ "$ARCH" == "x86_64" ]; then
        /bin/cp "${WORKDIR}/vmlinuz.signed" "$tmpRoot/boot/vmlinuz"
        /bin/cp "${WORKDIR}/vmlinuz.signed.sig" "$tmpRoot/boot/vmlinuz.sig"
        /bin/cp "${WORKDIR}/initramfs-verity.img" "$tmpRoot/boot/initramfs-verity.img"
        /bin/cp "${WORKDIR}/initramfs-verity.img.sig" "$tmpRoot/boot/initramfs-verity.img.sig"
    elif [ "$ARCH" == "aarch64" ]; then
        gzip "${WORKDIR}/vmlinuz.signed"
        /bin/cp "${WORKDIR}/vmlinuz.signed.gz" "$tmpRoot/boot/vmlinuz"
        /bin/cp "${WORKDIR}/vmlinuz.signed.sig" "$tmpRoot/boot/vmlinuz.sig"
        /bin/cp "${WORKDIR}/initramfs-verity.img" "$tmpRoot/boot/initramfs-verity.img"
        /bin/cp "${WORKDIR}/initramfs-verity.img.sig" "$tmpRoot/boot/initramfs-verity.img.sig"
    else
        echo "ARCH $ARCH not support currently"
    fi

    if [ $? -ne 0 ]; then
        echo "copy/gzip back(vmlinuz/initramfs) failed"
        return 7
    fi

	ssh-keygen -t rsa -f "$tmpRoot/etc/ssh/ssh_host_rsa_key" -N ''
	ssh-keygen -t ecdsa -f "$tmpRoot/etc/ssh/ssh_host_ecdsa_key" -N ''
	ssh-keygen -t ed25519 -f "$tmpRoot/etc/ssh/ssh_host_ed25519_key" -N ''
    sync
    umount "$tmpRoot"

    veritysetup format "${loopX}p2" "${loopX}p3" --root-hash-file="${WORKDIR}/roothash"
    veritysetup verify "${loopX}p2" "${loopX}p3" --root-hash-file="${WORKDIR}/roothash" --debug

    dmvroothash=$(cat "${WORKDIR}/roothash")

	vzlines=$(grep -n boot/vmlinuz "${WORKDIR}/grub.cfg" | cut -d : -f1)
	for ln in $vzlines
	do
		ori=$(sed -n "${ln}p" "${WORKDIR}/grub.cfg")
		sed -i "${ln}c \ ${ori} dmvroothash=${dmvroothash}" "${WORKDIR}/grub.cfg"
	done

    GRUB_PASSWORD_HASH2=$(echo -e "$GRUB_PASSWORD2\n$GRUB_PASSWORD2" | grub2-mkpasswd-pbkdf2 | grep -o "grub.*")
    grub2pwhline=$(awk '/password_pbkdf2/{print NR}' "${WORKDIR}/grub.cfg")
    oripwh=$(sed -n "${grub2pwhline}p" "${WORKDIR}/grub.cfg")
    sed -i "${grub2pwhline}c ${oripwh} ${GRUB_PASSWORD_HASH2}" "${WORKDIR}/grub.cfg"

    gpg --pinentry-mode=loopback --passphrase "${GPG_PASSWORD}" --default-key "$GPG_KEY" --detach-sign "${WORKDIR}/grub.cfg"

    if [ $? -ne 0 ]; then
        echo "modify grub.cfg failed"
        return 7
    fi

    /bin/cp "${WORKDIR}/grub.cfg" "$tmpBoot/EFI/openEuler/grub.cfg"
    /bin/cp "${WORKDIR}/grub.cfg.sig" "$tmpBoot/EFI/openEuler/grub.cfg.sig"
    /bin/cp "$tmpBoot/EFI/openEuler/fb$suffix" "$tmpBoot/EFI/BOOT/fb$suffix"
    /bin/cp "$tmpBoot/EFI/openEuler/mm$suffix" "$tmpBoot/EFI/BOOT/mm$suffix"
    if [ "$ARCH" == "x86_64" ]; then
        /bin/cp "$tmpBoot/EFI/openEuler/shim$suffix" "$tmpBoot/EFI/BOOT/BOOTX64.EFI"
    elif [ "$ARCH" == "aarch64" ]; then
        /bin/cp "$tmpBoot/EFI/openEuler/shim$suffix" "$tmpBoot/EFI/BOOT/BOOTAA64.EFI"
    else
        echo "ARCH $ARCH not support currently"
        return 7
    fi
    /bin/cp "${KEYDIR}/${BIOSkeyname}.der" "$tmpBoot/EFI/${BIOSkeyname}.der"
    if [ $? -ne 0 ]; then
        echo "copy back efi failed"
        return 7
    fi

    sync
    umount "$tmpBoot"
    dd if="${loopX}p2" of="$PWDDD"/update-root.img bs=8M
	dd if="${loopX}p1" of="$PWDDD"/update-boot.img bs=8M
    dd if="${loopX}p3" of="$PWDDD"/update-hash.img bs=8M
	losetup -D
    rm -rf "${WORKDIR}"
}

function dmvmain() {
    set -e
    umask 0177

    if [ -d "${WORKDIR}" ]; then
        rm -rf "${WORKDIR}"
	fi
    mkdir -m 750 -p "${WORKDIR}"

    BIOSpwd=$1
    GPGpwd=$2
    GRUBpwd=$3
    if [ "$(keys_exist)" == "False" ]; then
        rm -rf "${KEYDIR}"
		mkdir -p "${KEYDIR}"
        gpg_key_gen "$GPGpwd"
        BIOS_key_gen "$BIOSpwd"
    fi

    GPG_KEY=$(gpg --list-keys --keyid-format LONG ${GPGkeyid} | grep pub | awk -F 'rsa4096/' '{print $2}' | cut -b 1-16)
    rm -f "${KEYDIR}/gpg.key"
    gpg --export "$GPG_KEY" > "${KEYDIR}/gpg.key"
    create_new_grubxx_efi "$GPGpwd" "$GRUBpwd"
    sign_efi_imgs "$BIOSpwd" "$GPGpwd" "$GRUBpwd"
}"#;

pub const DMV_UPGRADE_ROLLBACK_SH: &str = r#"set -eux
if [ "$1" != "upgrade" ] && [ "$1" != "switch" ]; then
    echo "Invalid argument: $1"
    exit 1
fi

currentROOT=$(veritysetup status kubeos-root | grep "data device" | cut -d / -f3 | tr -d " ")
currentROOTdev=$(echo "$currentROOT" | sed 's/[^a-z]//g')
currentROOTpart=${currentROOT: -1}

if [ "$currentROOTpart" = "2" ]; then
    nextBOOTpart="4"
    nextROOTpart="5"
    nextHASHpart="6"
    grubBOOT="B"
elif [ "$currentROOTpart" = "5" ]; then
    nextBOOTpart="1"
    nextROOTpart="2"
    nextHASHpart="3"
    grubBOOT="A"
else
    echo "Current ROOT [$currentROOTpart] error"
    return 7
fi

if [ "$1" = "upgrade" ]; then
    dd if=/persist/update-boot.img of=/dev/"$currentROOTdev$nextBOOTpart" bs=8M
    dd if=/persist/update-root.img of=/dev/"$currentROOTdev$nextROOTpart" bs=8M
    dd if=/persist/update-hash.img of=/dev/"$currentROOTdev$nextHASHpart" bs=8M
    exit
fi

mkdir -p /tmp/tmp4BOOT
mount /dev/"$currentROOTdev$nextBOOTpart" /tmp/tmp4BOOT
grub2-editenv /tmp/tmp4BOOT/EFI/openEuler/grubenv set saved_entry="$grubBOOT"
umount /tmp/tmp4BOOT

bootres=$(efibootmgr)
exist=$(echo "$bootres" | grep "$nextBOOTpart,GPT" || true)
if [ -z "$exist" ]; then
    arch=$(arch)
    if [ "$arch" = "x86_64" ]; then
        efibootmgr -c -d "/dev/$currentROOTdev" -p "$nextBOOTpart" -l "\EFI\openEuler\shimx64.efi" -L openEuler
    elif [ "$arch" = "aarch64" ]; then
        efibootmgr -c -d "/dev/$currentROOTdev" -p "$nextBOOTpart" -l "\EFI\openEuler\shimaa64.efi" -L openEuler
    else
        echo "$arch not support"
        return 7
    fi
fi
nextbootNum=$(efibootmgr | grep "$nextBOOTpart,GPT" | cut -b 5-8)
bootcurrent=$(efibootmgr | grep "BootCurrent" | cut -d ':' -f2 | tr -d ' ')
efibootmgr -o "$nextbootNum,$bootcurrent""#;

pub const BOOT_EFI_MOUNT: &str = r#"[Unit]
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
WantedBy=local-fs.target"#;

pub const BOOT_GRUB2_MOUNT: &str = r#"[Unit]
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
WantedBy=local-fs.target"#;

pub const ETC_MOUNT: &str = r#"[Unit]
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
WantedBy=local-fs.target"#;

pub const OPT_CNI_MOUNT: &str = r#"[Unit]
Description=opt cni Dir
DefaultDependencies=no
Conflicts=umount.target
Before=local-fs.target umount.target
Wants=persist.mount
After=persist.mount

[Mount]
What=overlay
Where=/opt/cni
Type=overlay
Options=upperdir=/persist/opt,lowerdir=/opt/cni,workdir=/persist/optwork

[Install]
WantedBy=local-fs.target"#;

pub const OS_AGENT_SERVICE: &str = r#"[Unit]
Description=Agent For KubeOS

[Service]
Environment=GOTRACEBACK=crash
ExecStart=/usr/bin/os-agent
KillMode=process
Restart=on-failure

[Install]
WantedBy=multi-user.target"#;

pub const PERSIST_MOUNT: &str = r#"[Unit]
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
WantedBy=local-fs.target"#;

pub const VAR_MOUNT: &str = r#"[Unit]
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
WantedBy=local-fs.target"#;

pub const DMV_MAIN_GRUB_CFG: &str = r#"set pager=1

set superusers="root"
export superusers
password_pbkdf2 root

WHITELIST="boot_success saved_entry boot_indeterminate prev_saved_entry next_entry feature_menuentry_id boot_once feature_all_video_module menu_show_once feature_timeout_style menu_auto_hide menu_hide_ok fastboot config_directory"

if [ -f ${prefix}/grubenv ]; then
  load_env -f ${prefix}/grubenv --skip-sig $WHITELIST
fi
if [ "${next_entry}" ] ; then
   set default="${next_entry}"
   set next_entry=
   save_env next_entry
   set boot_once=true
else
   set default="${saved_entry}"
fi

if [ x"${feature_menuentry_id}" = xy ]; then
  menuentry_id_option="--id"
else
  menuentry_id_option=""
fi

export menuentry_id_option

if [ "${prev_saved_entry}" ]; then
  set saved_entry="${prev_saved_entry}"
  save_env saved_entry
  set prev_saved_entry=
  save_env prev_saved_entry
  set boot_once=true
fi

terminal_output console
if [ x$feature_timeout_style = xy ] ; then
  set timeout_style=menu
  set timeout=5
else
  set timeout=5
fi

menuentry 'A' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-A' {
        set gfxpayload=keep
        set root='hd0,gpt2'
       linux   /boot/vmlinuz root=/dev/vda2 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3 console=ttyS0 apparmor=0
        initrd  /boot/initramfs-verity.img
}

menuentry 'B' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-B' {
        set gfxpayload=keep
        set root='hd0,gpt5'
       linux   /boot/vmlinuz root=/dev/vda5 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3 console=ttyS0 apparmor=0
        initrd  /boot/initramfs-verity.img
}

### END /etc/grub.d/10_linux ###

### BEGIN /etc/grub.d/10_reset_boot_success ###
# Hiding the menu is ok if last boot was ok or if this is a first boot attempt to boot the entry
if [ "${boot_success}" = "1" -o "${boot_indeterminate}" = "1" ]; then
  set menu_hide_ok=1
else
  set menu_hide_ok=0
fi
# Reset boot_indeterminate after a successful boot
if [ "${boot_success}" = "1" ] ; then
  set boot_indeterminate=0
# Avoid boot_indeterminate causing the menu to be hidden more then once
elif [ "${boot_indeterminate}" = "1" ]; then
  set boot_indeterminate=2
fi
# Reset boot_success for current boot
set boot_success=0
save_env boot_success boot_indeterminate
### END /etc/grub.d/10_reset_boot_success ###

### BEGIN /etc/grub.d/12_menu_auto_hide ###
if [ x$feature_timeout_style = xy ] ; then
  if [ "${menu_show_once}" ]; then
    unset menu_show_once
    save_env menu_show_once
    set timeout_style=menu
    set timeout=60
  elif [ "${menu_auto_hide}" -a "${menu_hide_ok}" = "1" ]; then
    set orig_timeout_style=${timeout_style}
    set orig_timeout=${timeout}
    if [ "${fastboot}" = "1" ]; then
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
if [ -f  ${config_directory}/custom.cfg ]; then
  source ${config_directory}/custom.cfg
elif [ -z "${config_directory}" -a -f  ${prefix}/custom.cfg ]; then
  source ${prefix}/custom.cfg;
fi
### END /etc/grub.d/41_custom ###"#;

pub const GRUB_CFG_CONTENTS: &str = r#"set pager=1

if [ -f ${config_directory}/grubenv ]; then
  load_env -f ${config_directory}/grubenv
elif [ -s $prefix/grubenv ]; then
  load_env
fi
if [ "${next_entry}" ] ; then
   set default="${next_entry}"
   set next_entry=
   save_env next_entry
   set boot_once=true
else
   set default="${saved_entry}"
fi

if [ x"${feature_menuentry_id}" = xy ]; then
  menuentry_id_option="--id"
else
  menuentry_id_option=""
fi

export menuentry_id_option

if [ "${prev_saved_entry}" ]; then
  set saved_entry="${prev_saved_entry}"
  save_env saved_entry
  set prev_saved_entry=
  save_env prev_saved_entry
  set boot_once=true
fi

function savedefault {
  if [ -z "${boot_once}" ]; then
    saved_entry="${chosen}"
    save_env saved_entry
  fi
}

function load_video {
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
}

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
if [ -f ${prefix}/user.cfg ]; then
  source ${prefix}/user.cfg
  if [ -n "${GRUB2_PASSWORD}" ]; then
    set superusers="root"
    export superusers
    password_pbkdf2 root ${GRUB2_PASSWORD}
  fi
fi
### END /etc/grub.d/01_users ###

### BEGIN /etc/grub.d/10_linux ###
menuentry 'A' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-A' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        search --no-floppy --label ROOT-A --set=root
        linux   /boot/vmlinuz root=/dev/vda2 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3
        initrd  /boot/initramfs.img
}

menuentry 'B' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-B' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        search --no-floppy --label ROOT-B --set=root
        linux   /boot/vmlinuz root=/dev/vda3 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3
        initrd  /boot/initramfs.img
}

### END /etc/grub.d/10_linux ###

### BEGIN /etc/grub.d/10_reset_boot_success ###
# Hiding the menu is ok if last boot was ok or if this is a first boot attempt to boot the entry
if [ "${boot_success}" = "1" -o "${boot_indeterminate}" = "1" ]; then
  set menu_hide_ok=1
else
  set menu_hide_ok=0
fi
# Reset boot_indeterminate after a successful boot
if [ "${boot_success}" = "1" ] ; then
  set boot_indeterminate=0
# Avoid boot_indeterminate causing the menu to be hidden more then once
elif [ "${boot_indeterminate}" = "1" ]; then
  set boot_indeterminate=2
fi
# Reset boot_success for current boot
set boot_success=0
save_env boot_success boot_indeterminate
### END /etc/grub.d/10_reset_boot_success ###

### BEGIN /etc/grub.d/12_menu_auto_hide ###
if [ x$feature_timeout_style = xy ] ; then
  if [ "${menu_show_once}" ]; then
    unset menu_show_once
    save_env menu_show_once
    set timeout_style=menu
    set timeout=60
  elif [ "${menu_auto_hide}" -a "${menu_hide_ok}" = "1" ]; then
    set orig_timeout_style=${timeout_style}
    set orig_timeout=${timeout}
    if [ "${fastboot}" = "1" ]; then
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
if [ -f  ${config_directory}/custom.cfg ]; then
  source ${config_directory}/custom.cfg
elif [ -z "${config_directory}" -a -f  $prefix/custom.cfg ]; then
  source $prefix/custom.cfg;
fi
### END /etc/grub.d/41_custom ###"#;
