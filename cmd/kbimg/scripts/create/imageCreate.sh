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