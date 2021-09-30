# 快速使用指导
## 编译及部署
### 编译指导
* 编译环境：openEuler Linux x86
* 进行编译需要以下包：      
    * golang(大于等于1.15版本)      
    * make      
    * git
    ``` shell script
    sudo yum install golang make git
    ```  
* 使用git获取本项目的源码
* 编译二进制
    * operator：负责控制集群的升级
    * proxy：负责集群和agent通信，从k8s传递升级指令给agent，从agent传递升级状态给k8s
    * os-agent：负责节点升级
     ```shell script
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
    ```shell script
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
* OS镜像制作
    - 制作注意事项
        - 请确保已安装qemu-img，bc，parted，tar，yum，docker
        - 容器OS镜像制作需要使用root权限
        - 容器OS镜像制作工具的rpm包源为openEuler的全量（everything）ISO
        - 容器OS镜像制作之前需要先将当前机器上的selinux关闭或者设为允许模式
        - 使用默认rpmlist进行容器OS镜像制作出来的镜像默认和制作工具保存在相同路径，该分区至少有25G的剩余空间 
        - 容器镜像制作时不支持用户自定义配置挂载文件
        - 容器OS镜像制作工具执行异常中断，可能会残留文件、目录或挂载，需用户手动清理，对于可能残留的rootfs目录，该目录虽然权限为555，但容器OS镜像制作在开发环境进行，不会对生产环境产生影响。 
        - 请确保os-agent属主和属组为root，建议os-agent文件权限为500
    * 容器OS镜像制作        
    进入scripts目录，执行脚本
        ```
        cd scripts
        bash generate.sh ISO_PATH VERSION AGENT_PATH ENCRYPTED_PASSWD
        ```
    - 参数说明：
      - ISO_PATH ：全量iso的路径
      - VERSION ：制作的容器OS镜像的版本
      - AGENT_PATH：构建出来的os-agent的路径
      - ENCRYPTED_PASSWD：镜像的root用户密码，加密后的带盐值的密码。可以用openssl、kiwi等命令生成
    - 容器OS镜像说明：
      - 容器OS镜像制作成功后，在/opt/kubeOS/scripts目录下会生成：
        - raw格式的系统镜像system.img，system.img大小默认为20G，支持的根文件系统分区大小<2020MiB，持久化分区<16GB。 
        - qcow2格式的系统镜像system.qcow2
        - 可用于升级的根文件系统分区镜像update.img。
      - 制作出来的容器OS镜像目前只能用于虚拟机场景，仅支持X86架构，准备进行升级的虚拟机需要为上面制作出来的容器OS镜像启动的虚拟机，如不是，请用system.qcow2重新部署虚拟机，虚拟机部署请见《openEuler 虚拟机用户指南》
      - 容器OS运行底噪<150M (不包含k8s组件及相关依赖kubernetes-kubeadm，kubernetes-kubelet， containernetworking-plugins，socat，conntrack-tools，ebtables，ethtool)
      - 本项目不提供容器OS镜像，仅提供裁剪工具，裁剪出来的容器OS内部的安全性由OS发行商保证。
	 声明： os-agent使用本地unix socket进行通信，因此不会新增端口。下载镜像的时候会新增一个客户端的随机端口，1024~65535使用完后关闭。proxy和operator与api-server通信时作为客户端也会有一个随机端口，基于kubernetes的operator框架，必须使用端口。他们部署在容器里。

### 部署指导
- 环境要求
    - openEuler Linux x86系统
    - Kubernetes集群已部署
    - 准备进行升级的Node节点的OS为使用上一节方式制作出来的容器OS
- 部署    
    - 使用kubernetes的声明式API进行配置,部署CRD（CustomResourceDefinition），operator，proxy以及rbac机制的YAML需要用户自行编写
	- YAML举例说明模板参见本目录下example文件夹下的config文件夹，你也可以将config文件夹拷贝到docs上一级目录，并进行简单的修改使用
    - 这些YAML配置文件，由K8s集群管理员加载，如果恶意在yaml文件里面写了病毒，K8s集群管理员如果放行，传到我们的处理模块我们也是没有办法校验的，此处有风险
    - operator和proxy部署在kubernets集群中，operator应部署为deployment，proxy应部署为damonset
    - 尽量部署好k8s的安全措施，如rbac机制，pod的service account和security policy配置等。
    - 假定您已经编辑好了YAML，并且CRD，rbac机制，operator和proxy的YAML分别放在了当前目录下config/crd，config/rbac目录下和config/manager目录下，执行部署命令：
    ```
    kubectl apply -f confg/crd
    kubectl apply -f config/rbac 
    kubectl apply -f config/manager
    ```
    - 部署完成后通过以下命令行查看各个组件是否都正常启动,如果所有组件的STATUS都是 Running的，说明组件都正常启动了。
    ```
    kubectl get pods -A
    ```
## 使用指导
### 注意事项
1. 容器OS升级为所有软件包原子升级，默认不在容器OS内提供单包升级能力。
2. 容器OS升级为双区升级的方式，不支持更多分区数量。
3. 单节点的升级过程的日志可在节点的/var/log/message文件查看。
4. 请严格按照提供的升级和回退流程进行操作，异常调用顺序可能会导致系统无法升级或回退。
### 升级指导
在集群中创建类别为OS的定制对象，设置imageurl ，osversion，checksum和maxunavailable字段。类别OS来自于安装和部署章节创建的CRD对象，字段说明如下：
- imageurl：用于升级的镜像的地址，这个地址里包含协议，只支持http或https协议。
- osversion：用于升级的镜像的OS版本
- checksum：用于升级的镜像的checksum(SHA-256)值
- maxunavailable：同时进行升级的节点数，maxunavailable值设置为大于实际集群的节点数时也可正常部署，升级时会按照集群内实际节点数进行升级
- flagSafe：用于表示imageurl的地址是否是安全的        

容器OS升级保证默认安全，用户指定的imageurl为http地址时，需显示指定flagSafe为true，即用户明确该地址为安全时，才会下载镜像。如imageurl为http地址且没有指定flagSafe为true，默认该地址不安全，不会下载镜像并且提示用户该地址不安全      

对于imageurl，推荐使用https协议，使用https协议请确保升级的虚拟机已安装相应证书。如果镜像服务器由用户自己维护，需要用户自己进行签名，并保证升级节点已安装对应证书。用户将证书放在容器OS /etc/pki/ca-trust/source/anchors目录下，然后使用update-ca-trust extract 命令安装证书。地址由管理员传入，管理员应该保证网址的安全性，推荐采用内网地址。        

容器OS镜像的合法性检查需要由容器OS镜像服务提供者做合法性检查，确保下载的容器OS镜像来源可靠

用于创建定制对象的YAML示例如下，config/samples目录下的yaml仅供参考：
```
apiVersion: upgrade.openeuler.org/v1alpha1
kind: OS
metadata:
  name: os-sample
spec:
  osversion: edit.os.version
  imageurl: edit.image.url
  maxunavailable: edit.node.upgrade.number
  checksum:image.checksum
  flagSafe:imageurl.safety
```
假定将上面的YAML保存到config/samples/upgrade_v1alpha1_os.yaml

执行创建命令，在集群中创建定制对象后，节点会根据配置的字段信息进行升级。
```
kubectl apply -f config/samples/upgrade_v1alpha1_os.yaml
``` 
可以查看节点的OS版本来查看节点是否升级完成
```
kubectl get nodes -o custom-columns='NAME:.metadata.name,OS:.status.nodeInfo.osImage'
```
使用kubernetes的声明式API进行配置，如后续再次升级，与上面相同，对upgrade_v1alpha1_os.yaml的imageurl ，osversion，checksum和maxunavailable字段进行相应修改，执行如下命令：
```
kubectl apply -f config/samples/upgrade_v1alpha1_os.yaml
```
### 回退指导
#### 虚拟机无法正常启动
* 如果虚拟机不能正常启动的情况下，只能手动重启机器，选择第二启动项进行回退，手动回退仅支持回退到本次升级之前的版本
#### 虚拟机可以正常启动
* 如果虚拟机可以正常启动并进入系统，除了上面的手动回退之外，可以通过与升级类似的方式进行回退。      
* 修改upgrade_v1alpha1_os.yaml，设置imageurl ，osversion和checksum字段为期望回退的老版本镜像信息。类别OS来自于安装和部署章节创建的CRD对象，字段说明及示例请见上一节升级指导。     
* YAML修改完成后执行更新命令，在集群中更新定制对象后，节点会根据配置的字段信息进行回退
```
kubectl apply -f upgrade_v1alpha1_os.yaml
```
可以查看节点的OS版本来查看节点是否升级回退
```
kubectl get nodes -o custom-columns='NAME:.metadata.name,OS:.status.nodeInfo.osImage'
```
# 常见问题及解决办法
1. 使用容器OS的虚拟机加入集群后相关pod启动失败，kubelet日志错误为"not found /etc/resolv.conf"        
   解决方法：手动添加/etc/resolv.conf文件，内容与集群master节点上/etc/resolv.conf一致

   


