# 快速使用指导

[TOC]

## 编译指导

* 编译环境：openEuler Linux x86/AArch64

* 进行编译需要以下包：
  * golang(大于等于1.15版本)
  * make
  * git
  * rust(大于等于1.64版本)
  * cargo(大于等于1.64版本)
  * openssl-devel

  ``` shell
  sudo yum install golang make git rust cargo openssl-devel
  ```

* 使用git获取本项目的源码

  ``` shell
  sudo git clone https://gitee.com/openeuler/KubeOS.git
  ```

* 编译二进制
  * operator：负责控制集群的升级
  * proxy：负责集群和agent通信，从k8s传递升级指令给agent，从agent传递升级状态给k8s
  * os-agent：负责节点升级和运维

  ```shell
  cd KubeOS
  sudo make
  # 编译生成的二进制在bin目录下，查看二进制
  tree bin
  bin
  ├── operator
  ├── os-agent
  ├── proxy
  ├── rust
  │   ├── ...
  │   └── release
  │       ├── ...
  │       ├── os-agent
  │       └── proxy
  ```

  * ```bin/proxy```、```bin/os-agent```为go语言编写的proxy和os-agent，```bin/rust/release/proxy```、```bin/rust/release/os-agent```为rust语言编写的proxy和os-agent，二者功能一致。

## 镜像构建指导

### proxy及operator镜像构建指导

* proxy及operator容器镜像构建使用docker，请先确保docker已经安装和配置完毕

* 请用户自行编写Dockerfile来构建镜像，请注意
  * operator和proxy需要基于baseimage进行构建，用户保证baseimage的安全性
  * 需将operator和proxy拷贝到baseimage上
  * 请确保proxy属主和属组为root，文件权限为500
  * 请确保operator属主和属组为在容器内运行operator的用户，文件权限为500
  * operator和proxy的在容器内的位置和容器启动时运行的命令需与部署operator的yaml中指定的字段相对应

* 首先指定镜像仓库地址、镜像名及版本，Dockerfile路径，然后构建并推送镜像到镜像仓库

* Dockerfile参考如下, Dockerfile也可以使用多阶段构建:

  `proxy`容器镜像Dockerfile

  ``` dockerfile
  FROM openeuler/openeuler:24.03-lts
  COPY ./bin/proxy /proxy
  ENTRYPOINT ["/proxy"]
  ```

  `operator`容器镜像Dockerfile

  ``` dockerfile
  FROM openeuler/openeuler:24.03-lts
  COPY --chown=6552:6552 ./bin/operator /operator
  ENTRYPOINT ["/operator"]
  ```

  ```shell
  # 指定proxy的镜像仓库，镜像名及版本
  export IMG_PROXY=your_imageRepository/proxy_imageName:version
  # 指定proxy的Dockerfile地址
  export DOCKERFILE_PROXY=your_dockerfile_proxy
  # 指定operator的镜像仓库，镜像名及版本
  export IMG_OPERATOR=your_imageRepository/operator_imageName:version
  # 指定operator的Dockerfile路径
  export DOCKERFILE_OPERATOR=your_dockerfile_operator
  
  # 镜像构建
  docker build -t ${IMG_OPERATOR} -f ${DOCKERFILE_OPERATOR} .
  docker build -t ${IMG_PROXY} -f ${DOCKERFILE_PROXY} .
  # 推送镜像到镜像仓库
  docker push ${IMG_OPERATOR}
  docker push ${IMG_PROXY}
  ```

### KubeOS虚拟机镜像制作指导

* 制作注意事项
  * 请确保已安装qemu-img，bc，parted，tar，yum，docker，dosfstools
  * 容器OS镜像制作需要使用root权限
  * 容器OS 镜像制作工具的 rpm 包源为 openEuler 具体版本的 everything 仓库和 EPOL 仓库。制作镜像时提供的 repo 文件中，yum 源建议同时配置 openEuler 具体版本的 everything 仓库和 EPOL 仓库
  * 容器OS镜像制作之前需要先将当前机器上的selinux关闭或者设为允许模式
  * 使用默认rpmlist进行容器OS镜像制作出来的镜像默认和制作工具保存在相同路径，该分区至少有25G的剩余空间
  * 容器镜像制作时不支持用户自定义配置挂载文件
  * 容器OS镜像制作工具执行异常中断，可能会残留文件、目录或挂载，需用户手动清理，对于可能残留的rootfs目录，该目录虽然权限为555，但容器OS镜像制作在开发环境进行，不会对生产环境产生影响。
  * 请确保os-agent属主和属组为root，建议os-agent文件权限为500

* 容器OS虚拟机镜像制作
    在KubeOS项目根目录下，执行

    ```shell
    cargo run --package kbimg -- create -f KubeOS-Rust/kbimg/kbimg.toml vm-img 
    ```

    详细配置文件和命令行参数说明请见[KubeOS镜像制作指导](../docs/user_guide/KubeOS镜像制作指导-binary.md):
  * 本项目不提供容器OS镜像，仅提供裁剪工具，裁剪出来的容器OS内部的安全性由OS发行商保证。

* 声明： os-agent使用本地unix socket进行通信，因此不会新增端口。下载镜像的时候会新增一个客户端的随机端口，1024~65535使用完后关闭。proxy和operator与api-server通信时作为客户端也会有一个随机端口，基于kubernetes的operator框架，必须使用端口。他们部署在容器里。

## 部署指导

### os-operator和os-proxy部署指导

* 环境要求
  * openEuler Linux x86/AArch64系统
  * Kubernetes集群已部署
  * 准备进行升级的Node节点的OS为使用上一节方式制作出来的容器OS

* 部署
  * 使用kubernetes的声明式API进行配置,部署CRD（CustomResourceDefinition），operator，proxy以及rbac机制的YAML需要用户自行编写
  * YAML举例说明模板参见本目录下example文件夹下的文件夹，你也可以将config文件夹拷贝到docs上一级目录，并进行简单的修改使用
  * 这些YAML配置文件，由K8s集群管理员加载，如果恶意在yaml文件里面写了病毒，K8s集群管理员如果放行，传到我们的处理模块我们也是没有办法校验的，此处有风险
  * operator和proxy部署在kubernets集群中，operator应部署为deployment，proxy应部署为damonset
  * 尽量部署好k8s的安全措施，如rbac机制，pod的service account和security policy配置等。**注意**：operator所在容器仅需要普通用户权限运行，proxy所在容器需要root权限运行以访问worker节点上的os-agent.sock，但是可以drop全部的capabilities，如：

    ```yaml
    # operator
    spec:
      containers:
        securityContext:
          allowPrivilegeEscalation: false
          runAsUser: 6552
          runAsGroup: 6552
    ---
    # proxy
    spec:
      containers:
        securityContext:
          capabilities:
            drop:
            - all
    ```

  * 假定您已经编辑好了YAML，并且CRD，rbac机制，operator和proxy的YAML分别放在了当前目录下config/crd，config/rbac目录下和config/manager目录下，执行部署命令：

    ```shell
    kubectl apply -f confg/crd
    kubectl apply -f config/rbac 
    kubectl apply -f config/manager
    ```

  * 部署完成后通过以下命令行查看各个组件是否都正常启动,如果所有组件的STATUS都是 Running的，说明组件都正常启动了。

    ```shell
    kubectl get pods -A
    ```

## 使用指导

### 注意事项

* 公共注意事项
  * 仅支持虚拟机x86和arm64 UEFI场景。
  * 当前不支持集群节点OS多版本管理，即集群中OS的CR只能为一个。
  * 使用kubectl apply通过YAML创建或更新OS的CR时，不建议并发apply，当并发请求过多时，kube-apiserver会无法处理请求导致失败。
  * 如用户配置了容器镜像仓的证书或密钥，请用户保证证书或密钥文件的权限最小。
* 升级注意事项
  * 升级为所有软件包原子升级，默认不提供单包升级能力。
  * 升级为双区升级的方式，不支持更多分区数量。
  * 当前暂不支持跨大版本升级。
  * 单节点的升级过程的日志可在节点的 /var/log/messages 文件查看。
  * 请严格按照提供的升级和回退流程进行操作，异常调用顺序可能会导致系统无法升级或回退。
  * 节点上containerd如需配置ctr使用的私有镜像，请将配置文件host.toml按照ctr指导放在/etc/containerd/certs.d目录下。

* 配置注意事项
  * 用户自行指定配置内容，用户需保证配置内容安全可靠 ，尤其是持久化配置（kernel.sysctl.persist、grub.cmdline.current、grub.cmdline.next），KubeOS不对参数有效性进行检验。
  * opstype=config时，若osversion与当前集群节点的OS版本不一致，配置不会进行。
  * 当前仅支持kernel参数临时配置（kernel.sysctl）、持久化配置（kernel.sysctl.persist）和grub cmdline配置（grub.cmdline.current和grub.cmdline.next）。
  * 持久化配置会写入persist持久化分区，升级重启后配置保留；kernel参数临时配置重启后不保留。
  * 配置grub.cmdline.current或grub.cmdline.next时，如为单个参数（非key=value格式参数），请指定key为该参数，value为空。
  * 进行配置删除（operation=delete）时，key=value形式的配置需保证key、value和实际配置一致。
  * 配置不支持回退，如需回退，请修改配置版本和配置内容，重新下发配置。
  * 配置出现错误，节点状态陷入config时，请将配置版本恢复成上一版本并重新下发配置，从而使节点恢复至idel状态。 但是请注意：出现错误前已经配置完成的参数无法恢复。
  * 在配置grub.cmdline.current或grub.cmdline.next时，若需要将已存在的“key=value”格式的参数更新为只有key无value格式，比如将“rd.info=0”更新成rd.info，需要先删除“key=value”，然后在下一次配置时，添加key。不支持直接更新或者更新删除动作在同一次完成。

#### OS CR参数说明

在集群中创建类别为OS的定制对象，设置相应字段。类别OS来自于安装和部署章节创建的CRD对象，字段及说明如下：

* imageurl指定的地址里包含协议，只支持http或https协议。imageurl为https协议时为安全传输，imageurl为http地址时，需指定flagSafe为true，即用户明确该地址为安全时，才会下载镜像。如imageurl为http地址且没有指定flagSafe为true，默认该地址不安全，不会下载镜像并且在升级节点的日志中提示用户该地址不安全
* 对于imageurl，推荐使用https协议，使用https协议需要升级的机器已安装相应证书。如果镜像服务器由用户自己维护，需要用户自己进行签名，并保证升级节点已安装对应证书。用户需要将证书放在容器OS /etc/KubeOS/certs目录下。地址由管理员传入，管理员应该保证网址的安全性，推荐采用内网地址。
* 容器OS镜像的合法性检查需要由容器OS镜像服务提供者做合法性检查，确保下载的容器OS镜像来源可靠

  | 参数            |参数类型  | 参数说明                                                     | 使用说明 | 是否必选         |
  | -------------- | ------ | ------------------------------------------------------------ | ----- | ---------------- |
  | imagetype      | string | 升级镜像的类型           | 仅支持docker ，containerd ，或者是 disk，仅在升级场景有效。**注意**：若使用containerd，agent优先使用crictl工具拉取镜像，没有crictl时才会使用ctr命令拉取镜像。使用ctr拉取镜像时，镜像如果在私有仓内，需按照[官方文档](https://github.com/containerd/containerd/blob/main/docs/hosts.md)在/etc/containerd/certs.d目录下配置私有仓主机信息，才能成功拉取镜像。 |是               |
  | opstype        | string | 操作类型：升级,回退或者配置 | 仅支持upgrade ，config 或者 rollback |是               |
  | osversion      | string | 升级/回退的目标版本  | osversion需与节点的目标os版本对应（节点上/etc/os-release中PRETTY_NAME字段或k8s检查到的节点os版本） 例如：KubeOS 1.0.0。 |是               |
  | maxunavailable | int    | 每批同时进行升级/回退/配置的节点数。 | maxunavailable值大于实际节点数时，取实际节点数进行升级/回退/配置。 |是               |
  | containerimage    | string | 用于升级的容器镜像               | 仅在imagetype是容器类型时生效，仅支持以下3种格式的容器镜像地址： repository/name repository/name@sha256:xxxx repository/name:tag |是               |
  | imageurl       | string | 用于升级的磁盘镜像的地址 | imageurl中包含协议，只支持http或https协议，例如：<https://192.168.122.15/update.img> ，仅在使用磁盘镜像升级场景下有效 |是               |
  | checksum       | string | 用于升级的磁盘镜像校验的checksum(SHA-256)值或者是用于升级的容器镜像的digests值                      | 仅在升级场景下有效 |是               |
  | flagSafe       | bool   | 当imageurl的地址使用http协议表示是否是安全的                 | 需为 true 或者 false ，仅在imageurl使用http协议时有效 |是               |
  | mtls           | bool   | 用于表示与imageurl连接是否采用https双向认证     | 需为 true 或者 false ，仅在imageurl使用https协议时有效|是               |
  | cacert         | string | https或者https双向认证时使用的根证书文件                       | 仅在imageurl使用https协议时有效| imageurl使用https协议时必选 |
  | clientcert     | string | https双向认证时使用的客户端证书文件                          | 仅在使用https双向认证时有效|mtls为true时必选 |
  | clientkey      | string | https双向认证时使用的客户端公钥                              | 仅在使用https双向认证时有效|mtls为true时必选 |
  | evictpodforce      | bool | 升级/回退时是否强制驱逐pod                            | 需为 true 或者 false ，仅在升级或者回退时有效| 必选 |
  | sysconfigs      | / | 配置设置                          | 1. “opstype=config”时只进行配置。  2.“opstype=upgrade/rollback”时，代表升级/回退后配置，即在升级/回退重启后进行配置。```配置（Settings）指导``` | “opstype=config”时必选 |
  | upgradeconfigs | / | 升级前配置设置                       | 在升级或者回退时有效，在升级或者回退操作之前起效，详细字段说明请见```配置（Settings）指导```| 可选 |
  | nodeselector      | string | 需要进行升级/配置/回滚操作的节点label                           | 用于只对具有某些特定label的节点而不是集群所有worker节点进行运维的场景，需要进行运维操作的节点需要包含key为upgrade.openeuler.org/node-selector的label，nodeselector为该label的value值，此参数不配置时，或者配置为""时默认对所有节点进行操作| 可选 |
#### 升级指导

* 编写YAML文件，在集群中部署 OS 的cr实例，用于部署cr实例的YAML示例如下，假定将上面的YAML保存到upgrade_v1alpha1_os.yaml;
  * 使用磁盘镜像进行升级

      ```yaml
      apiVersion: upgrade.openeuler.org/v1alpha1
      kind: OS
      metadata:
        name: os-sample
      spec:
        imagetype: disk
        opstype: upgrade
        osversion: edit.os.version
        maxunavailable: edit.node.upgrade.number
        containerimage: ""
        evictpodforce: true/false
        imageurl: edit.image.url
        checksum: image.checksum
        flagSafe: imageurl.safety
        mtls: imageurl use mtls or not
        cacert:  ca certificate 
        clientcert:  client certificate 
        clientkey:  client certificate key 
      ```

  * 使用容器镜像进行升级
    * 使用容器镜像进行升级前请先制作升级所需的容器镜像，制作方式请见[《容器OS镜像制作指导》](../docs/user_guide/%E5%AE%B9%E5%99%A8OS%E9%95%9C%E5%83%8F%E5%88%B6%E4%BD%9C%E6%8C%87%E5%AF%BC.md)中 ```KubeOS OCI 镜像制作```
    * 节点容器引擎为docker

      ``` yaml
      apiVersion: upgrade.openeuler.org/v1alpha1
      kind: OS
      metadata:
        name: os-sample
      spec:
        imagetype: docker
        opstype: upgrade
        osversion: edit.os.version
        maxunavailable: edit.node.upgrade.number
        containerimage: container image like repository/name:tag
        evictpodforce: true/false
        imageurl: ""
        checksum: container image digests
        flagSafe: false
        mtls: true
      ```

    * 节点容器引擎为containerd

      ```yaml
      apiVersion: upgrade.openeuler.org/v1alpha1
      kind: OS
      metadata:
        name: os-sample
      spec:
        imagetype: containerd
        opstype: upgrade
        osversion: edit.os.version
        maxunavailable: edit.node.upgrade.number
        containerimage: container image like repository/name:tag
        evictpodforce: true/false
        imageurl: ""
        checksum: container image digests
        flagSafe: false
        mtls: true
      ```

    * 升级并且进行配置的示例如下
      * 以节点容器引擎为containerd为例，升级方式对配置无影响，upgradeconfigs在升级前起效，sysconfigs在升级后起效，配置参数说明请见```配置(Settings)指导```
      * 升级并且配置时opstype字段需为upgrade
      * upgradeconfig为升级之前执行的配置，sysconfigs为升级机器重启后执行的配置，用户可按需进行配置

        ```yaml
        apiVersion: upgrade.openeuler.org/v1alpha1
        kind: OS
        metadata:
            name: os-sample
        spec:
            imagetype: ""
            opstype: upgrade
            osversion: edit.os.version
            maxunavailable: edit.node.upgrade.number
            containerimage: ""
            evictpodforce: true/false
            imageurl: ""
            checksum: container image digests
            flagSafe: false
            mtls: false
            sysconfigs:
                version: edit.os.version
                configs:
                    - model: kernel.sysctl
                    contents:
                        - key: kernel param key1
                          value: kernel param value1
                        - key: kernel param key2
                          value: kernel param value2
                    - model: kernel.sysctl.persist
                      configpath: persist file path
                      contents:
                        - key: kernel param key3
                          value: kernel param value3
                        - key: ""
                          value: ""
            upgradeconfigs:
                version: 1.0.0
                configs:
                    - model: kernel.sysctl
                    contents:
                        - key: kernel param key4
                          value: kernel param value4          
        ```
    * 只升级部分节点示例如下
      * 以节点容器引擎为containerd为例，升级方式对节点筛选无影响
      * 需要进行升级的节点需包含key为upgrade.openeuler.org/node-selector的label，nodeselector的值为该label的value，即假定nodeselector值为kubeos，则只对包含upgrade.openeuler.org/node-selector=kubeos的label的worker节点进行升级
      * nodeselector对配置和回滚同样有效
      * 节点添加label和label修改命令示例如下：
      ``` shell
      # 为节点kubeos-node1增加label
      kubectl label nodes kubeos-node1 upgrade.openeuler.org/node-selector=kubeos-v1
      # 修改节点kubeos-node1的label
      kubectl label --overwrite nodes kubeos-node2 upgrade.openeuler.org/node-selector=kubeos-v2

      ```
      * yaml示例如下：
      ```yaml
      apiVersion: upgrade.openeuler.org/v1alpha1
      kind: OS
      metadata:
        name: os-sample
      spec:
        imagetype: containerd
        opstype: upgrade
        osversion: edit.os.version
        maxunavailable: edit.node.upgrade.number
        containerimage: container image like repository/name:tag
        evictpodforce: true/false
        imageurl: ""
        checksum: container image digests
        flagSafe: false
        mtls: true
        nodeselector: edit.node.label.key
      ```
* 查看未升级的节点的 OS 版本

    ```shell
    kubectl get nodes -o custom-columns='NAME:.metadata.name,OS:.status.nodeInfo.osImage'
    ```

* 执行命令，在集群中部署cr实例后，节点会根据配置的参数信息进行升级。

    ```shell
    kubectl apply -f upgrade_v1alpha1_os.yaml
    ```

* 再次查看节点的 OS 版本来确认节点是否升级完成

    ```shell
    kubectl get nodes -o custom-columns='NAME:.metadata.name,OS:.status.nodeInfo.osImage'
    ```

* 如果后续需要再次升级，与上面相同，对upgrade_v1alpha1_os.yaml的相应字段进行修改

#### 配置（Settings）指导

* Settings参数说明:

  基于示例YAML对配置的参数进行说明，示例YAML如下，配置的格式（缩进）需和示例保持一致：

  ```yaml
  apiVersion: upgrade.openeuler.org/v1alpha1
  kind: OS
  metadata:
    name: os-sample
  spec:
    imagetype: ""
    opstype: config
    osversion: edit.os.version
    maxunavailable: edit.node.config.number
    containerimage: ""
    evictpodforce: false
    checksum: ""
    sysconfigs:
        version: edit.sysconfigs.version
        configs:
            - model: kernel.sysctl
              contents: 
                - key: kernel param key1
                  value: kernel param value1
                - key: kernel param key2
                  value: kernel param value2
                  operation: delete
            - model: kernel.sysctl.persist
              configpath: persist file path
              contents:
                - key: kernel param key3
                  value: kernel param value3
            - model: grub.cmdline.current
              contents:
                - key: boot param key1
                - key: boot param key2
                  value: boot param value2
                - key: boot param key3
                  value: boot param value3
                  operation: delete
            - model: grub.cmdline.next
              contents:
                - key: boot param key4
                - key: boot param key5
                  value: boot param value5
                - key: boot param key6
                  value: boot param value6
                  operation: delete         
  ```

  配置的参数说明如下：

  | 参数       | 参数类型 | 参数说明                    | 使用说明                                                     | 配置中是否必选          |
  | ---------- | -------- | --------------------------- | ------------------------------------------------------------ | ----------------------- |
  | version    | string   | 配置的版本                  | 通过version是否相等来判断配置是否触发，version为空（为""或者没有值）时同样进行判断，所以不配置sysconfigs/upgradeconfigs时，继存的version值会被清空并触发配置。 | 是                      |
  | configs    | /        | 具体配置内容                | 包含具体配置项列表。                                         | 是                      |
  | model      | string   | 配置的类型                  | 支持的配置类型请看附录下的```Settings列表```                 | 是                      |
  | configpath | string   | 配置文件路径                | 仅在kernel.sysctl.persist配置类型中生效，请看附录下的```Settings列表```对配置文件路径的说明。 | 否                      |
  | contents   | /        | 具体key/value的值及操作类型 | 包含具体配置参数列表。                                       | 是                      |
  | key        | string   | 参数名称                    | key不能为空，不能包含"="，不建议配置含空格、tab键的字符串，具体请看附录下的```Settings列表```中每种配置类型对key的说明。 | 是                      |
  | value      | string   | 参数值                      | key=value形式的参数中，value不能为空，不建议配置含空格、tab键的字符串，具体请看附录下的```Settings列表```中对每种配置类型对value的说明。 | key=value形式的参数必选 |
  | operation  | string   | 对参数进行的操作            | 仅对kernel.sysctl.persist、grub.cmdline.current、grub.cmdline.next类型的参数生效。默认为添加或更新。仅支持配置为delete，代表删除已存在的参数（key=value需完全一致才能删除）。 | 否                      |

  * upgradeconfigs与sysconfigs参数相同，upgradeconfigs为升级/回退前进行的配置，仅在upgrade/rollback场景起效，sysconfigs既支持只进行配置，也支持在升级/回退重启后进行配置

* 使用说明

  * 编写YAML文件，在集群中部署 OS 的cr实例，用于部署cr实例的YAML示例如上，假定将上面的YAML保存到upgrade_v1alpha1_os.yaml

  * 查看配置之前的节点的配置的版本和节点状态（NODESTATUS状态为idle）

    ```shell
    kubectl get osinstances -o custom-columns='NAME:.metadata.name,NODESTATUS:.spec.nodestatus,SYSCONFIG:status.sysconfigs.version,UPGRADECONFIG:status.upgradeconfigs.version'
    ```

  * 执行命令，在集群中部署cr实例后，节点会根据配置的参数信息进行配置，再次查看节点状态(NODESTATUS变成config)

    ```shell
    kubectl apply -f upgrade_v1alpha1_os.yaml
    kubectl get osinstances -o custom-columns='NAME:.metadata.name,NODESTATUS:.spec.nodestatus,SYSCONFIG:status.sysconfigs.version,UPGRADECONFIG:status.upgradeconfigs.version'
    ```

  * 再次查看节点的配置的版本确认节点是否配置完成(NODESTATUS恢复为idle)

    ```shell
    kubectl get osinstances -o custom-columns='NAME:.metadata.name,NODESTATUS:.spec.nodestatus,SYSCONFIG:status.sysconfigs.version,UPGRADECONFIG:status.upgradeconfigs.version'
    ```

* 如果后续需要再次配置，与上面相同对 upgrade_v1alpha1_os.yaml 的相应字段进行相应修改。

#### 回退指导

* 回退场景
  * 虚拟机无法正常启动时，可在grub启动项页面手动切换启动项，使系统回退至上一版本（即手动回退）。
  * 虚拟机能够正常启动并且进入系统时，支持工具回退和手动回退，建议使用工具回退。
  * 工具回退有两种方式：
    1. rollback模式直接回退至上一版本。
    2. upgrade模式重新升级至上一版本
* 手动回退指导
  
  * 手动重启虚拟机，进入启动项页面后，选择第二启动项进行回退，手动回退仅支持回退到上一个版本。
* 工具回退指导
  * 回退至任意版本
    * 修改 OS 的cr实例的YAML 配置文件（例如 upgrade_v1alpha1_os.yaml），设置相应字段为期望回退的老版本镜像信息。类别OS来自于安装和部署章节创建的CRD对象，字段说明及示例请见上一节升级指导。

    * YAML修改完成后执行更新命令，在集群中更新定制对象后，节点会根据配置的字段信息进行回退

        ```shell
        kubectl apply -f upgrade_v1alpha1_os.yaml
        ```

  * 回退至上一版本
    * 修改upgrade_v1alpha1_os.yaml，设置osversion为上一版本，opstype为rollback，回退至上一版本（即切换至上一分区）。YAML示例如下：

        ```yaml
        apiVersion: upgrade.openeuler.org/v1alpha1
        kind: OS
        metadata:
        name: os-sample
        spec:
            imagetype: ""
            opstype: rollback
            osversion: KubeOS pervious version
            maxunavailable: 2
            containerimage: ""
            evictpodforce: true/false
            imageurl: ""
            checksum: ""
            flagSafe: false
            mtls: true
        ```

    * 修改upgrade_v1alpha1_os.yaml，设置sysconfigs/upgradeconfigs的version为上一版本，回退至上一版本（已配置的参数无法回退）。YAML示例如下：

      ```yaml
      apiVersion: upgrade.openeuler.org/v1alpha1
      kind: OS
      metadata:
        name: os-sample
      spec:
        imagetype: ""
        opstype: config
        osversion: edit.os.version
        maxunavailable: edit.node.config.number
        containerimage: ""
        evictpodforce: true/false
        imageurl: ""
        checksum: ""
        flagSafe: false
        mtls: false
        sysconfigs:
            version: previous config version
            configs:
                - model: kernel.sysctl
                  contents:
                    - key: kernel param key1
                      value: kernel param value1
                    - key: kernel param key2
                      value: kernel param value2
                - model: kernel.sysctl.persist
                  configpath: persist file path
                  contents:
                    - key: kernel param key3
                      value: kernel param value3         
      ```

  * YAML修改完成后执行更新命令，在集群中更新定制对象后，节点会根据配置的字段信息进行回退

    ```shell
    kubectl apply -f upgrade_v1alpha1_os.yaml
    ```

    更新完成后，节点会根据配置信息回退容器 OS。
  * 查看节点容器 OS 版本(回退OS版本)或节点config版本&节点状态为idle(回退config版本)，确认回退是否成功。

    ```shell
    kubectl get osinstances -o custom-columns='NAME:.metadata.name,NODESTATUS:.spec.nodestatus,SYSCONFIG:status.sysconfigs.version,UPGRADECONFIG:status.upgradeconfigs.version'
    ```

## Admin容器镜像制作、部署和使用

KubeOS提供一个分离的包含sshd服务和hostshell工具的Admin容器，来帮助管理员在必要情况下登录KubeOS，其中的sshd服务由[sysmaster](https://gitee.com/openeuler/sysmaster)/systemd拉起。Admin容器部署后用户可通过ssh连接到节点的Admin容器，进入Admin容器后执行hostshell命令获取host的root shell。

### admin容器镜像制作

以sysmaster为例，根据系统版本和架构，获取对应的sysmaster RPM包，如获取openEuler-22.03-LTS-SP1-aarch64版本的[sysmaster](https://repo.openeuler.org/openEuler-22.03-LTS-SP1/update/aarch64/Packages/)到scripts/admin-container目录下。

修改admin-container目录下的Dockerfile，指定sysmaster RPM包的路径，其中的openeuler-22.03-lts-sp1可在[openEuler Repo](https://repo.openeuler.org/openEuler-22.03-LTS-SP1/docker_img)下载。

```Dockerfile
FROM openeuler-22.03-lts-sp1
RUN yum -y install openssh-clients util-linux
ADD ./your-sysmaster.rpm /home
RUN rpm -ivh  /home/your-sysmaster.rpm
COPY ./hostshell /usr/bin/
COPY ./set-ssh-pub-key.sh /usr/local/bin
COPY ./set-ssh-pub-key.service /usr/lib/sysmaster
EXPOSE 22
RUN sed -i 's/sysinit.target/sysinit.target;sshd.service;set-ssh-pub-key.service/g' /usr/lib/sysmaster/basic.target
CMD ["/usr/lib/sysmaster/init"]
```

在KubeOS目录下，编译hostshell二进制:

```shell
make hostshell
```

进入scripts目录，执行:

```shell
cd scripts
bash -x kbimg.sh create admin-image -f admin-container/Dockerfile -d your_imageRepository/admin_imageName:version
docker push your_imageRepository/admin_imageName:version
```

### admin容器部署

在master节点上部署Admin容器，需要提供ssh公钥来免密登录，修改并应用如下示例yaml文件:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: root-secret
data:
  ssh-pub-key: <your-ssh-pub-key-encoded-with-base64>
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: admin-container-sysmaster
  namespace: default
  labels:
    control-plane: admin-container-sysmaster
spec:
  selector:
    matchLabels:
      control-plane: admin-container-sysmaster
  replicas: 1
  template:
    metadata:
      labels:
        control-plane: admin-container-sysmaster
    spec:
      hostPID: true
      containers:
        - name: admin-container-sysmaster
          image: <your_imageRepository/admin_imageName:version>
          imagePullPolicy: Always
          securityContext:
            privileged: true
          ports:
            - containerPort: 22
          # sysmaster要求
          env:
            - name: container
              value: containerd
          volumeMounts:
            # name 必须与下面的卷名匹配
            - name: secret-volume
              # mountPath必须为/etc/secret-volume
              mountPath: /etc/secret-volume
              readOnly: true
      nodeName: <your-worker-node-name>
      volumes:
        - name: secret-volume
          secret:
            # secretName必须与上面指定的Secret的name相同
            secretName: root-secret
---
apiVersion: v1
kind: Service
metadata:
  name: admin-container-sysmaster
  namespace: default
spec:
  type: NodePort
  ports:
    - port: 22
      targetPort: 22
      nodePort: <your-exposed-port>
  selector:
    control-plane: admin-container-sysmaster
```

### admin容器使用

ssh到Admin容器，然后执行hostshell命令进入host root shell, 如：

```shell
ssh -p your-exposed-port root@your.worker.node.ip
hostshell
```

#### hostshell说明

为了保证KubeOS的轻便性，许多工具或命令没有安装在KubeOS内。因此，用户可以在制作Admin容器时，将期望使用的二进制文件放在容器内的如/usr/bin目录下。hostshell工具在执行时会将容器下的/usr/bin, /usr/sbin, /usr/local/bin, /usr/local/sbin路径添加到host root shell的环境变量。

## 常见问题及解决办法

1. 使用容器OS的虚拟机加入集群后相关pod启动失败，kubelet日志错误为"not found /etc/resolv.conf"
   解决方法：镜像制作时配置或者手动添加/etc/resolv.conf文件，内容与集群master节点上/etc/resolv.conf一致

## 附录

### Setting 列表

#### kernel Settings

* kenerl.sysctl：临时设置内核参数，重启后无效，key/value 表示内核参数的 key/value， key与value均不能为空且key不能包含“=”，该参数不支持删除操作（operation=delete）示例如下:

    ```yaml
    configs:
      - model: kernel.sysctl
        contents:
            - key: user.max_user_namespaces
              value: 16384
            - key: net.ipv4.tcp_tw_recycle
              value: 0
              operation: delete
    ```

* kernel.sysctl.persist: 设置持久化内核参数，key/value表示内核参数的key/value，key与value均不能为空且key不能包含“=”， configpath为配置文件路径，支持新建（需保证父目录存在），如不指定configpath默认修改/etc/sysctl.conf，示例如下：
    ```yaml
    configs:
      - model: kernel.sysctl.persist
        configpath : /etc/persist.conf
        contents:
            - key: user.max_user_namespaces
              value: 16384
            - key: net.ipv4.tcp_tw_recycle
              value: 0
              operation: delete
    ```

#### Grub配置

* grub.cmdline: 设置grub.cfg文件中的内核引导参数，该行参数在grub.cfg文件中类似如下示例：

  ```shell
  linux   /boot/vmlinuz root=/dev/sda2 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3
  ```

* KubeOS使用双分区，grub.cmdline支持对当前分区或下一分区进行配置：

  * grub.cmdline.current：对当前分区的启动项参数进行配置。
  * grub.cmdline.next：对下一分区的启动项参数进行配置。

* 注意：升级/回退前后的配置，始终基于升级/回退操作下发时的分区位置进行current/next的区分。假设当前分区为A分区，下发升级操作并在sysconfigs（升级重启后配置）中配置grub.cmdline.current，重启后进行配置时仍修改A分区对应的grub cmdline。

* grub.cmdline.current/next支持“key=value”（value不能为空），也支持单key。若value中有“=”，例如“root=UUID=some-uuid”，key应设置为第一个“=”前的所有字符，value为第一个“=”后的所有字符。 配置方法示例如下：

    ```yaml
    configs:
    - model: grub.cmdline.current
      contents:
          - key: selinux
            value: "0"
          - key: root
            value: UUID=e4f1b0a0-590e-4c5f-9d8a-3a2c7b8e2d94
          - key: panic
            value: "3"
            operation: delete
          - key: crash_kexec_post_notifiers
    - model: grub.cmdline.next
      contents:
          - key: selinux
            value: "0"
          - key: root
            value: UUID=e4f1b0a0-590e-4c5f-9d8a-3a2c7b8e2d94
          - key: panic
            value: "3"
            operation: delete
          - key: crash_kexec_post_notifiers
    ```
