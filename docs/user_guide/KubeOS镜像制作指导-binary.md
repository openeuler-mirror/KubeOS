# KubeOS镜像制作说明

## 简介

kbimg是使用Rust语言编写的二进制工具，通过解析用户的[toml配置文件](#详细toml配置文件示例)，动态生成脚本，制作KubeOS虚拟机镜像、PXE物理机镜像、升级镜像、OCI容器镜像、ISO安装镜像和admin容器镜像。同时支持通过OCI镜像直接在物理机上安装KubeOS。

## 命令介绍

kbimg - CLI tool for generating various types of image for KubeOS

```text
Usage: kbimg [OPTIONS] <COMMAND> 

Commands:
  create   Create a new KubeOS image
  install  Install KubeOS to a target disk
  help     Print this message or the help of the given subcommand(s)

Options:
  -d, --debug    Enable debug mode, generate the scripts without execution
  -h, --help     Print help
  -V, --version  Print version
```

kbimg-create - Create a new KubeOS image

```text
Usage: kbimg create --file <FILE> <IMAGE_TYPE>

Arguments:
  <IMAGE_TYPE>  [possible values: vm-img, pxe-img, upgrade-img, admin-container, oci-img, iso-img]

Options:
  -f, --file <FILE>  Path to the toml configuration file
  -h, --help         Print help
```

kbimg-install - Install KubeOS to a target disk

```text
Usage: kbimg install --file <FILE> <INSTALL_TYPE>

Arguments:
  <INSTALL_TYPE>  [possible values: disk]

Options:
  -f, --file <FILE>  Path to the toml configuration file
  -h, --help         Print help
```

## 注意事项

* 请确保已安装`qemu-img bc parted tar yum docker dosfstools`
* 使用ISO镜像制作功能请确保已安装`elemental mtools xorriso rsync`
* 制作启用dm-verity功能的镜像，需要安装`pesign nss openssl veritysetup crypto-policies`
* KubeOS镜像制作需要使用root权限
* 制作镜像时提供的 repo 文件中，yum 源建议同时配置 openEuler 具体版本的 everything 仓库和 EPOL 仓库
* KubeOS镜像制作之前需要先将当前机器上的selinux关闭或者设为允许模式
* 使用默认rpmlist进行KubeOS镜像制作至少需要有25G的剩余空间
* KubeOS镜像制作工具执行异常中断，可能会残留文件、目录或挂载，需用户手动清理，对于可能残留的rootfs目录，该目录虽然权限为555，但容器OS镜像制作在开发环境进行，不会对生产环境产生影响
* 请确保os-agent属主和属组为root，建议os-agent文件权限为500

## 配置文件说明

### from_repo

从 repo 创建升级镜像、OCI容器镜像、虚拟机镜像或PXE物理机镜像

  | 参数 | 描述 |
  | --- | --- |
  | agent_path | os-agent 二进制的路径 |
  | legacy_bios | 目前仅支持设置为`false`，即UEFI引导 |
  | repo_path | repo 文件的路径，repo 文件中配置制作镜像所需要的 yum 源 |
  | root_passwd | root 用户密码，与/etc/shadow文件内密码格式一致，可使用`openssl passwd -6 -salt $(head -c18 /dev/urandom \| openssl base64)`命令生成 |
  | version | KubeOS 镜像的版本，将写入/etc/os-release文件内作为OS标识 |
  | rpmlist | 期望安装进镜像内的rpm包列表 |
  | upgrade_img | [可选项]指定生成的升级容器镜像的镜像名(制作升级容器镜像必需) |

### admin_container

制作admin运维容器

  | 参数 | 描述 |
  | --- | --- |
  | hostshell | hostshell二进制路径，可在项目根目录下通过`make hostshell`编译 |
  | img_name | 指定生成的容器镜像名 |

### oci_img

制作OCI容器镜像

  | 参数 | 描述 |
  | --- | --- |
  | image_name | 指定生成的 OCI 容器镜像名，例如 "kubeos-oci:v1" |

### security_stig

[可选项] 制作安全加固（STIG）镜像配置。**此配置在 OCI 镜像构建阶段生效**，开启后构建时执行安全加固脚本并打 SELinux 标签，生成的 OCI 镜像包含已加固的系统。

  | 参数 | 描述 |
  | --- | --- |
  | security_enable | 是否启用安全加固，默认 false |
  | security_scripts_path | security-tools.sh 脚本完整路径，例如 "/opt/kubeOS/security-tools/security-tools.sh" |
  | chrony_server | [可选项] chrony 时间同步服务器地址 |
  | rsyslog_server | [可选项] rsyslog 日志服务器地址 |
  | audit_server | [可选项] audit 远程审计服务器地址 |

### iso_img

制作ISO安装镜像，基于已有的 OCI 镜像生成可用于物理机安装的 ISO 文件

  | 参数 | 描述 |
  | --- | --- |
  | oci_image | 已有的 OCI 容器镜像名，作为 ISO 制作的基础镜像，例如 "kubeos-oci:v1" |
  | iso_image | 指定生成的 ISO 容器镜像名，例如 "kubeos-iso:v1" |
  | elemental_cli_path | elemental 二进制文件路径 |
  | output_dir | [可选项] ISO 文件输出目录，默认当前 scripts-auto 目录 |
  | grub_entry_name | [可选项] GRUB 引导菜单项名称，默认 "KubeOS" |
  | name | [可选项] ISO 文件名前缀（不含 .iso），默认 "KubeOS" |

### install

物理机ISO启动后安装KubeOS的配置。`kbimg install -f kbimg.toml disk` 使用 skopeo 从容器镜像仓库拉取 OCI 镜像并安装到目标磁盘。

  | 参数 | 描述 |
  | --- | --- |
  | target_disk | 目标磁盘设备，例如 "/dev/sda"、"/dev/nvme0n1" |
  | oci_image | skopeo 拉取用的 OCI 镜像地址，例如 "docker://192.168.1.1:5000/kubeos-oci:v1" |
  | configs | [可选项] 配置文件注入列表，每个条目包含 src（本地文件或URL）和 dst（rootfs 内目标路径） |
  | skip_tls | [可选项] 从 URL 下载配置文件时跳过 TLS 证书校验，默认 false |
  | reboot | [可选项] 安装完成后是否自动重启，默认 false |

#### [[install.configs]]

配置注入列表，用于在安装时将 cloud-init、ignition 等配置文件写入目标系统。每条包含：

  | 参数 | 描述 |
  | --- | --- |
  | src | 配置文件来源，可为本地文件路径或 http/https URL |
  | dst | 配置文件在目标 rootfs 中的目标路径，例如 "/etc/cloud/cloud.cfg.d/99_kubeos.cfg" |

### pxe_config

在制作PXE物理机镜像时，配置该参数用于PXE安装。制作PXE物理机镜像时必需。

  | 参数 | 描述 |
  | --- | --- |
  | server_ip | 用于下载根文件系统 tar 包的 HTTP 服务器地址 |
  | rootfs_name | 放置于 HTTP 服务器的文件系统 tar 包名称 |
  | disk | 安装 KubeOS 系统的目标磁盘名 |
  | route_ip | 配置目标机器网卡的路由 IP |
  | dhcp | [可选项] 是否启用 DHCP 模式配置网络，默认为 false |
  | local_ip | [可选项] 配置目标机器网卡的 IP，dhcp 为 false 时必需 |
  | net_name | [可选项] 配置目标机器网卡名，dhcp 为 false 时必需 |
  | netmask | [可选项] 配置目标机器网卡的子网掩码，dhcp 为 false 时必需 |

**注意**：`pxe_config`下的配置参数无法进行校验，需要用户自行确认其正确性。

### users

[可选项] 添加用户

  | 参数 | 描述 |
  | --- | --- |
  | name | 用户名 |
  | passwd | 密码 |
  | primary_groups | [可选项] 用户主组(默认为用户同名组) |
  | groups | [可选项] 用户附加组 |

**注意**：添加用户会默认创建用户同名组，配置用户附加组时，若组不存在会报错失败。若有特殊配置需求，用户可通过[chroot_script](#chroot_script)脚本自行实现。

### copy_files

[可选项] 拷贝文件到rootfs内指定目录

  | 参数 | 描述 |
  | --- | --- |
  | dst | 目标路径 |
  | src | 源文件路径 |
  | create_dir | [可选项]拷贝前创建文件夹 |

**注意**：拷贝文件无法保留权限，如果需要特殊权限，可借助[chroot_script](#chroot_script)脚本自行实现。

### grub

[可选项] grub配置，配置dm-verity时必需

  | 参数 | 描述 |
  | --- | --- |
  | passwd | grub 明文密码 |

### systemd_service

[可选项] 配置 systemd 服务开机自启

  | 参数 | 描述 |
  | --- | --- |
  | name | systemd 服务名 |

### chroot_script

[可选项] 自定义 chroot 脚本

  | 参数 | 描述 |
  | --- | --- |
  | path | 脚本路径 |
  | rm | [可选项]执行完毕后是否删除该脚本，配置`true`删除，`false`或空保留 |

### disk_partition

[可选项] 自定义分区大小和镜像大小

  | 参数 | 描述 |
  | --- | --- |
  | root | root分区大小, 单位为MiB，默认2560MiB |
  | img_size | [可选项]镜像大小，单位为GB，默认20GB |

### persist_mkdir

[可选项] persist 分区新建目录

  | 参数 | 描述 |
  | --- | --- |
  | name | 目录名 |

### dm_verity

[可选项] 制作启用dm-verity功能的虚拟机或升级镜像

  | 参数 | 描述 |
  | --- | --- |
  | efi_key | efi明文口令 |
  | grub_key | grub明文口令 |
  | keys_dir |[可选项]可指定密钥文件夹，复用先前制作镜像创建的密钥  |

## 使用说明

### 注意事项

* kbimg 执行需要 root 权限。
* 当前仅支持 x86和 AArch64 架构使用。
* 不支持并发执行。如果使用脚本`&`连续执行可能会出现异常情况。制作过程中碰到异常掉电或中断后无法清理环境时，可参考[异常退出清理方法](#异常退出清理方法)清理后重新制作。
* 制作镜像时提供的 repo 文件中，yum 源建议同时配置 openEuler 具体版本的 everything 仓库和 EPOL 仓库。
* dm-verity使用说明：
	*	仅支持虚拟机场景，暂不支持物理机环境。
	*	不支持通过 HTTP/HTTPS 服务器下载升级镜像进行系统升级。仅支持从容器镜像仓库下载升级镜像进行升级。
  *	启动虚拟机时，必须配置使用 virtio 类型设备。
  * 启用dm-verity功能的升级容器镜像不可用于升级未开启dm-verity的容器OS。同理，未启动dm-verity功能的升级容器镜像不可用于升级开启dm-verity功能的容器OS。在集群内，部分节点开启dm-verity功能，部分未开启，需要用户控制下发对应的升级镜像。
  *	制作升级容器镜像和虚拟机镜像时，推荐使用相同的密钥(配置`keys_dir`为先前制作镜像时创建的密钥文件路径。配置`efi_key`或`grub_key`一致不能保证密钥文件是一模一样的)。若密钥不一致，在切换备用分区时可能导致证书校验失败，从而无法启动系统。出现证书校验失败问题时，需要重新导入备用分区证书进行修复。

### KubeOS OCI 镜像制作

#### 注意事项

* 制作出的 OCI 镜像可用于后续的 ISO 镜像制作、物理机安装和KubeOS单节点通过命令行升级
* 使用示例 rpmlist 进行容器OS镜像制作时所需磁盘空间至少为10G，若使用自定义 rpmlist 可能会超过10G，rpmlist需至少包含示例中给出的rpm包。
* KubeOS OCI镜像当前不支持dm-verity场景。

#### 使用示例

* 配置文件示例（包含oci_img）

  ```toml
  [from_repo]
  agent_path = "./bin/rust/release/os-agent"
  legacy_bios = false
  repo_path = "/etc/yum.repos.d/openEuler.repo"
  root_passwd = "$1$xyz$RdLyKTL32WEvK3lg8CXID0"
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
      "dracut-network",
      "dracut-live",
      "coreutils",
      "dosfstools",
      "dracut",
      "gawk",
      "hwinfo",
      "net-tools",
      "parted",
      "skopeo",
      "curl",
      "e2fsprogs",
      "util-linux",
      "shim",
      "mokutil",
      # 如构建满足stig规范的镜像则需要安装以下包
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

  ]
  upgrade_img = "<registry>/kubeos-upgrade:v1"
  version = "v1"

  [oci_img]
  image_name = "kubeos-oci:v1"

  # 可选：如需满足stig规范则进行此项配置
  # [security_stig]
  # security_enable = true
  # security_scripts_path = "/opt/kubeOS/security-tools/security-tools.sh"
  # chrony_server = "0.us.pool.ntp.mil"
  # rsyslog_server = "192.168.1.100"
  # audit_server = "192.168.1.101"
  ```

* 执行命令

  ```bash
  kbimg create -f kbimg.toml oci-img
  ```

* 结果说明
  * 制作完成后，通过`docker images`查看制作出来的 OCI 容器镜像
  * OCI 镜像可推送到容器镜像仓库，供后续 ISO 制作和裸金属安装使用

  ```bash
  docker push <registry>/kubeos-oci:v1
  ```

### KubeOS ISO 镜像制作

#### 注意事项

* ISO 镜像基于已有的 OCI 镜像制作，需先完成 [OCI 镜像制作](#kubeos-oci-镜像制作)并推送至镜像仓库或本地 docker 缓存
* 制作 ISO 镜像需要安装 elemental 工具
* ISO 镜像用于物理机 UEFI 启动安装，不支持BIOS模式
* ISO 制作完成后的文件默认输出到 kbimg 执行目录下的 `scripts-auto` 目录，可通过 `output_dir` 指定

#### 使用示例

* 配置文件示例

  ```toml
  [from_repo]
  agent_path = "./bin/rust/release/os-agent"
  legacy_bios = false
  repo_path = "/etc/yum.repos.d/openEuler.repo"
  root_passwd = "$1$xyz$RdLyKTL32WEvK3lg8CXID0"
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
  ]
  upgrade_img = "kubeos-upgrade:v1"
  version = "v1"

  [oci_img]
  image_name = "<registry>/kubeos-oci:v1"

  [iso_img]
  oci_image = "<registry>/kubeos-oci:v1"
  iso_image = "kubeos-iso:v1"
  elemental_cli_path = "./bin/elemental"
  output_dir = "./output"
  # grub_entry_name = "KubeOS"
  # name = "KubeOS"
  ```

* 执行命令

  ```bash
  # 首先制作 OCI 镜像
  kbimg create oci-img -f kbimg.toml
  # 然后制作 ISO 镜像
  kbimg create iso-img -f kbimg.toml
  ```

* 结果说明
  * 制作完成后，在 `output_dir` 目录下生成 `.iso` 文件
  * 将 ISO 文件写入 U盘 或挂载到 BMC 虚拟光驱，物理机从 UEFI 引导即可进入安装流程

### KubeOS 裸金属安装

#### 注意事项

* 裸金属安装需要先将 OCI 镜像推送到容器镜像仓库，目标机器能够通过网络访问该仓库
* 仅支持 UEFI 引导（x86_64 和 aarch64），不支持 legacy BIOS
* 安装过程会格式化目标磁盘的全部数据，请确认磁盘上没有需要保留的数据
* 不支持多个磁盘同时安装 KubeOS，可能导致启动失败或挂载紊乱
* 安装过程中 skopeo 拉取 OCI 镜像可能耗时较长，取决于网络状况

#### 准备工作

1. 制作并推送 OCI 镜像到容器镜像仓库

   ```bash
   kbimg create oci-img -f kbimg.toml
   docker push <registry>/kubeos-oci:v1
   ```

2. 准备安装配置文件 kbimg.toml（在目标机器上），参考以下示例：

   ```toml
   [from_repo]
   agent_path = "/opt/kubeOS/bin/os-agent"
   legacy_bios = false
   repo_path = "/etc/yum.repos.d/openEuler.repo"
   root_passwd = "$1$xyz$RdLyKTL32WEvK3lg8CXID0"
   rpmlist = ["kernel", "passwd"]
   version = "v1"

   [install]
   target_disk = "/dev/sda"
   oci_image = "docker://<registry>/kubeos-oci:v1"
   # skip_tls = true
   # reboot = true
   
   # 以下为可选配置
   # 可以注入cloud-init 和 ignition 配置到指定目录
   # [[install.configs]]
   # src = "https://example.com/user-data"
   # dst = "/etc/cloud/cloud.cfg.d/99_kubeos.cfg"
   
   # [[install.configs]]
   # src = "/root/user-data"
   # dst = "/var/lib/cloud/user-data"

   # 配置进行磁盘分区时根分区的大小，注意需要和oci-img的大小匹配，如根分区大小不足安装会失败
   # [disk_partition]
   # root = 5000   # 根分区大小，单位 MiB（默认 2560）
   
   # 配置在persist目录创建的目录
   # [persist_mkdir]
   # name = ["bar", "foo"]

   # 如果oci-image镜像中进行了stig规范的安装加固对应install时也需要执行此参数，否则安全加固不生效
   # [security_stig]
   # security_enable = true

   ```

   如果配置文件来源为 URL，可通过 `skip_tls = true` 跳过 TLS 证书校验，实际使用curl命令进行下载，如不跳过TLS证书校验，请提前进行证书配置。

3. 在目标机器上执行安装

   ```bash
   kbimg install disk -f kbimg.toml
   ```

   安装成功后，若配置了 `reboot = true`，机器会自动重启进入新安装的 KubeOS 系统。

#### 磁盘分区布局

安装后目标磁盘的分区布局如下：

| 分区 | 大小 | 文件系统 | 标签 | 说明 |
| --- | --- | --- | --- | --- |
| 1 | 60 MiB | vfat | BOOT | EFI 引导分区 |
| 2 | root MiB | ext4 | ROOT-A | A 分区（当前运行） |
| 3 | root MiB | ext4 | ROOT-B | B 分区（升级备用） |
| 4 | 剩余空间 | ext4 | PERSIST | 持久化数据分区 |

root 分区大小可通过 `disk_partition.root` 自定义，默认 2560 MiB。A/B 分区大小相同。

### KubeOS 虚拟机镜像制作

#### 注意事项

* 制作出来的容器 OS 虚拟机镜像目前只能用于 CPU 架构为 x86 和 AArch64 的虚拟机。
* 默认root密码为openEuler12#$
* 使用默认rpmlist进行容器OS镜像制作时所需磁盘空间至少为25G，若使用自定义rpmlist可能会超过25G。
* 支持CPU 架构为 x86 和 aarch64 的虚拟机场景。若x86架构的虚拟机需要使用 legacy 启动模式，请在`[from_repo]`下配置`legacy_bios`为`true`
* `repo_path`为制作镜像所需要的 yum 源文件路径，yum 源建议配置为 openEuler 具体版本的 everything 仓库和 EPOL 仓库。
* 容器OS运行底噪<150M (不包含k8s组件及相关依赖`kubernetes-kubeadm，kubernetes-kubelet， containernetworking-plugins，socat，conntrack-tools，ebtables，ethtool`)

#### 使用示例

* 配置文件示例

```toml
[from_repo]
agent_path = "./bin/rust/release/os-agent"
legacy_bios = false
repo_path = "/etc/yum.repos.d/openEuler.repo"
root_passwd = "$1$xyz$RdLyKTL32WEvK3lg8CXID0" # default passwd: openEuler12#$
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
]
version = "v1"
```

* 结果说明
容器 OS 镜像制作完成后，会在 ./scripts-auto 目录下生成
  * system.qcow2: 用于启动虚拟机的qcow2 格式的系统镜像，大小默认为 20GiB，支持的根文件系统分区大小 < 2560 MiB，持久化分区 < 15GB 。
  * system.img: 用于启动虚拟机的img 格式的系统镜像，大小默认为 20GiB，支持的根文件系统分区大小 < 2560 MiB，持久化分区 < 15GB 。
  * kubeos.tar: 用于升级的根文件系统tar包。
  * update-boot.img/update-root.img/update-hash.img: 仅在dm-verity模式下生成，可忽略。

### KubeOS 物理机安装所需镜像及文件制作

#### 注意事项

* 制作出来的容器 OS 物理安装所需的镜像目前只能用于 CPU 架构为 x86 和 AArch64 的物理机安装。
* 容器OS 目前不支持 x86 架构的物理机使用 legacy 启动模式启动。
* 首先需要修改```kbimg.toml```中```pxe_config```的配置，对相关参数进行配置，详细参数可见[参数说明](#pxe_config)，ip目前仅支持ipv4，配置示例如下
* 不支持多个磁盘都安装KubeOS，可能会造成启动失败或挂载紊乱。
* 使用默认的 rpmlist 进行镜像制作时，所需磁盘空间至少为 5GB。如果使用自定义的 rpmlist，可能需要超过 5GB 的磁盘空间。
* PXE物理机镜像制作不支持dm-verity功能
* 在 PXE 安装阶段，需要从 HTTP 服务器的根目录下载根分区 tar 包（tar包名称为toml配置文件中配置的名称）。请确保机器拥有足够的内存空间以存储根分区 tar 包及临时中间文件。

#### 使用示例

* 首先需要修改```kbimg.toml```中```pxe_config```的配置，对相关参数进行配置，详细参数可见[参数说明](#pxe_config)，ip目前仅支持ipv4，配置示例如下

  ```toml
  [pxe_config]
  dhcp = false
  # rootfs file name
  rootfs_name = "kubeos.tar"
  # select the target disk to install kubeOS
  disk = "/dev/vda"
  # pxe server ip address where stores the rootfs on the http server
  server_ip = "192.168.122.50"
  # target machine ip
  local_ip = "192.168.122.100"
  # target machine route
  route_ip = "192.168.122.1"
  # target machine netmask
  netmask = "255.255.255.0"
  # target machine netDevice name
  net_name = "eth0"
  ```

* 如需进行DNS配置，请先自定义```resolv.conf```文件，并启用```copy_files```字段将配置文件拷贝到```/etc```目录

  ```toml
  [[copy_files]]
  dst = "/etc"
  src = "<path to resolv.conf>"
  ```

* KubeOS物理机安装所需镜像制作，及pxe_config配置全示例

  ```toml
  [from_repo]
  agent_path = "./bin/rust/release/os-agent"
  legacy_bios = false
  repo_path = "/etc/yum.repos.d/openEuler.repo"
  root_passwd = "$1$xyz$RdLyKTL32WEvK3lg8CXID0" # default passwd: openEuler12#$
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
      "coreutils",
      "dosfstools",
      "dracut",
      "gawk",
      "hwinfo",
      "net-tools",
      "parted",
  ]
  version = "v1"

  [pxe_config]
  dhcp = true
  rootfs_name = "kubeos.tar"
  disk = "/dev/vda"
  server_ip = "192.168.122.50"
  route_ip = "192.168.122.1"
  #local_ip = "192.168.1.100"
  #netmask = "255.255.255.0"
  #net_name = "eth0"
  ```

* 结果说明
  * initramfs.img: 用于pxe启动用的 initramfs 镜像
  * kubeos.tar: pxe安装所用的根分区文件系统

### admin运维容器镜像制作

* 首先在KubeOS项目根目录下，执行`make hostshell`命令编译hostshell二进制
* 在toml配置文件内，填入以下示例配置制作admin运维容器镜像

```toml
[admin_container]
img_name = "kubeos-admin-container:v1"
hostshell = "./bin/hostshell"
```

* 制作完成后，通过`docker images`查看制作出来的KubeOS容器镜像

## 使用说明

### 使用cloud-init在KubeOS启动时初始化

在`[from_repo]`配置内的`rpmlist`中，配置`cloud-init`包，可在KubeOS启动时使用`cloud-init`进行初始化。
若用户需要覆盖默认的cloud-init配置，可配置如下示例

  ```toml
  [[copy_files]]
  dst = "/etc/cloud"
  src = "./cloud.cfg"
  ```

### 创建systemd服务

* 新增 systemd 服务需要将对应的 .service 文件或 .mount 文件拷贝至镜像```/etc/systemd/system```目录下

  ```toml
  [[copy_files]]
  dst = "/etc/systemd/system"
  src = "./containerd.service"

  [systemd_service]
  name = ["containerd"]
  ```

## 附录

### 异常退出清理方法

1. 若在使用`kbimg`制作镜像过程中，异常退出，无法清理环境，可使用如下方法进行清理：

```bash
function unmount_dir() {
  local dir=$1
  if [ -L "${dir}" ] || [ -f "${dir}" ]; then
    echo "${dir} is not a directory, please check it."
    return 1
  fi
  if [ ! -d "${dir}" ]; then
    return 0
  fi
  local real_dir=$(readlink -e "${dir}")
  local mnts=$(awk '{print $2}' < /proc/mounts | grep "^${real_dir}" | sort -r)
  for m in ${mnts}; do
    echo "Unmount ${m}"
    umount -f "${m}" || true
  done
  return 0
}
ls -l ./scripts-auto/test.lock && rm -rf ./scripts-auto/test.lock
unmount_dir ./scripts-auto/rootfs/proc
unmount_dir ./scripts-auto/rootfs/sys
unmount_dir ./scripts-auto/rootfs/dev/pts
unmount_dir ./scripts-auto/rootfs/dev
unmount_dir ./scripts-auto/mnt/boot/grub2
unmount_dir ./scripts-auto/mnt
rm -rf ./scripts-auto/rootfs ./scripts-auto/mnt
```

2. 如果执行以上命令仍然无法删除目录，可尝试先调用如下命令，再重新执行第一步的命令。

```bash
fuser -kvm ./scripts-auto/rootfs
fuser -kvm ./scripts-auto/mnt
```

### 详细toml配置文件示例

请根据需求和[配置文件说明](#配置文件说明)，修改如下示例配置文件，生成所需镜像。

```toml
[from_repo]
agent_path = "./bin/rust/release/os-agent"
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
    # Below packages are required for pxe-image. Uncomment them if you want to generate pxe-image.
    # "coreutils",
    # "dosfstools",
    # "dracut",
    # "gawk",
    # "hwinfo",
    # "net-tools",
    # "parted",
]
upgrade_img = "kubeos-upgrade:v1"
version = "v1"

# [admin_container]
# img_name = "kubeos-admin-container:v1"
# hostshell = "./bin/hostshell"

# [oci_img]
# image_name = "kubeos-oci:v1"

# [iso_img]
# oci_image = "kubeos-oci:v1"
# iso_image = "kubeos-iso:v1"
# elemental_cli_path = "./bin/elemental-cli"
# output_dir = "./output"
# grub_entry_name = "KubeOS"
# name = "KubeOS"

# [pxe_config]
# dhcp = false
# disk = "/dev/vda"
# local_ip = "192.168.1.100"
# net_name = "eth0"
# netmask = "255.255.255.0"
# rootfs_name = "kubeos.tar"
# route_ip = "192.168.1.1"
# server_ip = "192.168.1.50"

# [[users]]
# groups = ["admin", "wheel"]
# name = "foo"
# passwd = "foo"
# primary_group = "foo"

# [[users]]
# groups = ["example"]
# name = "bar"
# passwd = "bar"

# [[copy_files]]
# create_dir = "/root/test"
# dst = "/root/test/foo.txt"
# src = "/root/KubeOS/foo.txt"

# [[copy_files]]
# dst = "/etc/bar.txt"
# src = "../bar.txt"

# [grub]
# passwd = "foo"

# [systemd_service]
# name = ["containerd", "kubelet"]

# [chroot_script]
# path = "./my_chroot.sh"
# rm = true

# [disk_partition]
# img_size = 30 # GB
# root = 3000  # MiB

# [persist_mkdir]
# name = ["bar", "foo"]

# [dm_verity]
# efi_key = "foo"
# grub_key = "bar"
# keys_dir = "./keys"

# [install]
# target_disk = "/dev/sda"
# oci_image = "docker://192.168.1.100:5000/kubeos-oci:v1"
# skip_tls = false
# reboot = false

# [[install.configs]]
# src = "https://example.com/user-data"
# dst = "/etc/cloud/cloud.cfg.d/99_kubeos.cfg"

# [[install.configs]]
# src = "/path/to/config.ign"
# dst = "/boot/efi/ignition/config.ign"

# [security_stig]
# security_enable = true
# security_scripts_path = "/opt/kubeOS/security-tools/security-tools.sh"
# chrony_server = "0.us.pool.ntp.mil"
# rsyslog_server = "192.168.1.100"
# audit_server = "192.168.1.101"
```
