# KubeOS镜像制作说明

## 简介

kbimg是使用Rust语言编写的二进制工具，通过解析用户的[toml配置文件](#详细toml配置文件示例)，动态生成脚本，制作KubeOS虚拟机镜像、PXE物理机镜像、升级镜像和admin容器镜像。

## 命令介绍

kbimg - CLI tool for generating various types of image for KubeOS

```text
Usage: kbimg [OPTIONS] <COMMAND> 

Commands:
  create  Create a new KubeOS image
  help    Print this message or the help of the given subcommand(s)

Options:
  -d, --debug    Enable debug mode, generate the scripts without execution
  -h, --help     Print help
  -V, --version  Print version
```

kbimg-create - Create a new KubeOS image

```text
Usage: kbimg create --file <FILE> <IMAGE_TYPE>

Arguments:
  <IMAGE_TYPE>  [possible values: vm-img, pxe-img, upgrade-img, admin-container]

Options:
  -f, --file <FILE>  Path to the toml configuration file
  -h, --help         Print help
```

## 注意事项

* 请确保已安装`qemu-img bc parted tar yum docker dosfstools`
* 制作启用dm-verity功能的镜像，需要安装`pesign nss openssl veritysetup crypto-policies`
* KubeOS镜像制作需要使用root权限
* 制作镜像时提供的 repo 文件中，yum 源建议同时配置 openEuler 具体版本的 everything 仓库和 EPOL 仓库
* KubeOS镜像制作之前需要先将当前机器上的selinux关闭或者设为允许模式
* 使用默认rpmlist进行KubeOS镜像制作至少需要有25G的剩余空间
* KubeOS镜像制作工具执行异常中断，可能会残留文件、目录或挂载，需用户手动清理，对于可能残留的rootfs目录，该目录虽然权限为555，但容器OS镜像制作在开发环境进行，不会对生产环境产生影响
* 请确保os-agent属主和属组为root，建议os-agent文件权限为500

## 配置文件说明

### from_repo

从 repo 创建升级容器镜像、虚拟机镜像或PXE物理机镜像

  | 参数 | 描述 |
  | --- | --- |
  | agent_path | os-agent 二进制的路径 |
  | legacy_bios | 镜像为 legacy 引导或 UEFI 引导 |
  | repo_path | repo 文件的路径，repo 文件中配置制作镜像所需要的 yum 源 |
  | root_passwd | root 用户密码，与/etc/shadow文件内密码格式一致，可使用`openssl passwd -6 -salt $(head -c18 /dev/urandom \| openssl base64)`命令生成 |
  | version | KubeOS 镜像的版本，将写入/etc/os-release文件内作为OS标识 |
  | rpmlist | 期望安装进镜像内的rpm包列表 |
  | upgrade_img | [OPTIONAL]指定生成的升级容器镜像的镜像名(制作升级容器镜像必需) |

### admin_container

制作admin运维容器

  | 参数 | 描述 |
  | --- | --- |
  | hostshell | hostshell二进制路径，可在项目根目录下通过`make hostshell`编译 |
  | img_name | 指定生成的容器镜像名 |

### pxe_config

在制作PXE物理机镜像时，配置该参数用于PXE安装。制作PXE物理机镜像时必需。

  | 参数 | 描述 |
  | --- | --- |
  | server_ip | 用于下载根文件系统 tar 包的 HTTP 服务器地址 |
  | rootfs_name | 放置于 HTTP 服务器的文件系统 tar 包名称 |
  | disk | 安装 KubeOS 系统的目标磁盘名 |
  | route_ip | 配置目标机器网卡的路由 IP |
  | dhcp | [OPTIONAL] 默认为 false，启用 DHCP 模式配置网络 |
  | local_ip | [OPTIONAL] 配置目标机器网卡的 IP，dhcp 为 false 时必需 |
  | net_name | [OPTIONAL] 配置目标机器网卡名，dhcp 为 false 时必需 |
  | netmask | [OPTIONAL] 配置目标机器网卡的子网掩码，dhcp 为 false 时必需 |

### users

[OPTIONAL] 添加用户

  | 参数 | 描述 |
  | --- | --- |
  | name | 用户名 |
  | passwd | 密码 |
  | primary_groups | [OPTIONAL] 用户主组 |
  | groups | [OPTIONAL] 用户附加组 |

### copy_files

[OPTIONAL] 拷贝文件到rootfs内指定目录

  | 参数 | 描述 |
  | --- | --- |
  | dst | 目标路径 |
  | src | 源文件路径 |
  | create_dir | [OPTIONAL]拷贝前创建文件夹 |

### grub

[OPTIONAL] grub配置，配置dm-verity时必需

  | 参数 | 描述 |
  | --- | --- |
  | passwd | grub 密码 |

### systemd_service

[OPTIONAL] 新增 systemd 服务

  | 参数 | 描述 |
  | --- | --- |
  | name | systemd 服务名 |

### chroot_script

[OPTIONAL] 自定义 chroot 脚本

  | 参数 | 描述 |
  | --- | --- |
  | path | 脚本路径 |
  | rm | [OPTIONAL]执行完毕后是否删除该脚本 |

### disk_partition

[OPTIONAL] 自定义分区大小和镜像大小

  | 参数 | 描述 |
  | --- | --- |
  | root | root分区大小, 单位为MiB |
  | img_size | [OPTIONAL]镜像大小，单位为GB |

### persist_mkdir

[OPTIONAL] persist 分区新建目录

  | 参数 | 描述 |
  | --- | --- |
  | name | 目录名 |

### dm_verity

[OPTIONAL] 制作启用dm-verity功能的虚拟机或升级镜像

  | 参数 | 描述 |
  | --- | --- |
  | efi_key | efi明文口令 |
  | grub_key | grub明文口令 |
  | keys_dir |[OPTIONAL]可指定密钥文件夹，复用先前制作镜像创建的密钥  |

## 使用示例

### KubeOS 虚拟机镜像制作

* 支持CPU 架构为 x86 和 aarch64 的虚拟机场景。若x86架构的虚拟机需要使用 legacy 启动模式，请在`[from_repo]`下配置`legacy_bios`为`true`
* `repo_path`为制作镜像所需要的 yum 源文件路径，yum 源建议配置为 openEuler 具体版本的 everything 仓库和 EPOL 仓库。
* 默认root密码为openEuler12#$
* 容器OS运行底噪<150M (不包含k8s组件及相关依赖`kubernetes-kubeadm，kubernetes-kubelet， containernetworking-plugins，socat，conntrack-tools，ebtables，ethtool`)

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
  * system.qcow2: qcow2 格式的系统镜像，大小默认为 20GiB，支持的根文件系统分区大小 < 2560 MiB，持久化分区 < 15GB 。
  * system.img: img 格式的系统镜像，大小默认为 20GiB，支持的根文件系统分区大小 < 2560 MiB，持久化分区 < 15GB 。
  * kubeos.tar: 用于升级的根文件系统tar包

### KubeOS 升级容器镜像制作

* 制作KubeOS容器镜像

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
upgrade_img = "kubeos-upgrade:v1"
version = "v1"
```

* 制作完成后，通过`docker images`查看制作出来的KubeOS容器镜像

### KubeOS PXE物理机安装所需镜像及文件制作

* 支持CPU 架构为 x86 和 aarch64 的物理机场景，不支持legacy引导模式
* PXE物理机镜像制作不支持dm-verity功能
* 使用默认的 rpmlist 进行镜像制作时，所需磁盘空间至少为 5GB。如果使用自定义的 rpmlist，可能需要超过 5GB 的磁盘空间。
* 在 PXE 安装阶段，需要从 HTTP 服务器下载根分区 tar 包。请确保机器拥有足够的内存空间以存储根分区 tar 包及临时中间文件。
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
```
