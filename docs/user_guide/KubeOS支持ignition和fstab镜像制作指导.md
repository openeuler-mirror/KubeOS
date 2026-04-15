# KubeOS支持ignition和fstab镜像制作指导
在 KubeOS 中支持使用 ignition 进行配置，支持使用```/etc/fstab```配置持久化磁盘挂载指导。
* ignition 是一个用于在系统首次启动时进行基础设施配置的工具，支持磁盘分区、文件系统创建、用户管理、网络配置和文件写入等操作，对于无法在KubeOS镜像制作时指定，或者需要动态配置的内容，可以使用 ignition 完成，具体配置能力和说明请见(ignition 使用指导)[]。
* ```/etc/fstab``` 是 Linux 系统中用于定义静态文件系统挂载信息的配置文件，系统启动时会根据该文件自动挂载指定的文件系统：
  * 对于无法在 KubeOS 镜像制作时指定的挂载，比如需要开机后才能配置的磁盘挂载，可以在```/etc/fstab```中配置，KubeOS 默认```/etc/fstab```中的磁盘挂载在重启后不生效，如需要重启后生效需要按照如下指导制作 KubeOS 镜像
  * ```/etc/fstab```中的磁盘挂载和 KubeOS 默认的及镜像制作时自定义的磁盘挂载不能重复（默认挂载请见[附录-KubeOS默认挂载目录](#kubeos默认挂载目录)）
* 支持ignition和fstab的KubeOS镜像当前只支持虚拟机场景镜像制作和使用


## 创建镜像生成基础环境
1. 配置yum源，以openEuler 24.03-LTS-SP3为例，具体repo文件请见[附录-yum源repo文件配置参考](#yum源repo文件配置参考)
```
mkdir -p /home/KubeOS
vim /home/KubeOS/24.03LTS-SP3.repo
```

2. 创建root根目录
```
mkdir -p /home/KubeOS/new_root 
```

3. 在```/home/KubeOS/new_root```建立新的 root 根用于创建 KubeOS，建议该路径有充足的空间

```
dnf install -y --installroot=/home/KubeOS/new_root dnf vim yum --setopt=reposdir=/home/KubeOS
```

4. 切根新root
```
mount -t proc proc /home/KubeOS/new_root/proc
mount -t sysfs sysfs /home/KubeOS/new_root/sys
mount -t devtmpfs devtmpfs /home/KubeOS/new_root/dev
mount -t tmpfs tmpfs /home/KubeOS/new_root/run
chroot /home/KubeOS/new_root
```

5. 安装必要软件
```
dnf install -y kernel dracut rpm-build golang  libblkid-devel dosfstools dracut-network gdisk grub2 grub2-tools grub2-tools-extra grub2-common KubeOS KubeOS-scripts uname-build-checks rng-tools systemd-cryptsetup multipath-tools lvm2 KubeOS KubeOS-scripts
```

6. 使用完成后退出环境（环境清理时进行）
```
exit
umount -l /home/KubeOS/new_root/proc
umount -l /home/KubeOS/new_root/sys
umount -l /home/KubeOS/new_root/dev
umount -l /home/KubeOS/new_root/run/
```

## 生成initramfs：

1. 退出切根环境，并将新的ignition软件包到/home路径下，并创建dracut路径
```
mv ignition-2.15.0-150500.1.4.src.rpm /home/KubeOS/new_root/home
mkdir /home/KubeOS/new_root/usr/lib/dracut/modules.d/30persist
```

2. 进入新的root目录编译新的ignition，并制作initramfs
```
chroot /home/KubeOS/new_root
cd /home
rpm -ivh ignition-2.15.0-150500.1.4.src.rpm
cd /root/rpmbuild
rpmbuild -ba SPECS/ignition.spec
cd /root/rpmbuild/RPMS/aarch64
rpm -ivh ignition-2.15.0-150500.1.4.aarch64.rpm
rm -rf /boot/initramfs-`/usr/bin/uname -r`.img

cp /opt/kubeOS/bootup/module-setup.sh /home/KubeOS/new_root/usr/lib/dracut/modules.d/30persist
cp /opt/kubeOS/bootup/persist-mount.service /home/KubeOS/new_root/usr/lib/dracut/modules.d/30persist
dracut --add "ignition lvm network" --add-drivers "iso9660 llc bridge failover crc64 jbd2 ext4 overlay mbcache virtio sd_mod sg realtek e1000 virtio-net net_failover mii dm-region-hash dm-mirror dm-log t10-pi virtio-mmio" /boot/initramfs-`/usr/bin/uname -r`.img `/usr/bin/uname -r`
```
3. 退出切根环境
```
exit
```

## KubeOS准备
1. 进入切根环境
```  
chroot /home/KubeOS/new_root
```

2. 修改KubeOS配置文件
* 需要在 grub.cfg 中增加ignition相关配置，需要用新的 grub.cfg 替换原 grub.cfg，所以创建grub.cfg配置文件，grub.cfg内容请见[附录-grub.cfg文件参考](#grubcfg文件参考)

* 修改 kbimg.toml，加入自定义内容，kbimg.toml需要修改下列字段：
* agent_path修改为'../bin/os-agent' #取决于当前kbimg.toml配置文件以及kbimg的位置，os-agent二进制在/opt/kubeOS/bin目录下
* rpmlist增加selinux-policy包（ignition需要），rpmlist为KubeOS制作rootfs的会安装包，如果需要任何调试工具在这里增加包名
* copy制作好的initramfs到KubeOS中
```
[[copy_files]]
dst = "/boot"
src = "initramfs-6.6.0-138.0.0.119.oe2403sp3.aarch64.img"(按dracut 命令生成的initramfs名修改即可)
```
* 开启串口方便调试（可以不加）
```
[systemd_service]
name = [ "serial-getty@ttyS0"]    # arm上
```
* 增加镜像和root分区大小，防止加包导致qcow2预估磁盘不足镜像制作失败
```
[disk_partition]
img_size = 30 # GB
root = 4000  # MiB
```
* 配置grub.cfg加入到KubeOS中，让grub支持ignition启动
```
[[copy_files]]
dst = "/boot/efi/EFI/openEuler"
src = "grub.cfg"
```
配置文件整理示例见[附录-kbimg.toml示例](#kbimgtoml-示例)：

## 制作KubeOS
1. 步骤二中生成的initramfs拷贝到/opt/kubeOS/scripts 
cp /boot/initramfs-`/usr/bin/uname -r`.img /opt/kubeOS/scripts 

2. 创建KubeOS镜像
./kbimg create -f kbimg.toml vm-img

## 附录   

### KubeOS默认挂载目录
以设备名为sda为例：
* boot-efi.mount: /dev/sda1 -> /boot/efi
* persist.mount: /dev/sda4 -> /persist
* etc.mount: /etc -> /persist/etc
* var.mount: /var -> /persist/var

### yum源repo文件配置参考
```
内容如下：
#generic-repos is licensed under the Mulan PSL v2.
#You can use this software according to the terms and conditions of the Mulan PSL v2.
#You may obtain a copy of Mulan PSL v2 at:
#    http://license.coscl.org.cn/MulanPSL2
#THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
#IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
#PURPOSE.
#See the Mulan PSL v2 for more details.

[OS]
name=OS
baseurl=http://repo.openeuler.org/openEuler-24.03-LTS-SP3/OS/$basearch/
sslverify=0
enabled=1
gpgcheck=0

[everything]
name=everything
baseurl=http://repo.openeuler.org/openEuler-24.03-LTS-SP3/everything/$basearch/
sslverify=0
enabled=1
gpgcheck=0

[EPOL]
name=EPOL
baseurl=http://repo.openeuler.org/openEuler-24.03-LTS-SP3/EPOL/main/$basearch/
sslverify=0
enabled=1
gpgcheck=0

[debuginfo]
name=debuginfo
baseurl=http://repo.openeuler.org/openEuler-24.03-LTS-SP3/debuginfo/$basearch/
sslverify=0
enabled=1
gpgcheck=0

[source]
name=source
baseurl=http://repo.openeuler.org/openEuler-24.03-LTS-SP3/source/
sslverify=0
enabled=1
gpgcheck=0

[update]
name=update
baseurl=http://repo.openeuler.org/openEuler-24.03-LTS-SP3/update/$basearch/
sslverify=0
enabled=1
gpgcheck=0

[update-source]
name=update-source
baseurl=http://repo.openeuler.org/openEuler-24.03-LTS-SP3/update/source/
sslverify=0
enabled=1
gpgcheck=0

```
### grub.cfg文件参考
```
# Copyright (c) Huawei Technologies Co., Ltd. 2024. All rights reserved.
# KubeOS is licensed under the Mulan PSL v2.
# You can use this software according to the terms and conditions of the Mulan PSL v2.
# You may obtain a copy of Mulan PSL v2 at:
#     http://license.coscl.org.cn/MulanPSL2
# THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
# PURPOSE.
# See the Mulan PSL v2 for more details.

set pager=1

if [ -f ${config_directory}/grubenv ]; then
  load_env -f ${config_directory}/grubenv
elif [ -s $prefix/grubenv ]; then
  load_env
fi
if [ "${next_entry}" ] ; then
   set default="${next_entry}"
   set next_entry=
   save_env next_entry
   set boot_once=true
else
   set default="${saved_entry}"
fi

if [ x"${feature_menuentry_id}" = xy ]; then
  menuentry_id_option="--id"
else
  menuentry_id_option=""
fi

export menuentry_id_option

if [ "${prev_saved_entry}" ]; then
  set saved_entry="${prev_saved_entry}"
  save_env saved_entry
  set prev_saved_entry=
  save_env prev_saved_entry
  set boot_once=true
fi

function savedefault {
  if [ -z "${boot_once}" ]; then
    saved_entry="${chosen}"
    save_env saved_entry
  fi
}

function load_video {
  if [ x$feature_all_video_module = xy ]; then
    insmod all_video
  else
    insmod efi_gop
    insmod efi_uga
    insmod ieee1275_fb
    insmod vbe
    insmod vga
    insmod video_bochs
    insmod video_cirrus
  fi
}

terminal_output console
if [ x$feature_timeout_style = xy ] ; then
  set timeout_style=menu
  set timeout=5
# Fallback normal timeout code in case the timeout_style feature is
# unavailable.
else
  set timeout=5
fi
### END /etc/grub.d/00_header ###

### BEGIN firstboot ###
set saved_root=$root
search --file --set=root /boot/writable/firstboot_happened
set flagpath="/boot/writable"
set ignition_firstboot=""
if ! [ -f "${flagpath}/firstboot_happened" ]; then
        set ignition_network_kcmdline=''
if [ -e "${flagpath}/ignition.firstboot" ]; then
        source "${flagpath}/ignition.firstboot"
fi
set ignition_firstboot="ignition.firstboot=1"
fi
set root=$saved_root
### END firstboot ###
### BEGIN /etc/grub.d/10_linux ###
menuentry 'A' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-A' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        search --no-floppy --label ROOT-A --set=root
        linux   /boot/vmlinuz root=/dev/vda2 ro rootfstype=ext4 quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3 ${ignition_firstboot} ignition.platform.id=metal
        initrd  /boot/initramfs-6.6.0-138.0.0.119.oe2403sp3.aarch64.img
}

menuentry 'B' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-B' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        search --no-floppy --label ROOT-B --set=root
        linux   /boot/vmlinuz root=/dev/vda3 ro rootfstype=ext4 quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3 ${ignition_firstboot} ignition.platform.id=metal
        initrd  /boot/initramfs-6.6.0-138.0.0.119.oe2403sp3.aarch64.img
}

### END /etc/grub.d/10_linux ###

### BEGIN /etc/grub.d/10_reset_boot_success ###
# Hiding the menu is ok if last boot was ok or if this is a first boot attempt to boot the entry
if [ "${boot_success}" = "1" -o "${boot_indeterminate}" = "1" ]; then
  set menu_hide_ok=1
else
  set menu_hide_ok=0
fi
# Reset boot_indeterminate after a successful boot
if [ "${boot_success}" = "1" ] ; then
  set boot_indeterminate=0
# Avoid boot_indeterminate causing the menu to be hidden more then once
elif [ "${boot_indeterminate}" = "1" ]; then
  set boot_indeterminate=2
fi
# Reset boot_success for current boot
set boot_success=0
save_env boot_success boot_indeterminate
### END /etc/grub.d/10_reset_boot_success ###

### BEGIN /etc/grub.d/12_menu_auto_hide ###
if [ x$feature_timeout_style = xy ] ; then
  if [ "${menu_show_once}" ]; then
    unset menu_show_once
    save_env menu_show_once
    set timeout_style=menu
    set timeout=60
  elif [ "${menu_auto_hide}" -a "${menu_hide_ok}" = "1" ]; then
    set orig_timeout_style=${timeout_style}
    set orig_timeout=${timeout}
    if [ "${fastboot}" = "1" ]; then
      # timeout_style=menu + timeout=0 avoids the countdown code keypress check
      set timeout_style=menu
      set timeout=0
    else
      set timeout_style=hidden
      set timeout=1
    fi
  fi
fi
### END /etc/grub.d/12_menu_auto_hide ###

### BEGIN /etc/grub.d/20_linux_xen ###
### END /etc/grub.d/20_linux_xen ###

### BEGIN /etc/grub.d/20_ppc_terminfo ###
### END /etc/grub.d/20_ppc_terminfo ###

### BEGIN /etc/grub.d/30_uefi-firmware ###
### END /etc/grub.d/30_uefi-firmware ###

### BEGIN /etc/grub.d/40_custom ###
# This file provides an easy way to add custom menu entries.  Simply type the
# menu entries you want to add after this comment.  Be careful not to change
# the 'exec tail' line above.
### END /etc/grub.d/40_custom ###

### BEGIN /etc/grub.d/41_custom ###
if [ -f  ${config_directory}/custom.cfg ]; then
  source ${config_directory}/custom.cfg
elif [ -z "${config_directory}" -a -f  $prefix/custom.cfg ]; then
  source $prefix/custom.cfg;
fi
### END /etc/grub.d/41_custom ###
```
### kbimg.toml 示例
```
[from_repo]
agent_path = "../bin/os-agent"
legacy_bios = false
repo_path = "/etc/yum.repos.d/openEuler.repo"
root_passwd = "$1$xyz$RdLyKTL32WEvK3lg8CXID0" # default passwd: openEuler12#$, use "openssl passwd -6 -salt $(head -c18 /dev/urandom | openssl base64)" to generate your passwd
rpmlist = [
    "NetworkManager",
    "cloud-init",
    "conntrack-tools",
    "containerd",
    "containernetworking-plugins",
    "cri-tools",
    "dhcp",
    "ebtables",
    "ethtool",
    "iptables",
    "kernel",
    "kubernetes-kubeadm",
    "kubernetes-kubelet",
    "openssh-server",
    "passwd",
    "rsyslog",
    "socat",
    "tar",
    "vi",
    "selinux-policy",
 ]
upgrade_img = "kubeos-upgrade:v1"
version = "v1"

[[copy_files]]
 dst = "/boot"
 src = "initramfs-6.6.0-138.0.0.119.oe2403sp3.aarch64.img"

[[copy_files]]
 dst = "/boot/efi/EFI/openEuler"
 src = "grub.cfg"

[systemd_service]
 name = [ "serial-getty@ttyS0"]

[disk_partition]
 img_size = 30 # GB
 root = 4000  # MiB
```
