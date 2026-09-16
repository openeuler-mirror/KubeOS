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
# 在 ignition files 阶段完成后对 /etc overlay 的 upperdir（/persist/etc）补标。
# 背景：ignition 的 SELinux relabel 是"白名单列表式"——relabelPath 只登记每个写入路径的
# 第一个缺失组件（filesystemEntries.go），passwd/unit 等仅登记文件本身（passwd.go/units.go），
# 深层中间目录（如 /etc/sysconfig/network-scripts、/etc/systemd/system/basic.target.wants）不
# 在列表内，导致目录漏标；策略加载前（switch-root 后数秒窗口）经 overlay 触发 copy-up 的
# 文件（如 /etc/machine-id）也会丢标签。本脚本在 ignition 完成、overlay 挂载后统一补标。
#
# 关键：必须用 -r /persist 做 rootpath 前缀剥离后打标，而不是裸 /persist 路径。
# file_contexts 的规则全部按最终路径（/etc/...、/var/...、/home/...）编写，用
# /persist/etc/... 前缀去匹配一条都命不中，只会落到兜底规则打成 default_t——不仅
# 修不好标签，还会把原本正确的标签刷坏。-r /persist 把 /persist/etc/hostname 映射
# 回 /etc/hostname 再匹配，规则正常命中，setxattr 落到实际文件上；且一条命令
# 覆盖 /persist 下全部子目录（etc/var/home/opt/audit 等 overlay upperdir）。
# setfiles 无需 selinuxfs/策略加载，纯 setxattr 写 xattr（ignition 打标同款工具）。

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

# /persist 未挂载说明 persist-mount.service 未成功，此时不应打标
if ! grep -q ' /sysroot/persist ' /proc/mounts; then
    log "/sysroot/persist not mounted (persist-mount.service failed?), skip"
    exit 0
fi

# 不带 -c：-c 会先对 file_contexts 与二进制策略做一致性校验，镜像内二者不匹配时
# setfiles 直接失败退出（且该校验对打标本身没有必要）；与 ignition 的 initramfs
# 打标方式一致，纯 setxattr，无需 selinuxfs/策略加载。
# 优先用 initramfs 自带的 setfiles（dracut 打包，依赖库齐全）；rootfs 里的 setfiles
# 依赖其动态库环境，目标镜像库异常时 chroot exec 会以 127 失败。两分支均用 -r
# 做前缀剥离，规则按剥离后的最终路径匹配。
if [ -x /usr/sbin/setfiles ]; then
    log "running setfiles on /persist with -r prefix strip (initramfs)"
    LANG=C /usr/sbin/setfiles -r /sysroot/persist \
        /sysroot/etc/selinux/$SELINUXTYPE/contexts/files/file_contexts /sysroot/persist
elif [ -x /sysroot/usr/sbin/setfiles ]; then
    log "running setfiles on /persist with -r prefix strip (chroot)"
 	LANG=C chroot /sysroot /usr/sbin/setfiles -r /persist \
 	    /etc/selinux/$SELINUXTYPE/contexts/files/file_contexts /persist
else
 	# 兜底：镜像自带 restorecon（仅适用于 selinuxfs 已就绪的环境）
 	log "setfiles not found in initramfs nor /sysroot, fallback to restorecon"
 	LANG=C chroot /sysroot /usr/sbin/restorecon -R -r /persist /persist
fi
