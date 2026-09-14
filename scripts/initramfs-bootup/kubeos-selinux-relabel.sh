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
#
# 在 ignition files 阶段完成后对 persist 区域执行全树 restorecon 补标。
# 背景：ignition 的 SELinux relabel 是"白名单列表式"——relabelPath 只登记每个写入路径的
# 第一个缺失组件（filesystemEntries.go），passwd/unit 等仅登记文件本身（passwd.go/units.go），
# 深层中间目录（如 /etc/sysconfig/network-scripts、/etc/systemd/system/basic.target.wants）不
# 在列表内，导致目录漏标。本脚本在 ignition 完成、persist 挂载后对 /persist 全树补标。
# restorecon 通过 chroot 在 /sysroot 内执行，读取镜像自带的 file_contexts，仅 setxattr 打标，
# 无需加载策略（未加载策略时内核处于 permissive，打标不受 AVC 限制），故本服务不依赖任何策略加载服务。

set -e

# initramfs 阶段 journal 在切根后丢失，关键信息直接写入内核日志（显示在 console）
log() { echo "kubeos-selinux-relabel: $*" > /dev/kmsg; }
trap 'log "script exited with status $?"' EXIT

SELINUXTYPE=$(awk -F= '/^SELINUXTYPE=/{print $2}' /sysroot/etc/selinux/config 2>/dev/null || true)
[ -n "$SELINUXTYPE" ] || SELINUXTYPE=targeted
log "SELINUXTYPE=$SELINUXTYPE"

# SELinux 未真正启用（如 SELINUX=disabled 但存在 config）时无策略文件，静默跳过
FILE_CONTEXTS="/sysroot/etc/selinux/$SELINUXTYPE/contexts/files/file_contexts"
if [ ! -f "$FILE_CONTEXTS" ]; then
    log "file_contexts not found ($FILE_CONTEXTS), skip"
    exit 0
fi

if [ ! -d /sysroot/persist ]; then
    log "/sysroot/persist not present (persist-mount.service failed?), skip"
    exit 0
fi

# restorecon 在未加载 selinuxfs 的 initramfs 环境会静默跳过（exit 0 但不打标），
# 故使用 setfiles（ignition 打标同款工具）：纯 setxattr 写 xattr，无需 selinuxfs/策略加载。
if [ -x /sysroot/usr/sbin/setfiles ]; then
    log "running setfiles on /persist (chroot)"
    LANG=C chroot /sysroot /usr/sbin/setfiles \
        /etc/selinux/$SELINUXTYPE/contexts/files/file_contexts /persist
elif [ -x /usr/sbin/setfiles ]; then
    log "running setfiles on /persist (initramfs, -r /sysroot)"
    LANG=C /usr/sbin/setfiles -r /sysroot \
        /sysroot/etc/selinux/$SELINUXTYPE/contexts/files/file_contexts /sysroot/persist
else
    # 兜底：镜像自带 restorecon（仅适用于 selinuxfs 已就绪的环境）
    log "setfiles not found in /sysroot nor initramfs, fallback to restorecon"
    LANG=C chroot /sysroot /usr/sbin/restorecon -R /persist
fi
