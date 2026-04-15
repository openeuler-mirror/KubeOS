#!/bin/bash
## Copyright (c) Huawei Technologies Co., Ltd. 2026. All rights reserved.
# KubeOS is licensed under the Mulan PSL v2.
# You can use this software according to the terms and conditions of the Mulan PSL v2.
# You may obtain a copy of Mulan PSL v2 at:
#     http://license.coscl.org.cn/MulanPSL2
# THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
# PURPOSE.
## See the Mulan PSL v2 for more details.

set -e
set -x

SCRIPT_NAME="KubeOS镜像制作脚本"
SCRIPT_VERSION="1.0.0"

SCRIPTS_DIR=$(cd "$(dirname "$(readlink -f "$0")")" && pwd)
KUBEOS_PATH=$(dirname "$SCRIPTS_DIR")
DEFAULT_WORK_DIR="/home/KubeOS"
DEFAULT_NEW_ROOT="/home/KubeOS/new_root"
LOCK_FILE="/var/run/create_kubeos_image.lock"
LOCK_FD=200

WORK_DIR=""
NEW_ROOT=""
DEBUG_MODE=false
IN_CHROOT=false

MOUNT_POINTS=()
IGNITION_RPM=""
REPO_FILE=""
GRUB_CFG=""
KBIMG_TOML="$SCRIPTS_DIR/kbimg.toml"

acquire_lock() {
    exec 200>"$LOCK_FILE"
    if ! flock -n 200; then
        log_error "另一个实例正在运行，脚本不允许并发执行"
        log_error "锁文件: $LOCK_FILE"
        exit 1
    fi
    log_debug "已获取文件锁: $LOCK_FILE"
}

release_lock() {
    if [[ -f "$LOCK_FILE" ]]; then
        rm -f "$LOCK_FILE" 2>/dev/null || true
    fi
    log_debug "已释放文件锁"
}

cleanup() {
    local exit_code=$?
    
    trap - INT TERM EXIT
    
    if [[ "$DEBUG_MODE" == "true" ]]; then
        echo "[DEBUG] Debug模式开启，跳过环境清理"
        release_lock
        return 0
    fi
    
    echo "[INFO] 开始清理环境..."
    
    if [[ "$IN_CHROOT" == "true" ]]; then
        echo "[INFO] 退出chroot环境..."
        exit 0 2>/dev/null || true
        IN_CHROOT=false
    fi
    
    for mount_point in "${MOUNT_POINTS[@]}"; do
        if mountpoint -q "$mount_point" 2>/dev/null; then
            echo "[INFO] 卸载: $mount_point"
            umount -l "$mount_point" 2>/dev/null || true
        fi
    done
    
    echo "[INFO] 环境清理完成"
    release_lock
    return $exit_code
}

trap 'echo "[INFO] 收到中断信号..."; cleanup; exit 130' INT TERM
trap 'cleanup' EXIT

usage() {
    cat << EOF
用法: $0 [选项]

选项:
    -w, --work-dir PATH       工作目录 (默认: $DEFAULT_WORK_DIR)
    -i, --ignition-rpm FILE   ignition RPM包路径 (必需)
    -r, --repo-file FILE      yum源repo文件路径 (必需)
    -g, --grub-cfg FILE       grub.cfg文件路径 (必需)
    -d, --debug               Debug模式，报错时不清理环境
    -h, --help                显示帮助信息
    -v, --version             显示版本信息

示例:
    $0 -i /path/to/ignition.rpm -r /path/to/repo.repo -g /path/to/grub.cfg
    $0 --debug -i ignition.rpm -r repo.repo -g grub.cfg
EOF
}

log_info() {
    echo "[INFO] $1"
}

log_error() {
    echo "[ERROR] $1" >&2
}

log_warn() {
    echo "[WARN] $1"
}

log_debug() {
    if [[ "$DEBUG_MODE" == "true" ]]; then
        echo "[DEBUG] $1"
    fi
}

check_command() {
    if ! command -v "$1" &>/dev/null; then
        log_error "命令 '$1' 未找到，请先安装"
        exit 1
    fi
}

check_file() {
    if [[ ! -f "$1" ]]; then
        log_error "文件不存在: $1"
        exit 1
    fi
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -w|--work-dir)
                WORK_DIR="$2"
                shift 2
                ;;
            -i|--ignition-rpm)
                IGNITION_RPM="$2"
                shift 2
                ;;
            -r|--repo-file)
                REPO_FILE="$2"
                shift 2
                ;;
            -g|--grub-cfg)
                GRUB_CFG="$2"
                shift 2
                ;;
            -d|--debug)
                DEBUG_MODE=true
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            -v|--version)
                echo "$SCRIPT_NAME v$SCRIPT_VERSION"
                exit 0
                ;;
            *)
                log_error "未知参数: $1"
                usage
                exit 1
                ;;
        esac
    done
    
    WORK_DIR="${WORK_DIR:-$DEFAULT_WORK_DIR}"
    NEW_ROOT="${WORK_DIR}/new_root"
    
    if [[ -z "$IGNITION_RPM" ]]; then
        log_error "必须指定ignition RPM包路径 (-i/--ignition-rpm)"
        usage
        exit 1
    fi
    
    if [[ -z "$REPO_FILE" ]]; then
        log_error "必须指定yum源repo文件路径 (-r/--repo-file)"
        usage
        exit 1
    fi
    
    if [[ -z "$GRUB_CFG" ]]; then
        log_error "必须指定grub.cfg文件路径 (-g/--grub-cfg)"
        usage
        exit 1
    fi
    
    check_file "$IGNITION_RPM"
    check_file "$REPO_FILE"
    check_file "$GRUB_CFG"
}

check_prerequisites() {
    log_info "检查系统环境..."
    
    check_command dnf
    check_command chroot
    check_command mount
    check_command umount
    check_command rpm
    check_command dracut
    
    if [[ $EUID -ne 0 ]]; then
        log_error "此脚本需要root权限运行"
        exit 1
    fi
    
    log_info "系统环境检查通过"
}

generate_kbimg_toml() {
    local initramfs_name="$1"
    local toml="$KBIMG_TOML"
    
    log_info "生成kbimg.toml..."
    
    if [[ -f "$toml" ]]; then
        cp "$toml" "${toml}.bak"
    fi
    
    cat > "$toml" << EOF
[from_repo]
agent_path = "../bin/os-agent"
legacy_bios = false
repo_path = "/etc/yum.repos.d/openEuler.repo"
root_passwd = "\$1\$xyz\$RdLyKTL32WEvK3lg8CXID0"
rpmlist = [
    "NetworkManager",
    "cloud-init",
    "conntrack-tools",
    "containerd",
    "containernetworking-plugins",
    "cri-tools",
    "dhcp",
    "ebtables",
    "ethtool",
    "iptables",
    "kernel",
    "kubernetes-kubeadm",
    "kubernetes-kubelet",
    "openssh-server",
    "passwd",
    "rsyslog",
    "socat",
    "tar",
    "vi",
    "selinux-policy",
 ]
upgrade_img = "kubeos-upgrade:v1"
version = "v1"

[[copy_files]]
 dst = "/boot"
 src = "${initramfs_name}"

[[copy_files]]
 dst = "/boot/efi/EFI/openEuler"
 src = "grub.cfg"

[systemd_service]
 name = [ "serial-getty@ttyS0"]

[disk_partition]
 img_size = 30
 root = 4000
EOF
    
    log_info "kbimg.toml生成完成: $toml"
}

step1_create_base_env() {
    log_info "=========================================="
    log_info "步骤1: 创建镜像生成基础环境"
    log_info "=========================================="
    
    log_info "1.1 创建工作目录..."
    mkdir -p "$WORK_DIR"
    
    log_info "1.2 配置yum源..."
    cp "$REPO_FILE" "$WORK_DIR/"
    
    log_info "1.3 创建root根目录..."
    mkdir -p "$NEW_ROOT"
    
    log_info "1.4 安装基础环境..."
    dnf install -y --installroot="$NEW_ROOT" dnf vim yum dnf kernel dracut rpm-build golang libblkid-devel dosfstools dracut-network gdisk grub2 grub2-tools grub2-tools-extra grub2-common KubeOS KubeOS-scripts uname-build-checks rng-tools systemd-cryptsetup multipath-tools lvm2 --setopt=reposdir="$WORK_DIR"
    
    log_info "1.5 挂载系统目录..."
    mount -t proc proc "$NEW_ROOT/proc"
    MOUNT_POINTS+=("$NEW_ROOT/proc")
    
    mount -t sysfs sysfs "$NEW_ROOT/sys"
    MOUNT_POINTS+=("$NEW_ROOT/sys")
    
    mount -t devtmpfs devtmpfs "$NEW_ROOT/dev"
    MOUNT_POINTS+=("$NEW_ROOT/dev")
    
    mount -t tmpfs tmpfs "$NEW_ROOT/run"
    MOUNT_POINTS+=("$NEW_ROOT/run")
    
    log_info "步骤1完成"
}

step2_generate_initramfs() {
    log_info "=========================================="
    log_info "步骤2: 生成initramfs"
    log_info "=========================================="
    
    log_info "2.1 准备ignition RPM包..."
    local ignition_basename=$(basename "$IGNITION_RPM")
    cp "$IGNITION_RPM" "$NEW_ROOT/home/"
    
    log_info "2.2 创建dracut模块目录..."
    mkdir -p "$NEW_ROOT/usr/lib/dracut/modules.d/30persist"
    
    log_info "2.3 复制persist模块文件..."
    if [[ -f "$KUBEOS_PATH/bootup/module-setup.sh" ]]; then
        cp "$KUBEOS_PATH/bootup/module-setup.sh" "$NEW_ROOT/usr/lib/dracut/modules.d/30persist/"
    else
        log_warn "文件不存在: $KUBEOS_PATH/bootup/module-setup.sh"
    fi
    
    if [[ -f "$KUBEOS_PATH/bootup/persist-mount.service" ]]; then
        cp "$KUBEOS_PATH/bootup/persist-mount.service" "$NEW_ROOT/usr/lib/dracut/modules.d/30persist/"
    else
        log_warn "文件不存在: $KUBEOS_PATH/bootup/persist-mount.service"
    fi
    
    log_info "2.4 编译ignition并生成initramfs..."
    IN_CHROOT=true
    set +e
    
    chroot "$NEW_ROOT" /bin/bash -c "
set -e
echo '[INFO] 卸载已有的ignition包...'
rpm -e ignition 2>/dev/null || true
echo '[INFO] 编译ignition...'
cd /home
rpmbuild --rebuild $ignition_basename
echo '[INFO] 安装编译好的ignition...'
rpm -ivh /root/rpmbuild/RPMS/\$(uname -m)/ignition-*.rpm

echo '[INFO] 删除旧initramfs...'
rm -rf /boot/initramfs-\$(uname -r).img

echo '[INFO] 生成新initramfs...'
dracut --add 'ignition lvm network' --add-drivers 'iso9660 llc bridge failover crc64 jbd2 ext4 overlay mbcache virtio sd_mod sg realtek e1000 virtio-net net_failover mii dm-region-hash dm-mirror dm-log t10-pi virtio-mmio' /boot/initramfs-\$(uname -r).img \$(uname -r)

echo '[INFO] initramfs生成完成'
"
    
    local chroot_ret=$?
    set -e
    IN_CHROOT=false
    
    if [[ $chroot_ret -ne 0 ]]; then
        log_error "chroot环境编译ignition或生成initramfs失败"
        exit 1
    fi
    
    log_info "步骤2完成"
}

step3_prepare_kubeos() {
    log_info "=========================================="
    log_info "步骤3: KubeOS准备"
    log_info "=========================================="
    
    log_info "3.1 复制initramfs到scripts目录..."
    local kernel_version=$(chroot "$NEW_ROOT" uname -r)
    local initramfs_name="initramfs-${kernel_version}.img"
    
    if [[ -f "$NEW_ROOT/boot/$initramfs_name" ]]; then
        cp "$NEW_ROOT/boot/$initramfs_name" "$SCRIPTS_DIR/"
        log_info "initramfs已复制到: $SCRIPTS_DIR/$initramfs_name"
    else
        log_error "initramfs文件不存在: $NEW_ROOT/boot/$initramfs_name"
        exit 1
    fi
    
    log_info "3.2 复制grub.cfg..."
    local grub_real_path=$(readlink -f "$GRUB_CFG")
    if [[ "$grub_real_path" != "$SCRIPTS_DIR/grub.cfg" ]]; then
        cp -f "$GRUB_CFG" "$SCRIPTS_DIR/"
    fi
    
    log_info "3.3 生成kbimg.toml..."
    generate_kbimg_toml "$initramfs_name"
    
    log_info "步骤3完成"
}

step4_create_kubeos_image() {
    log_info "=========================================="
    log_info "步骤4: 制作KubeOS镜像"
    log_info "=========================================="
    
    log_info "4.1 创建KubeOS镜像..."
    cd "$SCRIPTS_DIR"
    
    if [[ -f "./kbimg" ]]; then
        ./kbimg create -f kbimg.toml vm-img
    elif command -v kbimg &>/dev/null; then
        kbimg create -f kbimg.toml vm-img
    else
        log_error "kbimg工具不存在"
        exit 1
    fi
    
    log_info "步骤4完成"
    log_info "=========================================="
    log_info "KubeOS镜像制作完成!"
    log_info "=========================================="
}

unmount_all() {
    log_info "卸载所有挂载点..."
    
    for mount_point in "${MOUNT_POINTS[@]}"; do
        if mountpoint -q "$mount_point" 2>/dev/null; then
            log_info "卸载: $mount_point"
            umount -l "$mount_point" 2>/dev/null || true
        fi
    done
    
    MOUNT_POINTS=()
}

main() {
    log_info "$SCRIPT_NAME v$SCRIPT_VERSION"
    log_info "KubeOS路径: $KUBEOS_PATH"
    log_info "脚本目录: $SCRIPTS_DIR"
    log_info "工作目录: $WORK_DIR"
    log_info "Debug模式: $DEBUG_MODE"
    
    acquire_lock
    check_prerequisites
    
    step1_create_base_env
    step2_generate_initramfs
    step3_prepare_kubeos
    step4_create_kubeos_image
    
    unmount_all
    
    log_info "所有步骤完成!"
}

parse_args "$@"
main
