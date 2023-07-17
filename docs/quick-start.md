# 快速使用指导

## 编译及部署

### 编译指导

* 编译环境：openEuler Linux x86/AArch64
* 进行编译需要以下包：
  * golang(大于等于1.15版本)
  * make
  * git

  ``` shell
  sudo yum install golang make git
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
  ```  

  * proxy及operator容器镜像构建
    * proxy及operator容器镜像构建使用docker，请先确保docker已经安装和配置完毕
    * 请用户自行编写Dockerfile来构建镜像，请注意
      * operator和proxy需要基于baseimage进行构建，用户保证baseimage的安全性
      * 需将operator和proxy拷贝到baseimage上
      * 请确保proxy属主和属组为root，文件权限为500
      * 请确保operator属主和属组为在容器内运行operator的用户，文件权限为500
      * operator和proxy的在容器内的位置和容器启动时运行的命令需与部署operator的yaml中指定的字段相对应
    * 首先指定镜像仓库地址、镜像名及版本，Dockerfile路径，然后构建并推送镜像到镜像仓库
    * Dockerfile参考如下, Dockerfile也可以使用多阶段构建:

      ``` dockerfile
      FROM your_baseimage
      COPY ./bin/proxy /proxy
      ENTRYPOINT ["/proxy"]
      FROM your_baseimage
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

* OS虚拟机镜像制作
  * 制作注意事项
    * 请确保已安装qemu-img，bc，parted，tar，yum，docker
    * 容器OS镜像制作需要使用root权限
    * 容器OS 镜像制作工具的 rpm 包源为 openEuler 具体版本的 everything 仓库和 EPOL 仓库。制作镜像时提供的 repo 文件中，yum 源建议同时配置 openEuler 具体版本的 everything 仓库和 EPOL 仓库
    * 容器OS镜像制作之前需要先将当前机器上的selinux关闭或者设为允许模式
    * 使用默认rpmlist进行容器OS镜像制作出来的镜像默认和制作工具保存在相同路径，该分区至少有25G的剩余空间
    * 容器镜像制作时不支持用户自定义配置挂载文件
    * 容器OS镜像制作工具执行异常中断，可能会残留文件、目录或挂载，需用户手动清理，对于可能残留的rootfs目录，该目录虽然权限为555，但容器OS镜像制作在开发环境进行，不会对生产环境产生影响。
    * 请确保os-agent属主和属组为root，建议os-agent文件权限为500
      * 容器OS虚拟机镜像制作
          进入scripts目录，执行脚本

          ```shell
          cd scripts
          bash kbimg.sh create vm-image -p xxx.repo -v v1 -b ../bin/os-agent -e '''$1$xyz$RdLyKTL32WEvK3lg8CXID0'''
          ```

        * 其中 xx.repo 为制作镜像所需要的 yum 源，yum 源建议配置为 openEuler 具体版本的 everything 仓库和 EPOL 仓库。
        * 容器 OS 镜像制作完成后，会在 scripts 目录下生成：
          * raw格式的系统镜像system.img，system.img大小默认为20G，支持的根文件系统分区大小<2020MiB，持久化分区<16GB。
          * qcow2 格式的系统镜像 system.qcow2。
          * 可用于升级的根文件系统分区镜像 update.img 。
        * 制作出来的容器 OS 虚拟机镜像目前只能用于 CPU 架构为 x86 和 AArch64 的虚拟机场景，x86 架构的虚拟机使用 legacy 启动模式启动需制作镜像时指定-l参数
        * 容器OS运行底噪<150M (不包含k8s组件及相关依赖kubernetes-kubeadm，kubernetes-kubelet， containernetworking-plugins，socat，conntrack-tools，ebtables，ethtool)
        * 本项目不提供容器OS镜像，仅提供裁剪工具，裁剪出来的容器OS内部的安全性由OS发行商保证。
  * 声明： os-agent使用本地unix socket进行通信，因此不会新增端口。下载镜像的时候会新增一个客户端的随机端口，1024~65535使用完后关闭。proxy和operator与api-server通信时作为客户端也会有一个随机端口，基于kubernetes的operator框架，必须使用端口。他们部署在容器里。

### 部署指导

* 环境要求
  * openEuler Linux x86/AArch64系统
  * Kubernetes集群已部署
  * 准备进行升级的Node节点的OS为使用上一节方式制作出来的容器OS

* 部署
  * 使用kubernetes的声明式API进行配置,部署CRD（CustomResourceDefinition），operator，proxy以及rbac机制的YAML需要用户自行编写
  * YAML举例说明模板参见本目录下example文件夹下的文件夹，你也可以将config文件夹拷贝到docs上一级目录，并进行简单的修改使用
  * 这些YAML配置文件，由K8s集群管理员加载，如果恶意在yaml文件里面写了病毒，K8s集群管理员如果放行，传到我们的处理模块我们也是没有办法校验的，此处有风险
  * operator和proxy部署在kubernets集群中，operator应部署为deployment，proxy应部署为damonset
  * 尽量部署好k8s的安全措施，如rbac机制，pod的service account和security policy配置等。
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

### 使用指导

* 注意事项
  * 容器OS升级为所有软件包原子升级，默认不在容器OS内提供单包升级能力。
  * 容器OS升级为双区升级的方式，不支持更多分区数量。
  * 单节点的升级过程的日志可在节点的/var/log/message文件查看。
  * 请严格按照提供的升级和回退流程进行操作，异常调用顺序可能会导致系统无法升级或回退。
  * 使用docker镜像升级和mtls双向认证仅支持 openEuler 22.09 及之后的版本
  * 不支持跨大版本升级

* 使用指导
  * 参数说明：在集群中创建类别为OS的定制对象，设置相应字段。类别OS来自于安装和部署章节创建的CRD对象，字段及说明如下：
    * imageurl指定的地址里包含协议，只支持http或https协议。imageurl为https协议时为安全传输，imageurl为http地址时，需指定flagSafe为true，即用户明确该地址为安全时，才会下载镜像。如imageurl为http地址且没有指定flagSafe为true，默认该地址不安全，不会下载镜像并且在升级节点的日志中提示用户该地址不安全
    * 对于imageurl，推荐使用https协议，使用https协议需要升级的机器已安装相应证书。如果镜像服务器由用户自己维护，需要用户自己进行签名，并保证升级节点已安装对应证书。用户需要将证书放在容器OS /etc/KubeOS/certs目录下。地址由管理员传入，管理员应该保证网址的安全性，推荐采用内网地址。
    * 容器OS镜像的合法性检查需要由容器OS镜像服务提供者做合法性检查，确保下载的容器OS镜像来源可靠

      | 参数            |参数类型  | 参数说明                                                     | 使用说明 | 是否必选         |
      | -------------- | ------ | ------------------------------------------------------------ | ----- | ---------------- |
      | imagetype      | string | 使用的升级镜像的类型           | 需为 docker ，containerd ，或者是 disk，其他值无效，且该参数仅在升级场景有效|是               |
      | opstype        | string | 进行的操作，升级,回退或者配置 | 需为 upgrade ，config 或者 rollback ，其他值无效 |是               |
      | osversion      | string | 用于升级或回退的镜像的OS版本          | 需为 KubeOS version , 例如: KubeOS 1.0.0|是               |
      | maxunavailable | int    | 同时进行升级或回退的节点数 | maxunavailable值设置为大于实际集群的节点数时也可正常部署，升级或回退时会按照集群内实际节点数进行|是               |
      | containerimage    | string | 用于升级的容器镜像               | 需要为容器镜像格式：repository/name:tag，仅在使用容器镜像升级场景下有效|是               |
      | imageurl       | string | 用于升级的磁盘镜像的地址 | imageurl中包含协议，只支持http或https协议，例如：<https://192.168.122.15/update.img> 仅在使用磁盘镜像升级场景下有效|是               |
      | checksum       | string | 用于升级的磁盘镜像校验的checksum(SHA-256)值或者是用于升级的容器镜像的digests值                      | 仅在升级场景下有效 |是               |
      | flagSafe       | bool   | 当imageurl的地址使用http协议表示是否是安全的                 | 需为 true 或者 false ，仅在imageurl使用http协议时有效 |是               |
      | mtls           | bool   | 用于表示与imageurl连接是否采用https双向认证     | 需为 true 或者 false ，仅在imageurl使用https协议时有效|是               |
      | cacert         | string | https或者https双向认证时使用的根证书文件                       | 仅在imageurl使用https协议时有效| imageurl使用https协议时必选 |
      | clientcert     | string | https双向认证时使用的客户端证书文件                          | 仅在使用https双向认证时有效|mtls为true时必选 |
      | clientkey      | string | https双向认证时使用的客户端公钥                              | 仅在使用https双向认证时有效|mtls为true时必选 |
      | evictpodforce      | bool | 用于表示升级/回退时是否强制驱逐pod                            | 需为 true 或者 false ，仅在升级或者回退时有效| 必选 |
      | sysconfigs      | / | 需要进行配置的参数值                            | 在配置或者升级或者回退机器时有效，在升级或者回退操作之后即机器重启之后起效，详细字段说明请见```配置（Settings）指导```| 可选 |
      | upgradeconfigs      | / | 需要升级前进行的配置的参数值                            | 在升级或者回退时有效，在升级或者回退操作之前起效，详细字段说明请见```配置（Settings）指导```| 可选 |  

  * 升级指导
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
            dockerimage: ""
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
            imageurl: ""
            checksum: container image digests
            flagSafe: false
            mtls: true
          ```

        * 升级并且进行配置的示例如下，
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
                dockerimage: ""
                imageurl: ""
                checksum: container image digests
                flagSafe: false
                mtls: false
                sysconfigs:
                    version: edit.os.version
                    configs:
                        - model: kernel.systcl
                        contents:
                            - key: kernel param key1
                              value: kernel param value1
                            - key: kernel param key2
                              value: kernel param value2
                        - model: kernel.systcl.persist
                          configpath: persist file path
                          contents:
                            - key: kernel param key3
                              value: kernel param value3
                            - key: ""
                              value: ""
                upgradeconfigs:
                    version: 1.0.0
                    configs:
                        - model: kernel.systcl
                        contents:
                            - key: kernel param key4
                              value: kernel param value4          
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

    * 如果后续需要再次升级，与上面相同对 upgrade_v1alpha1_os.yaml 的 imageurl, osversion, checksum, maxunavailable, flagSafe 或者containerimage字段进行相应修改。
  * 配置（Settings）指导
    * Settings参数说明:
      以进行配置时的示例yaml为例对配置的参数进行说明，示例yaml如下：

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
        dockerimage: ""
        imageurl: ""
        checksum: ""
        flagSafe: false
        mtls: false
        sysconfigs:
            version: 1.0.0
            configs:
                - model: kernel.systcl
                  contents:
                    - key: kernel param key1
                      value: kernel param value1
                    - key: kernel param key2
                      value: kernel param value2
                      operation: delete
                - model: kernel.systcl.persist
                  configpath: persist file path
                  contents:
                    - key: kernel param key3
                      value: kernel param value3         
      ```

      * 配置的参数说明如下：
        * version: 配置的版本,通过版本差异触发配置，请修改配置后更新 version
        * configs: 具体配置内容
          * model: 进行的配置的类型，支持的配置类型请看[Settings 列表](#setting-列表)
          * configpath: 如为持久化配置，配置文件路径
          * contents: 配置参数的 key / value 和对参数的操作。
            * key / value: 请看[Settings 列表](#setting-列表)对支持的配置的 key / value的说明。
            * operation: 若不指定operation，则默认为添加或更新。若指定为delete，代表删除目前OS中已配置的参数。
            **注意：** 当operation为delete时，yaml中的key/value必须和OS上想删除参数的key/value**一致**，否则删除失败。
      * upgradeconfigs与sysconfig参数相同，upgradeconfig为升级前进行的配置，仅在升级/回滚场景起效，在升级/回滚操作执行前进行配置，只进行配置或者需要升级/回滚重启后执行配置，使用sysconfigs
    * 使用说明
      * 编写YAML文件，在集群中部署 OS 的cr实例，用于部署cr实例的YAML示例如上，假定将上面的YAML保存到upgrade_v1alpha1_os.yaml
      * 查看配置之前的节点的配置的版本和节点状态（NODESTATUS状态为idle）

        ```shell
        kubectl get osinstances -o custom-columns='NAME:.metadata.name,NODESTATUS:.spec.nodestatus,SYSCONFIG:status.sysconfigs.version,UPGRADESYSCONFIG:status.upgradesysconfigs.version'
        ```

      * 执行命令，在集群中部署cr实例后，节点会根据配置的参数信息进行配置，再次查看节点状态(NODESTATUS变成config)

        ```shell
        kubectl apply -f upgrade_v1alpha1_os.yaml
        kubectl get osinstances -o custom-columns='NAME:.metadata.name,NODESTATUS:.spec.nodestatus,SYSCONFIG:status.sysconfigs.version,UPGRADESYSCONFIG:status.upgradesysconfigs.version'
        ```

      * 再次查看节点的配置的版本确认节点是否配置完成(NODESTATUS恢复为idle)

        ```shell
        kubectl get osinstances -o custom-columns='NAME:.metadata.name,NODESTATUS:.spec.nodestatus,SYSCONFIG:status.sysconfigs.version,UPGRADESYSCONFIG:status.upgradesysconfigs.version'
        ```

    * 如果后续需要再次升级，与上面相同对 upgrade_v1alpha1_os.yaml 的相应字段进行相应修改。
  * 回退指导
    * 回退场景
      * 虚拟机无法正常启动时，需要退回到上一可以启动的版本时进行回退操作，仅支持手动回退容器 OS 。
      * 虚拟机能够正常启动并且进入系统，需要将当前版本退回到老版本时进行回退操作，支持工具回退（类似升级方式）和手动回退，建议使用工具回退。
      * 配置出现错误，节点状态陷入config时，可以回退至上一个配置版本以恢复节点至idle状态。
      **注意**：在配置新版本时，出现错误前已经配置的参数无法回退。
    * 手动回退指导
      * 手动重启虚拟机，选择第二启动项进行回退，手动回退仅支持回退到本次升级之前的版本。
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
            dockerimage: ""
            imageurl: ""
            checksum: ""
            flagSafe: false
            mtls: false
            sysconfigs:
                version: previous config version
                configs:
                    - model: kernel.systcl
                      contents:
                        - key: kernel param key1
                          value: kernel param value1
                        - key: kernel param key2
                          value: kernel param value2
                    - model: kernel.systcl.persist
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
        kubectl get nodes -o custom-columns='NAME:.metadata.name,OS:.status.nodeInfo.osImage'

        kubectl get osinstances -o custom-columns='NAME:.metadata.name,NODESTATUS:.spec.nodestatus,SYSCONFIG:status.sysconfigs.version,UPGRADESYSCONFIG:status.upgradesysconfigs.version'
        ```

# 常见问题及解决办法

1. 使用容器OS的虚拟机加入集群后相关pod启动失败，kubelet日志错误为"not found /etc/resolv.conf"
   解决方法：镜像制作时配置或者手动添加/etc/resolv.conf文件，内容与集群master节点上/etc/resolv.conf一致

# 附录

## Setting 列表

### kernel Settings

* kenerl.sysctl: 设置内核参数，key/value 表示内核参数的 key/value, 示例如下:

    ```yaml
    configs:
      - model: kernel.systcl
        contents:
            - key: user.max_user_namespaces
              value: 16384
            - key: net.ipv4.tcp_tw_recycle
              value: 0
              operation: delete
    ```

* kernel.sysctl.persist: 设置持久化内核参数，key/value 表示内核参数的 key/value, configpath为配置修改/新建的文件路径，如不指定configpath默认修改/etc/sysctl.conf

    ```yaml
    configs:
      - model: kernel.systcl.persist
        configpath : /etc/persist.conf
        contents:
            - key: user.max_user_namespaces
              value: 16384
            - key: net.ipv4.tcp_tw_recycle
              value: 0
              operation: delete
    ```

### Grub配置

* grub.cmdline: 设置grub.cfg文件中的内核引导参数，该行参数在grub.cfg文件中类似如下示例：

  ```shell
      linux   /boot/vmlinuz root=UUID=5b1aaf5d-5b25-4e4b-a0d3-3d4c8d2e6a6e ro consoleblank=600 console=tty0 console=ttyS0,115200n8 selinux=1 panic=3
  ```  

  key/value 表示如上示例中内核引导参数的 key=value。
  **注意：** 当该参数有多个等号，如root=UUID=some-uuid时，配置时的key为第一个等号前的所有字符，value为第一个等号后的所有字符。
  配置方法示例如下：

    ```yaml
    configs:
      - model: grub.cmdline
        contents:
            - key: selinux
              value: 0
            - key: root
              value: UUID=e4f1b0a0-590e-4c5f-9d8a-3a2c7b8e2d94
            - key: panic
              value: 3
              operation: delete
    ```
