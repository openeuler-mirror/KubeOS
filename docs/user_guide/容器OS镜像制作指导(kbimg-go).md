# 容器OS镜像制作指导

## 简介 ##

kbimg是KubeOS部署和升级所需的镜像制作工具，可以使用kbimg制作KubeOS 容器，虚拟机和物理机镜像

## 配置文件介绍 ##

### 制作选项 option_config ###
#### 参数说明 ###
* OPTIONS

  | 参数         | 描述                                                         |
    | ------------ | ------------------------------------------------------------ |
  | image           | 镜像制作的方式，upgrade_image、vm_image_repo/docker、pxe_image_repo/docker       |
  | p           | repo 文件的路径，repo 文件中配置制作镜像所需要的 yum 源        |
  | v           | 制作出来的KubeOS镜像的版本                                   |
  | b           | os-agent二进制的路径                                         |
  | e           | KubeOS 镜像 root 用户密码，加密后的带盐值的密码，可以用 openssl，kiwi 命令生成 |
  | d           | 生成或者使用的 docke r镜像                                     |
  
* 镜像制作方式

  | 参数          | 描述                           |
    |------------------------------| ---------------------------------------------- |
  | upgrade_image | 生成用于安装和升级的OCI镜像格式的 KubeOS 镜像 |
  | vm_image_repo      | 使用 repo 源生成用于部署和升级的虚拟机镜像              |
  | vm_image_docker      | 使用 docker 镜像生成用于部署和升级的虚拟机镜像              |
  | pxe_image_repo     | 使用 repo 源生成物理机安装所需的镜像及文件              |
  | pxe_image_docker    | 使用 docker 镜像生成物理机安装所需的镜像及文件              |

### 分区选项 partition_config
#### 参数说明 ###

  | 参数          | 描述                           |
  |------------------------------| ---------------------------------------------- |
  | label | 使用 parted 创建分区时需要指定分区名称 |
  | limit  | 代表上一个分区的终止位置到这一个分区的终止位置，单位为MiB，最后一个分区的 limit 是100%，不过 yaml 格式限制必须用数字       |
  | type  | 分区的文件系统类型              |

### 文件选项 file_config
#### 参数说明 ###

  | 参数          | 描述                           |
  |------------------------------| ---------------------------------------------- |
  | sourcePath | 想要迁移的文件/文件夹位置 |
  | targetPath  | 制作出的容器 OS 镜像中的目标位置 |

### 用户选项 user_config
#### 参数说明 ###

  | 参数          | 描述                           |
  |------------------------------| ---------------------------------------------- |
  | name | 用户名 |
  | passwd  | 用户密码，可以通过 openssl passwd -crypt xxxx ，根据给出的加密密码设置 |
  | groups | 用户组名称 |


### 主机选项 host_config
#### 参数说明 ###

  | 参数          | 描述                           |
  |------------------------------| ---------------------------------------------- |
  | hostname | 主机名称 |

### grub 选项 grub_config
#### 参数说明 ###

  | 参数          | 描述                           |
  |------------------------------| ---------------------------------------------- |
  | password | grub2 密码，通过 grub2-mkpasswd-pbkdf2 得到 |

### systemd 选项 systemd_service_config
#### 参数说明 ###

  | 参数          | 描述                           |
  |------------------------------| ---------------------------------------------- |
  | name | systemd service 的名称，可以在 file_config 中将 systemd 文件迁移进容器 OS |
  | start | true 为开机自启动，false 为不启动 |


## 使用说明 ##

#### 注意事项 ###

* kbimg.sh 执行需要 root 权限
* 当前仅支持 x86和 AArch64 架构使用
* 容器 OS 镜像制作工具的 rpm 包源为 openEuler 具体版本的 everything 仓库和 EPOL 仓库。制作镜像时提供的 repo 文件中，yum 源建议同时配置 openEuler 具体版本的 everything 仓库和 EPOL 仓库

### KubeOS OCI 镜像制作 ###

#### 注意事项 ####

* 制作的 OCI 镜像仅用于后续的虚拟机/物理机镜像制作或升级使用，不支持启动容器
* 使用默认 rpmlist 进行容器OS镜像制作时所需磁盘空间至少为6G，如自已定义 rpmlist 可能会超过6G

#### 使用示例 ####
* 制作KubeOS容器镜像，请在 `cmd/kbimg/scripts` 目录下自定义 `kbimg.yaml` 文件，这里提供一份制作文件内容的参考，别的镜像制作方式按照同样思路来指定配置文件内容
```shell
option_config:
  image: upgrade_image
  p: /etc/yum.repos.d/openEuler.repo
  v: v1
  b: /opt/KubeOS/bin/os-agent
  e: '$1$jGm34rPK$wljTBdbK9CRaJnG55/hEg1'
  d: openeuler_repository/openeuler:v4

partition_config:
  - Label: BOOT
	limit: 80
    type: ext4
  - Label: ROOT-A
    limit: 3180
    type: ext4
  - Label: ROOT-B
    limit: 4280
    type: ext4
  - Label: TEST-A
	limit: 5280
    type: vfat
  - Label: TEST-B
    limit: 6380
	type: vfat
  - Label: TEST-C
	limit: 7380
	type: ext4
  - Label: PERSIST
	limit: 999999999
	type: ext4
	
file_config:
  - sourcePath: /opt/Testfiles/test.service
    targetPath: /etc/systemd/system

user_config:
  - name: goer1
    passwd: 'kqjlRtDdGR/q6'
    groups: root

host_config:
  hostname: Salamanca

grub_config:
  password:xxxxx

systemd_service_config:
  - name: test.service
    start: true
```

- 启动 kbimg 进行制作
```bash
kbimg --config .../../kbimg.yaml
```
* 制作完成后查看制作出来的KubeOS容器镜像

``` bash
docker images
```

### KubeOS 虚拟机镜像制作 ###

#### 注意事项 ####

* 如使用 docker 镜像制作请先拉取相应镜像或者先制作docker镜像，并保证 docker 镜像的安全性
* 制作出来的容器 OS 虚拟机镜像目前只能用于 CPU 架构为 x86 和 AArch64 的虚拟机
* 容器 OS 目前不支持 x86 架构的虚拟机使用 legacy 启动模式启动
* 使用默认rpmlist进行容器OS镜像制作时所需磁盘空间至少为25G，如自已定义rpmlist可能会超过25G

#### 使用示例 ####

* 使用repo源制作
  - 更改配置文件中的镜像制作方式为 vm_image_repo

 ```bash
  option_config:
  image: vm_image_repo
  p: /etc/yum.repos.d/openEuler.repo
  v: v1
  b: /opt/KubeOS/bin/os-agent
  e: '$1$VF.Nxbia$oECrfHoXPiZBOhdF/3A4Z/'
```

- 使用docker镜像制作
  - 更改配置文件中的镜像制作方式为 vm_image_docker

``` bash
  option_config:
  image: vm_image_docker
  d: openeuler_repository/openeuler:v3
```

- 启动 kbimg 进行制作

```bash
kbimg --config .../../kbimg.yaml
```

* 结果说明    
  容器 OS 镜像制作完成后，会生成：
    * system.qcow2: qcow2 格式的系统镜像，大小默认为 20GiB，支持的根文件系统分区大小 < 2020 MiB，持久化分区 < 16GiB 。
    * update.img: 用于升级的根文件系统分区镜像


### KubeOS 物理机安装所需镜像及文件制作 ###

#### 注意事项 ####

* 如使用 docker 镜像制作请先拉取相应镜像或者先制作 docker 镜像，并保证 docker 镜像的安全性
* 制作出来的容器 OS 物理安装所需的镜像目前只能用于 CPU 架构为 x86 和 AArch64 的物理机安装
* Global.cfg配置中指定的ip为安装时使用的临时ip,请在系统安装启动后请参考《openEuler 22.09 管理员指南-配置网络》进行网络配置
* 不支持多个磁盘都安装KubeOS，可能会造成启动失败或挂载紊乱
* 容器OS 目前不支持 x86 架构的物理机使用 legacy 启动模式启动
* 使用默认rpmlist进行镜像制作时所需磁盘空间至少为5G，如自已定义 rpmlist 可能会超过5G
#### 使用示例 ####

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

- KubeOS物理机安装所需镜像制作
    - 如需进行DNS配置，请在```scripts```目录下自定义```resolv.conf```文件
```shell
    cd /opt/kubeOS/scripts
    touch resolv.conf
    vim resolv.conf
```

- 使用 repo 源制作
```bash
  option_config:
  image: pxe_image_repo
  p: /etc/yum.repos.d/openEuler.repo
  v: v1
  b: /opt/KubeOS/bin/os-agent
  e: '$1$VF.Nxbia$oECrfHoXPiZBOhdF/3A4Z/'
```

- 使用 docker 镜像制作
``` bash
  option_config:
  image: pxe_image_docker
  d: openeuler_repository/openeuler:v3
```

- 启动 kbimg 进行制作

```bash
kbimg --config .../../kbimg.yaml
```

* 结果说明
    * initramfs.img: 用于pxe启动用的 initramfs 镜像
    * kubeos.tar: pxe安装所用的 OS
