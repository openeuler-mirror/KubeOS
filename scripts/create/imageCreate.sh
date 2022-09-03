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
  rm -f system.img update.img
  qemu-img create system.img ${IMG_SIZE}G
  parted system.img -s mklabel gpt
  parted system.img -s mkpart primary fat32 1MiB 60MiB
  parted system.img -s mkpart primary ext4 60MiB 2160MiB
  parted system.img -s mkpart primary ext4 2160MiB 4260MiB
  parted system.img -s mkpart primary ext4 4260MiB 100%
  parted system.img -s set 1 boot on
  local device=$(losetup -f)
  losetup "${device}" system.img

  mkdir -p "${TMP_MOUNT_PATH}"

  init_part system.img2 ROOT-A "${TMP_MOUNT_PATH}"
  local BOOT_PATH=${TMP_MOUNT_PATH}/boot/efi
  mkdir -p ${BOOT_PATH}
  chmod 755 ${BOOT_PATH}
  init_part system.img1 BOOT "${BOOT_PATH}"
  tar -x -C ${TMP_MOUNT_PATH} -f os.tar
  sync
  cp bootloader.sh "${TMP_MOUNT_PATH}"
  mount_proc_dev_sys "${TMP_MOUNT_PATH}"
  DEVICE="${device}" chroot "${TMP_MOUNT_PATH}" bash bootloader.sh
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
  local DOCKER_IMG="$5"
  create_os_tar_from_repo "$@"
  docker build -t ${DOCKER_IMG} -f ./Dockerfile .
}

function create_vm_img() {
  local opt=$1
  shift
    case $opt in
    "repo")
      create_os_tar_from_repo "$@"
      create_img
      ;;
    "docker")
      create_os_tar_from_docker "$@"
      create_img
      ;;
    esac

}
