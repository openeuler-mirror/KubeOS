# 容器OS镜像制作指导

## 简介

kbimg是KubeOS部署和升级所需的镜像制作工具，可以使用kbimg制作KubeOS 容器，虚拟机和物理机镜像

## 命令介绍

### 命令格式

**bash kbimg.sh** \[ --help | -h \] create \[ COMMANDS \]  \[ OPTIONS \]

### 参数说明

* COMMANDS

  | 参数          | 描述                           |
    |------------------------------| ---------------------------------------------- |
  | upgrade-image | 生成用于安装和升级的OCI镜像格式的 KubeOS 镜像 |
  | vm-image      | 生成用于部署和升级的虚拟机镜像              |
  | pxe-image     | 生成物理机安装所需的镜像及文件              |

* OPTIONS

  | 参数         | 描述                                                         |
    | ------------ | ------------------------------------------------------------ |
  | -p           | repo 文件的路径，repo 文件中配置制作镜像所需要的 yum 源        |
  | -v           | 制作出来的KubeOS镜像的版本                                   |
  | -b           | os-agent二进制的路径                                         |
  | -e           | KubeOS 镜像 root 用户密码，加密后的带盐值的密码，可以用 openssl，kiwi 命令生成 |
  | -d           | 生成或者使用的 docke r镜像                                     |
  | -l           | 如果指定参数，则镜像为legacy引导，不指定默认是UEFI引导               |
  | -h  --help | 查看帮助信息                                                 |

## 使用说明

### 注意事项

* kbimg.sh 执行需要 root 权限
* 当前仅支持 x86和 AArch64 架构使用
* 容器 OS 镜像制作工具的 rpm 包源为 openEuler 具体版本的 everything 仓库和 EPOL 仓库。制作镜像时提供的 repo 文件中，yum 源建议同时配置 openEuler 具体版本的 everything 仓库和 EPOL 仓库

### KubeOS OCI 镜像制作

#### 注意事项

* 制作的 OCI 镜像仅用于后续的虚拟机/物理机镜像制作或升级使用，不支持启动容器
* 使用默认 rpmlist 进行容器OS镜像制作时所需磁盘空间至少为6G，如自已定义 rpmlist 可能会超过6G

#### 使用示例

* 如需进行DNS配置，请先在```scripts```目录下自定义```resolv.conf```文件

```shell
  cd /opt/kubeOS/scripts
  touch resolv.conf
  vim resolv.conf
```

* 制作KubeOS容器镜像

``` shell
cd /opt/kubeOS/scripts
bash kbimg.sh create upgrade-image -p xxx.repo -v v1 -b ../bin/os-agent -e '''$1$xyz$RdLyKTL32WEvK3lg8CXID0''' -d your_imageRepository/imageName:version 
```

* 制作完成后查看制作出来的KubeOS容器镜像

``` shell
docker images
```

### KubeOS 虚拟机镜像制作

#### 注意事项

* 如使用 docker 镜像制作请先拉取相应镜像或者先制作docker镜像，并保证 docker 镜像的安全性
* 制作出来的容器 OS 虚拟机镜像目前只能用于 CPU 架构为 x86 和 AArch64 的虚拟机
* 容器 OS 目前不支持 x86 架构的虚拟机使用 legacy 启动模式启动
* 使用默认rpmlist进行容器OS镜像制作时所需磁盘空间至少为25G，如自已定义rpmlist可能会超过25G

#### 使用示例

* 使用repo源制作
  * 如需进行DNS配置，请先在```scripts```目录下自定义```resolv.conf```文件

  ```shell
  cd /opt/kubeOS/scripts
  touch resolv.conf
  vim resolv.conf
  ```

  * KubeOS虚拟机镜像制作

  ``` shell
  cd /opt/kubeOS/scripts
  bash kbimg.sh create vm-image -p xxx.repo -v v1 -b ../bin/os-agent -e '''$1$xyz$RdLyKTL32WEvK3lg8CXID0'''
  ```

* 使用docker镜像制作

  ``` shell
  cd /opt/kubeOS/scripts
  bash kbimg.sh create vm-image -d  your_imageRepository/imageName:version
  ```

* 结果说明
  容器 OS 镜像制作完成后，会在 /opt/kubeOS/scripts 目录下生成：
  * system.qcow2: qcow2 格式的系统镜像，大小默认为 20GiB，支持的根文件系统分区大小 < 2020 MiB，持久化分区 < 16GiB 。
  * update.img: 用于升级的根文件系统分区镜像

### KubeOS 物理机安装所需镜像及文件制作

#### 注意事项

* 如使用 docker 镜像制作请先拉取相应镜像或者先制作 docker 镜像，并保证 docker 镜像的安全性
* 制作出来的容器 OS 物理安装所需的镜像目前只能用于 CPU 架构为 x86 和 AArch64 的物理机安装
* Global.cfg配置中指定的ip为安装时使用的临时ip,请在系统安装启动后请参考《openEuler 22.09 管理员指南-配置网络》进行网络配置
* 不支持多个磁盘都安装KubeOS，可能会造成启动失败或挂载紊乱
* 容器OS 目前不支持 x86 架构的物理机使用 legacy 启动模式启动
* 使用默认rpmlist进行镜像制作时所需磁盘空间至少为5G，如自已定义 rpmlist 可能会超过5G

#### 使用示例

* 首先需要修改```00bootup/Global.cfg```的配置，对相关参数进行配置，参数均为必填，ip目前仅支持ipv4，配置示例如下

  ```shell
  # rootfs file name
  rootfs_name=kubeos.tar
  # select the target disk to install kubeOS
  disk=/dev/sda
  # pxe server ip address where stores the rootfs on the http server
  server_ip=192.168.1.50
  # target machine temporary ip
  local_ip=192.168.1.100
  # target machine temporary route
  route_ip=192.168.1.1
  # target machine temporary netmask
  netmask=255.255.255.0
  # target machine netDevice name
  net_name=eth0
  ```

* 使用 repo 源制作
  * 如需进行DNS配置，请在```scripts```目录下自定义```resolv.conf```文件

  ```shell
    cd /opt/kubeOS/scripts
    touch resolv.conf
    vim resolv.conf
  ```

  * KubeOS物理机安装所需镜像制作

  ```shell
    cd /opt/kubeOS/scripts
    bash kbimg.sh create pxe-image -p xxx.repo -v v1 -b ../bin/os-agent -e '''$1$xyz$RdLyKTL32WEvK3lg8CXID0'''
  ```

* 使用 docker 镜像制作

  ``` shell
  cd /opt/kubeOS/scripts
  bash kbimg.sh create pxe-image -d your_imageRepository/imageName:version
  ```

* 结果说明
  * initramfs.img: 用于pxe启动用的 initramfs 镜像
  * kubeos.tar: pxe安装所用的 OS


# KubeOS-Rust 镜像制作说明

## 简介

KubeOS 虚拟机镜像制作的 Rust 二进制版本

## 命令介绍

### 命令格式

**.../kbimg** \[ --config | -c \] \<path to kbimg.toml\>

## 配置文件说明

* from_repo: 从 repo 创建 OCI 镜像、虚拟机镜像或物理机镜像

  | 参数 | 描述 |
  | --- | --- |
  | agent_path | os-agent 二进制的路径 |
  | image_type | upgrade: 用于安装和升级的 OCI 镜像格式的 KubeOS 镜像; vm-repo: 用于部署和升级的虚拟机镜像; pxe-repo: 物理机安装所需的镜像及文件 |
  | legacy_bios | 镜像为 legacy 引导或 UEFI 引导 |
  | repo_path | repo 文件的路径，repo 文件中配置制作镜像所需要的 yum 源 |
  | root_passwd | KubeOS 镜像 root 用户密码，加密后的带盐值的密码，可以用 openssl、kiwi 命令生成 |
  | version | 制作出来的 KubeOS 镜像的版本 |
  | rpmlist | 镜像所需的 rpm 包 |
  | docker_img | 生成或者使用的 docker 镜像 |

* from_docker: 从 docker 镜像创建虚拟机镜像或物理机镜像

  | 参数 | 描述 |
  | --- | --- |
  | docker_img | 生成或者使用的 docker 镜像 |
  | image_type | vm-docker: 用于部署和升级的虚拟机镜像; pxe-docker: 物理机安装所需的镜像及文件 |

* admin_container: 

  | 参数 | 描述 |
  | --- | --- |
  | dockerfile | dockerfile 路径 |
  | docker_img | 生成或者使用的 docker 镜像 |

* [OPTIONAL] users: 添加用户

  | 参数 | 描述 |
  | --- | --- |
  | groups | [OPTIONAL] 用户组 (第一个为主组，其他为附加组) |
  | name | 用户名 |
  | passwd | 密码 |
  | sudo | [OPTIONAL] 用户是否具有 sudo 权限 |

* [OPTIONAL] copy_files: 拷贝文件到指定目录

  | 参数 | 描述 |
  | --- | --- |
  | dst | 目标目录 |
  | src | 源文件路径 |

* [OPTIONAL] grub: grub配置

  | 参数 | 描述 |
  | --- | --- |
  | passwd | [OPTIONAL] grub 密码 |

* [OPTIONAL] systemd_service: 新增 systemd 服务

  | 参数 | 描述 |
  | --- | --- |
  | name | systemd 服务名 |

* [OPTIONAL] chroot_script: 自定义 chroot 脚本

  | 参数 | 描述 |
  | --- | --- |
  | path | 脚本路径 |

* [OPTIONAL] disk_partition: 自定义分区大小和镜像大小

  | 参数 | 描述 |
  | --- | --- |
  | first | 引导分区大小 |
  | second | ROOT-A 分区大小 | 
  | third | ROOT-B 分区大小 |
  | img_size | 镜像大小 |

* [OPTIONAL] persist_mkdir: persist 分区新建目录

  | 参数 | 描述 |
  | --- | --- |
  | name | 目录名 |

## 使用说明

#### 注意事项

* 新增 systemd 服务需要将对应的 .service 文件或 .mount 文件拷贝至镜像```/usr/lib/systemd/system```目录

  ```toml
  [[copy_files]]
  dst = "/usr/lib/systemd/system"
  src = ".../containerd.service"

  [systemd_service]
  name = ["containerd"]
  ```

  * 如需挂载数据盘，请先自定义```persist-data.mount```文件，并启用```copy_files```和```systemd_service```字段设置启动时挂载，启用```persist_mkdir```字段创建挂载点
  * .mount文件名由挂载点路径生成，将斜杠替换为连字符
  * 请先在磁盘映像文件上创建ext4文件系统

    ```
    # persist-data.mount
    [Unit]
    Description=Mount Disk
    Documentation=man:systemd.mount(5)

    [Mount]
    What=/dev/vdb
    Where=/persist/data
    Type=ext4
    Options=defaults,noatime

    [Install]
    WantedBy=local-fs.target
    ```

    ```toml
    [[copy_files]]
    dst = "/usr/lib/systemd/system"
    src = ".../persist-data.mount"

    [systemd_service]
    name = ["persist-data.mount"]

    [persist_mkdir]
    name = ["data"]
    ```

  * 如需配置逻辑卷，请先自定义```volume.service```文件，并启用```copy_files```和```systemd_service```设置启动时配置逻辑卷，启用```persist_mkdir```字段创建挂载点

    ```
    # volume.service
    [Unit]
    Description=Mount Logical Volume
    After=local-fs.target

    [Service]
    Type=oneshot
    RemainAfterExit=yes
    ExecStart=pvcreate /dev/vdb
    ExecStart=pvcreate /dev/vdc
    ExecStart=vgcreate my_vg /dev/vdb /dev/vdc
    ExecStart=lvcreate -L 15G -n my_lv my_vg
    ExecStart=mkfs.ext4 /dev/my_vg/my_lv
    ExecStart=mount /dev/my_vg/my_lv /persist/lv_data

    [Install]
    WantedBy=local-fs.target
    ```

    ```toml
    [[copy_files]]
    dst = "/usr/lib/systemd/system"
    src = ".../volume.service"

    [systemd_service]
    name = ["volume"]

    [persist_mkdir]
    name = ["lv_data"]
    ```

## 使用示例

### KubeOS OCI 镜像制作

* 如需进行DNS配置，请先自定义```resolv.conf```文件，并启用```copy_files```字段将配置文件拷贝到```/etc```目录

  ```shell
  touch \<arbitrary directory\>/resolv.conf
  vim \<arbitrary directory\>resolv.conf
  ```

  ```toml
  [[copy_files]]
  dst = "/etc"
  src = "<path to resolv.conf>"
  ```

* 制作KubeOS容器镜像

  ```toml
  [from_repo]
  agent_path = "<directory>/bin/os-agent"
  image_type = "upgrade"
  legacy_bios = false
  repo_path = "xxx.repo"
  root_passwd = "$1$xyz$RdLyKTL32WEvK3lg8CXID0"
  version = "v1"
  docker_img = "your_imageRepository/imageName:version"
  rpmlist = [
      # your rpms
  ]
  ```

* 制作完成后查看制作出来的KubeOS容器镜像
  ``` shell
  docker images
  ```

### KubeOS 虚拟机镜像制作

* 使用repo源制作

    * 如需进行DNS配置，请先自定义```resolv.conf```文件，并启用**copy_files**字段将配置文件拷贝到```/etc```目录
    
      ```shell
      touch \<arbitrary directory\>/resolv.conf
      vim \<arbitrary directory\>resolv.conf
      ```

      ```toml
      [[copy_files]]
      dst = "/etc"
      src = "<path to resolv.conf>"
      ```

    * KubeOS虚拟机镜像制作

      ```toml
      [from_repo]
      agent_path = "<directory>/bin/os-agent"
      image_type = "vm-repo"
      legacy_bios = false
      repo_path = "xxx.repo"
      root_passwd = "$1$xyz$RdLyKTL32WEvK3lg8CXID0"
      version = "v1"
      rpmlist = [
          # your rpms
      ]
      ```

* 使用docker镜像制作

  ```toml
  [from_dockerimg]
  docker_img = "your_imageRepository/imageName:version"
  image_type = "vm-docker"
  ```

* 结果说明    
  容器 OS 镜像制作完成后，会在 ./scripts-auto 目录下生成
    * system.qcow2: qcow2 格式的系统镜像，大小默认为 20GiB，支持的根文件系统分区大小 < 2020 MiB，持久化分区 < 16GiB 。
    * update.img: 用于升级的根文件系统分区镜像

### KubeOS 物理机安装所需镜像及文件制作

* 首先需要修改```kbimg.toml```中```pxe_config```的配置，对相关参数进行配置，参数均为必填，ip目前仅支持ipv4，配置示例如下

  ```toml
  [pxe_config]
  # rootfs file name
  rootfs_name = "kubeos.tar"
  # select the target disk to install kubeOS
  disk = "/dev/sda"
  # pxe server ip address where stores the rootfs on the http server
  server_ip = "192.168.1.50"
  # target machine ip
  local_ip = "192.168.1.100"
  # target machine route
  route_ip = "192.168.1.1"
  # target machine netmask
  netmask = "255.255.255.0"
  # target machine netDevice name
  net_name = "eth0"
  ```

* 使用 repo 源制作
  * 如需进行DNS配置，请先自定义```resolv.conf```文件，并启用```copy_files```字段将配置文件拷贝到```/etc```目录
    
    ```shell
    touch \<arbitrary directory\>/resolv.conf
    vim \<arbitrary directory\>resolv.conf
    ```

    ```toml
    [[copy_files]]
    dst = "/etc"
    src = "<path to resolv.conf>"
    ```

  * KubeOS物理机安装所需镜像制作
    ```toml
    [from_repo]
    agent_path = "<directory>/bin/os-agent"
    image_type = "pxe-repo"
    legacy_bios = true
    repo_path = "xxx.repo"
    root_passwd = "$1$xyz$RdLyKTL32WEvK3lg8CXID0"
    version = "v1"
    rpmlist = [
        # your rpms
    ]
    ```

* 使用 docker 镜像制作
  ```toml
  [from_dockerimg]
  docker_img = "your_imageRepository/imageName:version"
  image_type = "vm-docker"
  ```

* 结果说明
  * initramfs.img: 用于pxe启动用的 initramfs 镜像
  * kubeos.tar: pxe安装所用的 OS
