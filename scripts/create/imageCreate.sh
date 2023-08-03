#!/bin/bash
## Copyright (c) Huawei Technologies Co., Ltd. 2022. All rights reserved.
# KubeOS is licensed under the Mulan PSL v2.
# You can use this software according to the terms and conditions of the Mulan PSL v2.
# You may obtain a copy of Mulan PSL v2 at:
#     http://license.coscl.org.cn/MulanPSL2
# THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
# PURPOSE.
## See the Mulan PSL v2 for more details.

TMP_MOUNT_PATH="${PWD}/mnt"
RPM_ROOT="${PWD}/rootfs"
IMG_SIZE=20
PWD="$(pwd)"
function create_img() {
  local BOOT_MODE=$1
  rm -f system.img update.img
  qemu-img create system.img ${IMG_SIZE}G
  if [ "$BOOT_MODE" = "legacy" ]; then
    local BOOT_PATH=${TMP_MOUNT_PATH}/boot/grub2
    parted system.img -s mklabel msdos
    parted system.img -s mkpart primary ext4 1MiB 60MiB
  else
    local BOOT_PATH=${TMP_MOUNT_PATH}/boot/efi
    parted system.img -s mklabel gpt
    parted system.img -s mkpart primary fat32 1MiB 60MiB
  fi
  parted system.img -s mkpart primary ext4 60MiB 2160MiB
  parted system.img -s mkpart primary ext4 2160MiB 4260MiB
  parted system.img -s mkpart primary ext4 4260MiB 100%
  local device=$(losetup -f)
  losetup "${device}" system.img

  mkdir -p "${TMP_MOUNT_PATH}"

  init_part system.img2 ROOT-A "${TMP_MOUNT_PATH}"
  
  mkdir -p ${BOOT_PATH}
  chmod 755 ${BOOT_PATH}
  if [ "$BOOT_MODE" = "legacy" ]; then
    init_part system.img1 GRUB2 "${BOOT_PATH}"
  else
    init_part system.img1 BOOT "${BOOT_PATH}"
  fi
  tar -x -C ${TMP_MOUNT_PATH} -f os.tar
  if [ "$BOOT_MODE" = "legacy" ]; then
    sed -i "s/insmod part_gpt/insmod part_msdos/g; \
s/set root='hd0,gpt2'/set root='hd0,msdos2'/g; \
s/set root='hd0,gpt3'/set root='hd0,msdos3'/g" \
"${TMP_MOUNT_PATH}"/boot/grub2/grub.cfg
  fi
  sync
  cp bootloader.sh "${TMP_MOUNT_PATH}"
  mount_proc_dev_sys "${TMP_MOUNT_PATH}"
  DEVICE="${device}" BOOT_MODE="${BOOT_MODE}" chroot "${TMP_MOUNT_PATH}" bash bootloader.sh
  rm -rf "${TMP_MOUNT_PATH}/bootloader.sh"
  sync

  dd if=/dev/disk/by-label/ROOT-A of=update.img bs=8M
  sync
  unmount_dir "${TMP_MOUNT_PATH}"
  init_part system.img3 ROOT-B "${TMP_MOUNT_PATH}"
  umount "${TMP_MOUNT_PATH}"

  init_part system.img4 PERSIST "${TMP_MOUNT_PATH}"
  mkdir ${TMP_MOUNT_PATH}/{var,etc,etcwork}
  mkdir -p ${TMP_MOUNT_PATH}/etc/KubeOS/certs
  umount "${TMP_MOUNT_PATH}"

  losetup -D
  parted system.img -- set 1 boot on
  qemu-img convert system.img -O qcow2 system.qcow2
}

function create_pxe_img() {
  rm -rf initramfs.img kubeos.tar
  local opt=$1
  shift
  case $opt in
  "repo")
    create_os_tar_from_repo "$@"
    ;;
  "docker")
    create_os_tar_from_docker "$@"
    ;;
  esac
  tar -xvf os.tar  ./initramfs.img
  mv os.tar kubeos.tar
}

function create_docker_image() {
  local DOCKER_IMG="$6"
  create_os_tar_from_repo "$@"
  docker build -t ${DOCKER_IMG} -f ./Dockerfile .
}

function create_vm_img() {
  local opt=$1
  shift
  local BOOT_MODE=$5
    case $opt in
    "repo")
      create_os_tar_from_repo "$@"
      create_img "${BOOT_MODE}"
      ;;
    "docker")
      create_os_tar_from_docker "$@"
      create_img "${BOOT_MODE}"
      ;;
    esac

}

function create_admin_img() {
  local DOCKERFILE="$1"
  local DOCKER_IMG="$2"
  local ADMIN_CONTAINER_DIR="$3"
  cp ../bin/hostshell ${ADMIN_CONTAINER_DIR}
  docker build -t ${DOCKER_IMG} -f ${DOCKERFILE} ${ADMIN_CONTAINER_DIR}
  rm -rf ${ADMIN_CONTAINER_DIR}/hostshell
}