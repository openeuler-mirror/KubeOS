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

set -e
NAME=KubeOS
REPO=""
VERSION=""
AGENT_PATH=""
PASSWD=""
DOCKER_IMG=""
DOCKERFILE=""
LOCK=./test.lock
ADMIN_CONTAINER_DIR=./admin-container
BOOT_MODE=efi

source common/globalVariables.sh &>/dev/null
source common/log.sh &>/dev/null
source common/utils.sh &>/dev/null
source create/rootfsCreate.sh &>/dev/null
source create/imageCreate.sh &>/dev/null
source create/partCreate.sh &>/dev/null
source create/cpFiles.sh &>/dev/null
source 00bootup/Global.cfg &>/dev/null

function file_lock() {
  local lock_file=$1
  exec {lock_fd}>"${lock_file}"
  flock -xn "${lock_fd}"
}

function test_lock() {
  file_lock "${LOCK}"
  if [ $? -ne 0 ]; then
    log_error_print "There is already an generate process running."
    exit 203
  fi
}

function clean_space() {
  delete_dir "${RPM_ROOT}"
  delete_dir "${TMP_MOUNT_PATH}"
  delete_file os.tar
  rm -rf "${LOCK}"
  delete_file ${ADMIN_CONTAINER_DIR}/hostshell
}

function clean_img() {
  delete_file system.img
  delete_file update.img
  delete_file initramfs.img
  delete_file kubeos.tar
}

function verify_admin_input() {
  set +eE
  for i in "f" "d"
  do
    echo "$@" | grep -q "\-$i "
    if [ "$?" -ne 0 ];then
          log_error_print "option -$i is mandatory, please check input"
          show_admin_image_usage
          exit 3
    fi
  done
  set -eE
  while getopts "f:d:" opt
      do
        case $opt in
          f)
            check_param $OPTARG
            DOCKERFILE="$OPTARG"
            ;;
          d)
            check_param $OPTARG
            DOCKER_IMG="$OPTARG"
            ;;
          *)
            log_error_print "option $opt not found"
            show_admin_image_usage
            exit 3
           ;;
        esac
      done
}

function verify_upgrade_image_input() {
  while getopts "p:v:e:b:d:l" opt
    do
      case $opt in
        p)
          REPO="$OPTARG"
          ;;
        v)
          VERSION="$OPTARG"
          ;;
        b)
          AGENT_PATH="$OPTARG"
          ;;
        e)
          # encrypted password contains special characters, not verify.
          PASSWD="$OPTARG"
          ;;
        d)
          DOCKER_IMG="$OPTARG"
          ;;
        l)
          BOOT_MODE=legacy
          ;;
      esac
    done
}

function verify_repo_input() {
    while getopts "p:v:e:b:l" opt
      do
        case $opt in
          p)
            REPO="$OPTARG"
            ;;
          v)
            VERSION="$OPTARG"
            ;;
          b)
            AGENT_PATH="$OPTARG"
            ;;
          e)
            # encrypted password contains special characters.,not verify.
            PASSWD="$OPTARG"
            ;;
          l)
            BOOT_MODE=legacy
            ;;
        esac
       done
}

function verify_docker_input() {
  DOCKER_IMG=$2
}

function verify_create_input() {
  local ret=
  local cmd=$1
  case $1 in
  "upgrade-image")
    shift
    verify_upgrade_image_input "$@"
    create_docker_image "${REPO}" "${VERSION}" "${AGENT_PATH}" "${PASSWD}" "${BOOT_MODE}" "${DOCKER_IMG}"
    ;;
  "vm-image")
    shift
    if [[  $# -eq 8 || $# -eq 9  ]]; then
      verify_repo_input "$@"
      create_vm_img "repo" "${REPO}" "${VERSION}" "${AGENT_PATH}" "${PASSWD}" "${BOOT_MODE}"
    elif [ $# -eq 2 ]; then
      verify_docker_input "$@"
      create_vm_img "docker" "${DOCKER_IMG}"
    fi
    ;;
  "pxe-image")
    shift
    check_conf_valid ${rootfs_name} ${disk} ${server_ip} ${local_ip} ${route_ip} ${netmask} ${net_name}
    if [ $# -eq 8 ]; then
      verify_repo_input "$@"
      create_pxe_img "repo" "${REPO}" "${VERSION}" "${AGENT_PATH}" "${PASSWD}"
    elif [ $# -eq 2 ]; then
      verify_docker_input "$@"
      create_pxe_img "docker" "${DOCKER_IMG}"
    fi
    ;;
  "admin-image")
    shift
    if [ $# -eq 1 ]; then
      if [ "$1" == "-h" ] || [ "$1" == "--help" ]; then
        show_admin_image_usage
        exit 0
      fi
    fi
    if [ $# -ne 4 ]; then
      log_error_print "the number of parameters is incorrect, please check it."
      show_admin_image_usage
      exit 3
    fi
    verify_admin_input "$@"
    check_docker_file "${DOCKERFILE}"
    create_admin_img  "${DOCKERFILE}" "${DOCKER_IMG}" "${ADMIN_CONTAINER_DIR}"
    ;;
  esac
}

function kubeos_image_main() {
  local ret=
  local cmd=$1
  if [ "$#" -eq 1 ]; then
    case $1 in
    *)
      log_error_print "params is invalid,please check it."
      exit 3;;
    esac
  fi
  case $cmd in
  create)
    shift
    verify_create_input "$@"
    ;;
  *)
    log_error_print "command $1 not found"
    exit 3
    ;;
  esac
}

test_lock
trap clean_space EXIT
trap clean_img ERR

kubeos_image_main "$@"
