# KubeOS镜像制作指导（stig版本）
## KubeOS镜像制作
1. 安装KubeOS，安装后/opt/kubeOS目录如下
```  
yum install KubeOS-scritps
ls /opt/kubeOS
drwxr-xr-x 2 root root 4096 Jun 16 03:04 bin
drwxr-xr-x 2 root root 4096 Jun 16 03:04 bootup
drwxr-xr-x 2 root root 4096 Jun 16 03:04 cve-tools
drwxr-xr-x 2 root root 4096 Jun 16 03:04 scripts
drwxr-xr-x 2 root root 4096 Jun 16 03:04 security-tools
```

2. 修改KubeOS kbimg.toml配置文件
```
cd /opt/kubeOS/scripts
vim kbimg.toml 
```

* 2.1 agent_path修改为```../bin/os-agent``` （取决于当前kbimg.toml配置文件以及kbimg的位置，os-agent二进制在/opt/kubeOS/bin目录下）

* 2. 2 增加rpm包用于stig规则和initramfs制作，新增rpmlist请见[附录-kbimg.toml示例](#kbimgtoml-示例)；

* 2.3 需要在 grub.cfg 中增加ignition相关配置，需要用新的 grub.cfg 替换原 grub.cfg，所以创建grub.cfg配置文件，grub.cfg内容请见[附录-grub.cfg文件参考](#grubcfg文件参考)

  ```
  [[copy_files]]
   dst = "/boot/efi/EFI/openEuler/"
   src = "./grub.cfg"
  ```

* 2.4 需要在KubeOS中增加使能fstab的文件```module-setup.sh```、```persist-mount.service```，文件位于```/opt/kubeOS/bootup```

  ```
  [[copy_files]]
  create_dir = "/usr/lib/dracut/modules.d/30persist"
  dst = "/usr/lib/dracut/modules.d/30persist"
  src = "../bootup/module-setup.sh"
  
  [[copy_files]]
  create_dir = "/usr/lib/dracut/modules.d/30persist"
  dst = "/usr/lib/dracut/modules.d/30persist"
  src = "../bootup/persist-mount.service"
  ```

* 2.5 需要在KubeOS中用于cve报告生成的前置脚本，脚本在制作镜像过程中获取rpm报cve信息放在```/etc```下面

  ```
  [[copy_files]]
  dst = "/persist"
  src = "../cve-tools/gen_fixed_cve.sh"
  ```

* 2.6 需要在KubeOS中增加stig规则配置脚本，脚本在制作镜像过程中完成stig相关配置：

  ```
  [[copy_files]]
  dst = "/persist"
  src = "../security-tools/security-tools.sh"
  ```

* 2.7 stig规则中存在需客户自行配置的内容，需要在KubeOS中增加以下脚本，需要在脚本中配置CHRONY_SERVER、RSYSLOG_SERVER和AUDIT_SERVER（现脚本中为示例内容，需根据实际服务器ip进行配置），脚本会制作镜像过程执行完成stig相关配置，stig配置相关说明请见[KubeOS安全配置基线](https://atomgit.com/openeuler/security-committee/blob/master/sub-projects/secure-configuration-benchmark/release/KubeOS%E5%AE%89%E5%85%A8%E9%85%8D%E7%BD%AE%E5%9F%BA%E7%BA%BF.md)中8.1、16.1、16.12

  ```
  #!/bin/bash
  
  # ===== 请在此处填写您的服务器IP =====
  CHRONY_SERVER="0.us.pool.ntp.mil"     # 时间源IP
  RSYSLOG_SERVER="192.168.1.100"  # 日志接收端IP
  AUDIT_SERVER="192.168.1.101"    # 审计远程服务器IP
  # ===================================
  
  echo "=== 修复 chrony ==="
  if grep -qE "^server\s+\S+\s+maxpoll\s+16" /etc/chrony.conf; then
      echo "chrony 已满足要求，跳过。"
  else
      cp /etc/chrony.conf /etc/chrony.conf.bak
      # 删除所有已有的 maxpoll 行（若有），再添加新的
      sed -i '/^server.*maxpoll/d' /etc/chrony.conf
      echo "server $CHRONY_SERVER maxpoll 16" >> /etc/chrony.conf
      echo "chrony 已修复"
  fi
  
  echo "=== 修复 rsyslog ==="
  udp_ok=false
  tcp_ok=false
  grep -qE "^\*\.\*\s+@$RSYSLOG_SERVER:514" /etc/rsyslog.conf && udp_ok=true
  grep -qE "^\*\.\*\s+@@$RSYSLOG_SERVER:514" /etc/rsyslog.conf && tcp_ok=true
  if $udp_ok && $tcp_ok; then
      echo "rsyslog 已满足要求，跳过。"
  else
      cp /etc/rsyslog.conf /etc/rsyslog.conf.bak
      if ! $udp_ok; then
          echo "*.* @$RSYSLOG_SERVER:514" >> /etc/rsyslog.conf
      fi
      if ! $tcp_ok; then
          echo "*.* @@$RSYSLOG_SERVER:514" >> /etc/rsyslog.conf
      fi
      echo "rsyslog 已修复"
  fi
  
  echo "=== 修复 audit ==="
  if grep -qE "^remote_server\s*=\s*$AUDIT_SERVER" /etc/audit/audisp-remote.conf; then
      echo "audit 已满足要求，跳过。"
  else
      cp /etc/audit/audisp-remote.conf /etc/audit/audisp-remote.conf.bak
      # 若存在 remote_server 行则替换，否则追加
      if grep -q "^remote_server" /etc/audit/audisp-remote.conf; then
          sed -i "s/^remote_server\s*=.*/remote_server = $AUDIT_SERVER/" /etc/audit/audisp-remote.conf
      else
          echo "remote_server = $AUDIT_SERVER" >> /etc/audit/audisp-remote.conf
      fi
      echo "audit 已修复"
  fi
  
  echo "全部完成。"
  ```

  需要copy脚本到KubeOS中

  ```
  [[copy_files]]
  dst = "/persist"
  src = "./modify-stig-dynamic.sh"
  ```

* 2.8 配置以上两个脚本在镜像制作过程中执行，以及制作initramfs：

  * 在当前目录创建```start_custom_chroot.sh```，脚本内容参考如下：

  ```
  #!/bin/bash
  
  # 定义脚本路径
  SCRIPT1="/persist/gen_fixed_cve.sh"
  SCRIPT2="/persist/security-tools.sh"
  SCRIPT3="/persist/modify-stig-dynamic.sh"
  
  # 检查并执行脚本1
  if [ -f "$SCRIPT1" ]; then
      echo "执行 $SCRIPT1"
      bash "$SCRIPT1"
  else
      echo "警告: $SCRIPT1 不存在，跳过"
  fi
  
  # 检查并执行脚本2
  if [ -f "$SCRIPT2" ]; then
      echo "执行 $SCRIPT2"
      bash "$SCRIPT2"
  else
      echo "警告: $SCRIPT2 不存在，跳过"
  fi
  
  # 检查并执行脚本3
  if [ -f "$SCRIPT3" ]; then
      echo "执行 $SCRIPT3"
      bash "$SCRIPT3"
  else
      echo "警告: $SCRIPT3 不存在，跳过"
  fi
  
  # 执行 dracut 命令，生成固定名称的 initramfs.img
  echo "正在生成 initramfs.img..."
  dracut --force --add 'ignition network' --add-drivers 'iso9660 llc bridge failover crc64 jbd2 ext4 overlay mbcache virtio sd_mod sg realtek e1000 virtio-net net_failover mii t10-pi virtio-mmio' /boot/initramfs.img
  
  # 检查 dracut 执行结果
  if [ $? -eq 0 ]; then
      echo "成功: initramfs 已生成于 /boot/initramfs.img"
  else
      echo "错误: dracut 执行失败"
      exit 1
  fi
  
  echo "完成"                                                                                                               
  ```

  * 复制脚本到KubeOS中并且，配置start脚本执行

  ```
  [[copy_files]]
  dst = "/persist"
  src = "../cve-tools/gen_fixed_cve.sh"
  
  [[copy_files]]
  dst = "/persist"
  src = "../security-tools/security-tools.sh"
  
  [chroot_script]
   path = "./start_custom_chroot.sh"
   rm = false
  ```

* 2.9 stig规则中要求```/home```、```/var/log/audit```单独挂载，以及对nfs挂载有参数要求，KubeOS中可以使用overlay进行单独挂载，示例配置home.mount、var-log-audit.mout、persist-nfs.mount如下：

  ```
  cat home.mount 
  [Unit]
  Description=home Dir
  DefaultDependencies=no
  Conflicts=umount.target
  Before=local-fs.target umount.target
  Wants=persist.mount
  After=persist.mount
  
  [Mount]
  What=overlay
  Where=/home
  Type=overlay
  Options=defaults,nosuid,upperdir=/persist/home,lowerdir=/home,workdir=/persist/homework
  
  [Install]
  WantedBy=local-fs.target
  
  cat var-log-audit.mount 
  [Unit]
  Description=var-log-audit Dir
  DefaultDependencies=no
  Conflicts=umount.target
  Before=local-fs.target umount.target
  Wants=persist.mount
  After=persist.mount
  
  [Mount]
  What=overlay
  Where=/var/log/audit
  Type=overlay
  Options=upperdir=/persist/audit,lowerdir=/var/log/audit,workdir=/persist/auditwork
  
  [Service]
  Type=oneshot
  RemainAfterExit=yes
  ExecStartPost=/bin/chmod 600 /persist/audit
  
  [Install]
  WantedBy=local-fs.target
  
  cat persist-nfs.mount 
  [Unit]
  Description=Mount NFS audit log from host
  After=network-online.target
  Wants=network-online.target
  DefaultDependencies=no
  
  [Mount]
  What=your-nfs-path #需要自行配置nfs挂载目录
  Where=/persist/nfs #示例路径不需要一定为该路径
  Type=nfs
  Options=defaults,_netdev,hard,timeo=600,retrans=2,nodev,nosuid,noexec
  
  [Install]
  WantedBy=multi-user.target
  ```

  * 复制文件到KubeOS中对应目录

    ```
    [[copy_files]]
    dst = "/usr/lib/systemd/system/"
    src = "./var-log-audit.mount"
    
    [[copy_files]]
    dst = "/usr/lib/systemd/system/"
    src = "./home.mount"
    
    [[copy_files]]
    dst = "/usr/lib/systemd/system/"
    src = "./persist-nfs.mount"
    ```

  * 配置开机启动挂载

    ```
    [systemd_service]
     name = ["serial-getty@ttyS0","home.mount","persist-nfs.mount","var-log-audit.mount","set-selinux-label"]
    ```

  * 在persist目录创建对应路径

    ```
    [persist_mkdir]
     name = ["home","homework", "nfs", "audit","auditwork"]
    ```

* 2.10 满足stig要求需要在系统启动时为目录增加selinux标签，需要增加systemd service配置文件示例如下，**增加的挂载目录需要都列在RequiresMountsFor参数中**，保障命令在mount挂载后执行

  ```
  [Unit]
  Description=Run command after overlay mounts are ready
  RequiresMountsFor=/home /etc /var /var/log/audit /persist /opt/cni /persist/nfs
  After=local-fs.target
  
  [Service]
  Type=oneshot
  RemainAfterExit=yes
  ExecStart=/usr/sbin/fixfiles restore /
  
  [Install]
  WantedBy=multi-user.target
  ```

  * 复制service文件到KubeOS中，并配置开机启动

  ```
  [[copy_files]]
  dst = "/usr/lib/systemd/system/"
  src = "./set-selinux-label.service"
  
  [systemd_service]
   name = ["serial-getty@ttyS0","home.mount","persist-nfs.mount","var-log-audit.mount","set-selinux-label"]
  ```

* 2.11开启串口方便调试（可以不加）
```
[systemd_service]
name = [ "serial-getty@ttyS0"]    # arm上
```
* 2.12 增加镜像和root分区大小，防止加包导致qcow2预估磁盘不足镜像制作失败
```
[disk_partition]
img_size = 30 # GB
root = 4000  # MiB
```
配置文件整理示例见[附录-kbimg.toml示例](#kbimgtoml-示例)

3. 制作KubeOS镜像

   ```
   ./kbimg create -f kbimg.toml vm-img
   ```

   * 进行执行stig安全配置需要一定时间，镜像制作时间会增加
   * stig规则配置以后禁止ssh root登录，如需要进行调试，修改```/opt/kubeOS/security-tools/security-tools.sh```脚本中293行```configure_line "/etc/ssh/sshd_config" "PermitRootLogin no" ```为```configure_line "/etc/ssh/sshd_config" "PermitRootLogin yes" ```，或者在系统启动后通过串口进入系统修改```/etc/ssh/sshd_config文件中参数PermitRootLogin为yes

## CVE报告生成

```
cd /opt/kubeOS/cve-tools
./kubeos_cve_compare.py -n ${new_image} -o ${old_image}

new_image指新镜像的路径
old_image指旧镜像的路径
```

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
        initrd  /boot/initramfs.img
}

menuentry 'B' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-B' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        search --no-floppy --label ROOT-B --set=root
        linux   /boot/vmlinuz root=/dev/vda3 ro rootfstype=ext4 quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3 ${ignition_firstboot} ignition.platform.id=metal
        initrd  /boot/initramfs.img
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
repo_path = "./openEuler-24.03-lts-sp3.repo"
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
    "vim",
    "openssh-clients",
    "nfs-utils",
    "aide",
    "audispd-plugins",
    "audit",
    "autofs",
    "chrony",
    "firewalld",
    "kbd",
    "openssh",
    "pam",
    "pam_pkcs11",
    "nss",
    "nss-util",
    "ccid",
    "pcsc-lite",
    "pcsc-tools",
    "opensc",
    "policycoreutils",
    "policycoreutils-python-utils",
    "rsyslog",
    "selinux-policy",
    "sssd",
    "sudo",
    "openscap",
    "scap-security-guide",
    # Below packages are required for initramfs. 
    "coreutils",
    "ignition",
    "dosfstools",
    "dracut",
    "dracut-network", 
    "gdisk",
    "uname-build-checks",
    "rng-tools",
    "systemd-cryptsetup",
]
upgrade_img = "kubeos-upgrade:v1"
version = "v1"


[[copy_files]]
 dst = "/boot/efi/EFI/openEuler/"
 src = "./grub.cfg"

[[copy_files]]
dst = "/usr/lib/systemd/system/"
src = "./var-log-audit.mount"

[[copy_files]]
dst = "/usr/lib/systemd/system/"
src = "./home.mount"

[[copy_files]]
dst = "/usr/lib/systemd/system/"
src = "./persist-nfs.mount"

[[copy_files]]
dst = "/usr/lib/systemd/system/"
src = "./set-selinux-label.service"

[[copy_files]]
create_dir = "/usr/lib/dracut/modules.d/30persist"
dst = "/usr/lib/dracut/modules.d/30persist"
src = "../bootup/module-setup.sh"

[[copy_files]]
create_dir = "/usr/lib/dracut/modules.d/30persist"
dst = "/usr/lib/dracut/modules.d/30persist"
src = "../bootup/persist-mount.service"

[[copy_files]]
dst = "/persist"
src = "../cve-tools/gen_fixed_cve.sh"

[[copy_files]]
dst = "/persist"
src = "../security-tools/security-tools.sh"

[systemd_service]
 name = ["serial-getty@ttyS0","home.mount","persist-nfs.mount","var-log-audit.mount","set-selinux-label"]

[chroot_script]
 path = "./start_custom_chroot.sh"
 rm = false

[disk_partition]
 img_size = 30 # GB
 root = 6000  # MiB

[persist_mkdir]
 name = ["home","homework", "nfs", "audit","auditwork"]


```
