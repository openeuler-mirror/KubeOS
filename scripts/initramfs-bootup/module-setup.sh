#! /bin/bash
## Copyright (c) Huawei Technologies Co., Ltd. 2026. All rights reserved.
 # KubeOS is licensed under the Mulan PSL v2.
 # You can use this software according to the terms and conditions of the Mulan PSL v2.
 # You may obtain a copy of Mulan PSL v2 at:
 #     http://license.coscl.org.cn/MulanPSL2
 # THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 # IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 # PURPOSE.
## See the Mulan PSL v2 for more details.

depends() {
    return 0
}

install() {
    # relabel 脚本通过 chroot 在 /sysroot 内调用 setfiles，需保证 initramfs 内有 chroot
    inst_multiple chroot
    # setfiles 优先用 initramfs 自带的（dracut 打包时依赖库齐全），不依赖 rootfs
 	# 内的动态库环境，避免目标镜像库缺失时 chroot exec 失败（exit 127）
 	inst_multiple setfiles

    inst_simple "$moddir/persist-mount.service" \
        "$systemdsystemunitdir/persist-mount.service"
    systemctl -q --root="$initdir" enable persist-mount.service

    inst_simple "$moddir/kubeos-selinux-relabel.service" \
        "$systemdsystemunitdir/kubeos-selinux-relabel.service"
    inst_simple "$moddir/kubeos-selinux-relabel.sh" \
        "/usr/libexec/kubeos/kubeos-selinux-relabel.sh"
    systemctl -q --root="$initdir" enable kubeos-selinux-relabel.service
}

installkernel() {
    hostonly='' instmods ext4 overlay =fs/nls 
}