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

function init_boot_part() {
	local offset=$(fdisk -l system.img | grep $1 | awk '{print $2}')
	local sizelimit=$(fdisk -l system.img | grep $1 | awk '{print $3}')
        echo "offset: $offset"
	echo "sizelimit: $sizelimit"
	sizelimit=$(echo "($sizelimit - $offset)*512" | bc)
	offset=$(echo "${offset}*512" | bc)
        echo "sizelimit: $sizelimit"
        echo "offset: $offset"

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

function init_flexible_part() {
	local offset=$(fdisk -l system.img | grep $1 | awk '{print $2}')
	local sizelimit=$(fdisk -l system.img | grep $1 | awk '{print $3}')
        echo "offset: $offset"
	echo "sizelimit: $sizelimit"
	sizelimit=$(echo "($sizelimit - $offset)*512" | bc)
	offset=$(echo "${offset}*512" | bc)
        echo "offset: $offset"
	echo "sizelimit: $sizelimit"
	local loop=$(losetup -f)
	losetup -o "${offset}" --sizelimit "${sizelimit}" "${loop}" system.img
	if [ $4 == "vfat" ];then
			mkfs.vfat -n "$2" "${loop}"
			mount -t vfat "${loop}" "$3"
	elif [ $4 == "ext4" ];then
			mkfs.ext4 -L "$2" "${loop}"
			mount -t ext4 "${loop}" "$3"
			rm -rf "$3/lost+found"
	else
			log_error_print "the type of $4 is wrong, please check it."
			exit 3
	fi
}

function check_file_valid() {
        local file="$1"
        local mesg="$2"
        if [ ! -e "${file}" ]; then
                log_error_print "${mesg} is not exist."
		exit 3
        fi
        if [ ! -f "${file}" ];then
                log_error_print "${mesg} is not a file."
                exit 3
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

function check_conf_valid() {
        local conf_path="${PWD}/00bootup/Global.cfg"
        check_file_valid ${conf_path} "Globab.cfg"
        if [ $# != 7 ];then
                log_error_print "configure configured in Global.cfg is empty."
                exit 3
        fi
        for addr in ${server_ip} ${local_ip} ${route_ip} ${netmask}; do
                check_ip_valid $addr
        done
}

function check_ip_valid() {
        local ipaddr="$1";
        if [[ ! $ipaddr =~ ^[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$ ]] ; then
                log_error_print "ip address configured in Global.cfg is not valid."
                exit 3;
        fi
        for quad in $(echo "${ipaddr//./ }"); do
                if [ $quad -ge 0 ] && [ $quad -le 255 ];then
                        continue
                fi
                log_error_print "ip address configured in Global.cfg is not valid."
                exit 3;
        done

}

function check_docker_file() {
        check_file_valid $1 "admin-container Dockerfile"
}