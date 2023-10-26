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

function prepare_yum() {
        # init rpmdb
        local REPO=$1
        rpm --root "${RPM_ROOT}" --initdb
        mkdir -p "${RPM_ROOT}"{/etc/yum.repos.d,/persist,/proc,/dev/pts,/sys}
        mount_proc_dev_sys "${RPM_ROOT}"
        # init yum repo
        local iso_repo="${RPM_ROOT}/etc/yum.repos.d/iso.repo"
        cat "${REPO}" > ${RPM_ROOT}/etc/yum.repos.d/iso.repo
}

function install_packages() {
        local REPO=$1
        local BOOT_MODE=$2
	prepare_yum ${REPO}

	echo "install package.."

	local filesize=$(stat -c "%s" ./rpmlist)
	local maxsize=$((1024*1024))
	if [ "${filesize}" -gt "${maxsize}" ]; then
		echo "please check if rpmlist is too big or something wrong"
		exit 7
	fi

	local rpms=$(cat ./rpmlist | tr "\n" " ")
        if [ "${ARCH}" == "x86_64" ]; then
                if [ "${BOOT_MODE}" = "legacy" ]; then
                        rpms+=" grub2"
                else
                        rpms+=" grub2-efi grub2-tools grub2-efi-x64-modules grub2-pc-modules"
                fi
                yum -y --installroot="${RPM_ROOT}" install --nogpgcheck --setopt install_weak_deps=False ${rpms} 
        elif [ "${ARCH}" == "aarch64" ]; then
                yum -y --installroot="${RPM_ROOT}" install --nogpgcheck --setopt install_weak_deps=False ${rpms} grub2-efi grub2-tools grub2-efi-aa64-modules
        fi
        yum -y --installroot="${RPM_ROOT}" clean all
}

function install_misc() {
        local VERSION=$1
        local AGENT_PATH=$2
        local PASSWD=$3
        local BOOT_MODE=$4
        local DNS_CONF="${PWD}/resolv.conf"

        cp ../files/*mount ../files/os-agent.service "${RPM_ROOT}/usr/lib/systemd/system/"
        cp ../files/os-release "${RPM_ROOT}/usr/lib/"
        cp "${AGENT_PATH}" "${RPM_ROOT}/usr/bin"
        rm "${RPM_ROOT}/etc/os-release"
        cat <<EOF > "${RPM_ROOT}/usr/lib/os-release"
NAME=${NAME}
ID=${NAME}
EOF
        echo "PRETTY_NAME=\"${NAME} ${VERSION}\"" >> "${RPM_ROOT}/usr/lib/os-release"
        echo "VERSION_ID=${VERSION}" >> "${RPM_ROOT}/usr/lib/os-release"
        mv "${RPM_ROOT}"/boot/vmlinuz* "${RPM_ROOT}/boot/vmlinuz"
        mv "${RPM_ROOT}"/boot/initramfs* "${RPM_ROOT}/boot/initramfs.img"
        if [ "$BOOT_MODE" = "legacy" ]; then
                cp grub.cfg "${RPM_ROOT}"/boot/grub2
                sed -i "s/insmod part_gpt/insmod part_msdos/g; \
s/set root='hd0,gpt2'/set root='hd0,msdos2'/g; \
s/set root='hd0,gpt3'/set root='hd0,msdos3'/g" \
"${RPM_ROOT}"/boot/grub2/grub.cfg
        else
                cp grub.cfg "${RPM_ROOT}"/boot/efi/EFI/openEuler
        fi
	cp -r ./00bootup ${RPM_ROOT}/usr/lib/dracut/modules.d/ 
        cp set_in_chroot.sh "${RPM_ROOT}"
        ROOT_PWD="${PASSWD}" BOOT_MODE="${BOOT_MODE}" chroot "${RPM_ROOT}" bash /set_in_chroot.sh
        rm "${RPM_ROOT}/set_in_chroot.sh"
        if [  -e "${DNS_CONF}" ]; then
                cp "${DNS_CONF}" "${RPM_ROOT}/etc/resolv.conf"
        fi
}

function apply_user_config() {
        install_files
        cp userConfig.sh "${RPM_ROOT}"
        chroot "${RPM_ROOT}" bash /userConfig.sh
        rm "${RPM_ROOT}/userConfig.sh"
}

function create_docker_image() {
  local DOCKER_IMG="$6"
  create_os_tar_from_repo "$@"
  docker build -t ${DOCKER_IMG} -f ./Dockerfile .
}


function create_os_tar_from_repo() {
        local REPO=$1
        local VERSION=$2
        local AGENT_PATH=$3
        local PASSWD=$4
        local BOOT_MODE=$5
        install_packages ${REPO} ${BOOT_MODE}
        install_misc ${VERSION} ${AGENT_PATH} ${PASSWD} ${BOOT_MODE}
        apply_user_config
        unmount_dir "${RPM_ROOT}"
        tar -C "$RPM_ROOT" -cf ./os.tar .
}

function create_os_tar_from_docker() {
        local DOCKER_IMG=$1
        container_id=$(docker create ${DOCKER_IMG})
        echo "$container_id"
        docker cp $container_id:/os.tar ./
        docker rm $container_id
}

