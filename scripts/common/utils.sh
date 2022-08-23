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

CHECK_REGEX='\||;|&|&&|\|\||>|>>|<|,|#|!|\$'

function mount_proc_dev_sys() {
        local tmp_root=$1
        mount -t proc none "${tmp_root}/proc"
        mount --bind /dev "${tmp_root}/dev"
        mount --bind /dev/pts "${tmp_root}/dev/pts"
        mount -t sysfs none "${tmp_root}/sys"
}

function unmount_dir() {
        local dir=$1

        if [ -L "${dir}" ] || [ -f "${dir}" ]; then
                log_error_print "${dir} is not a directory, please check it."
                return 1
        fi

        if [ ! -d "${dir}" ]; then
                return 0
        fi

        local real_dir=$(readlink -e "${dir}")
        local mnts=$(awk '{print $2}' < /proc/mounts | grep "^${real_dir}" | sort -r)
        for m in ${mnts}; do
                log_info_print "Unmount ${m}"
                umount -f "${m}" || true
        done

        return 0
}

function init_part() {
        local offset=$(fdisk -l system.img | grep $1 | awk '{print $2}')
        local sizelimit=$(fdisk -l system.img | grep $1 | awk '{print $3}')
        sizelimit=$(echo "($sizelimit - $offset)*512" | bc)
        offset=$(echo "${offset}*512" | bc)
        local loop=$(losetup -f)
        losetup -o "${offset}" --sizelimit "${sizelimit}" "${loop}" system.img
        if [ $2 == "BOOT" ];then
                mkfs.vfat -n "$2" "${loop}"
                mount -t vfat "${loop}" "$3"
        else
                mkfs.ext4 -L "$2" "${loop}"
                mount -t ext4 "${loop}" "$3"
                rm -rf "$3/lost+found"
        fi
}

function delete_dir() {
        local ret=0
        local dir="$1"
        unmount_dir "${dir}"
        ret=$?
        if [ "${ret}" -eq 0 ]; then
                rm -rf "${dir}"
                return 0
        else
                log_error_print "${dir} is failed to unmount , can not delete $dir."
                return 1
        fi
}

function delete_file() {
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

function check_binary_exist() {
        if [ ! -f "$1" ];then
                log_error_print "binary path is invalid."
                exit 3
        fi
}

function check_repo_path() {
        if [ ! -f "$1" ];then
                log_error_print "REPO path is invalid."
                exit 3
        fi

        if [ -d "${RPM_ROOT}" ]; then
                log_error_print "there is a rootfs folder. please confirm if rootfs is being used, if not, please remove ${RPM_ROOT} first."
                exit 5
        fi
}

function check_disk_space() {
        local disk_ava="$(df ${PWD} | awk 'NR==2{print}' | awk '{print $4}')"
        case $1 in
        docker)
          local maxsize=$((6*1024*1024))
          if [ "${disk_ava}" -lt "${maxsize}" ]; then
             log_error_print "The available disk space is not enough, at least 6GiB."
             exit 6
          fi
          ;;
        vm)
          local maxsize=$((5*1024*1024))
          if [ "${disk_ava}" -lt "${maxsize}" ]; then
             log_error_print "The available disk space is not enough, at least 25GiB."
             exit 6
          fi
          ;;
        pxe)
          local maxsize=$((5*1024*1024))
          if [ "${disk_ava}" -lt "${maxsize}" ]; then
             log_error_print "The available disk space is not enough, at least 5GiB."
             exit 6
          fi
          ;;
        esac
}

function check_param() {
        set +eE
        local arg=$1
        echo "${arg}" | grep -v -E -q ${CHECK_REGEX}
        filterParam=$(echo "${arg}" | grep -v -E ${CHECK_REGEX})
        if [[ "${filterParam}" != "${arg}" ]]; then
           log_error_print "params ${arg} is invalid, please check it."
           exit 3
        fi
        set -eE
}

function check_docker_exist() {
        if [[ "$(docker images -q $1 2> /dev/null)" == "" ]]; then
          log_error_print "docker is not exist please pull $1 first "
          exit 9
        fi
}
