## dm-verity功能介绍

KubeOS基于[dm-verity](https://www.kernel.org/doc/html/latest/admin-guide/device-mapper/verity.html)提供对根文件系统rootfs的完整性保护。Dm-verity对目标设备rootfs分成固定大小(4096)的块，每块分别计算hash得到第一层hash。第一层hash按照固定大小的块再次计算hash形成第二层。如此迭代，形成一棵hash树，最终得到roothash。示意如下
```
0层：rootfs                block0   ...    blockx   ...   blockx      .....       blockn
                              |               |              |          |            |
1层：对0层计算hash          h1.0    ...     h1.x    ...    h1.x       .....        h1.n
                              \______________/      \__________________/ \__________/
                                     |                       |                 |
2层：对1层计算hash                  h2.0                    h2.x              h2.n
                                     \ .............              ............/

逐层计算hash                                 \ .......            ..... /
                                                  \...           .../
                                                      \         /
root层                                                 roothash

```
hash树除roothash外的中间节点hash作为元数据验证rootfs的完整性，验证时重新计算roothash，并与存储的初始roothash进行比对，一致则rootfs完整。因此，dm-verity的关键在于保证roothash的完整性。

## dm-verity当前实现

KubeOS当前基于dm-verity+安全启动实现对rootfs的完整性保护，安全启动用于保护roothash完整性。由于安全启动只支持UEFI模式启动，因此当前只支持UEFI启动的场景，不支持legacy启动场景。

安全启动基于密码学签名机制实现信任传递。通过在BIOS中引入可信证书，BIOS验证shim的签名，shim验证grub签名，grub验证kernel签名，签名验证失败则启动失败，由此完成系统启动，保证启动内核完整性。为保护dm-verity的roothash的完整，此处扩展安全启动功能，通过在grub中导入可信公钥，实现grub对initramfs、grub.cfg的签名验证，roothash在制作镜像时写入grub.cfg，系统启动时从grub.cfg获取roothash作为对比基线，借助dm-verity实现对rootfs的完整性校验。制作镜像时，安全启动的根信任证书保存在boot分区EFI目录下，名称为`rsa4BIOS.der`，首次启动时需要将该证书导入UEFI固件中**PK Options**和**DB Options**，参考如下安全启动设置。

安全启动需要用户自行生成证书及相关签名密钥，并设置口令保护签名私钥。此处不对口令做复杂性校验，建议包含大小写字母、数字等。主要涉及的口令有以下三个：
* BIOS签名私钥口令（pesign签名数据库口令）：明文口令，用于安全启动中保护签名私钥的安全性。此签名私钥存于镜像制作服务器上，用于对shim、grub进行签名。每次制作镜像需要输入该口令，否则无法签名。
* grub配置文件签名私钥口令：明文口令，用于保护grub配置文件签名私钥。grub中导入公钥，对应私钥存于镜像制作服务器上，用于对配置文件grub.cfg签名，签名私钥由该口令保护，每次制作镜像需要输入。
* grub shell口令：明文口令，如果在镜像启动时要进入grub shell则需要输入该口令。

dm-verity+安全启动采用双boot+root，此时磁盘分区对应如下：
```
      ----------------
part1 |     boot1    |
      ----------------
part2 |     root1    |      ---- Grub menuentry 'A'
      ----------------
part3 |     hash1    |
      ----------------
part4 |     boot2    |
      ----------------
part5 |     root2    |      ---- Grub menuentry 'B'
      ----------------
part6 |     hash2    |
      ----------------
part7 |    persist   |
      ----------------
```
其中boot分区包含启动文件，ROOT分区存放rootfs，hash分区存放hash元数据，用于验证root分区的完整性。

当前实现说明如下：

* dm-verity开启后不支持grub阶段加载mod，因此grub.cfg中`insmod xx`命令删除，使用dm-verity/grub.cfg文件替换
* dm-verity+安全启动模式下，grub.cfg文件不支持修改。如需修改需要在镜像制作服务器上修改并重新签名
* 当前dm-verity/grub.cfg中，menuentry选项默认设置第一个disk, virtio类型设备：`set root='hd0,gpt2'`, `root=/dev/vda2` or `root=/dev/vda5`
* 开启dm-verity，用户可以配置`ESP/EFI/openEuler/grubenv`文件，实现部分配置grub环境变量（白名单形式），如启动项（从哪一个root启动）
* 开启dm-verity，rootfs以只读方式挂载`/dev/mapper/kubeos-root`。当前dm-verity通过veritysetup工具实现，可以通过如下命令查看rootfs完整性状态
```
veritysetup status kubeos-root # 显示状态（verified）、目标数据设备、hash元数据设备、roothash

veritysetup verify /dev/vda2 /dev/vda3 roothash --debug # status显示的数据设备、hash设备、roothash，验证成功Command successful
```
* 如果当前rootfs（如root1）验证失败，尝试从另一个rootfs（root2）启动，若均验证失败，则系统启动失败
* 安全启动证书基于rsa签名，制作镜像时通过openssl生成自签名证书`rsa4BIOS.der`
* 安全启动可以通过mokutil工具查看，`mokutil --sb`
* 镜像制作服务器需要安装如下包
```
yum install -y pesign nss openssl veritysetup crypto-policies
```
* **密钥管理** 当前第一次开启dm-verity功能，镜像制作服务器会自动生成相关证书和密钥文件，默认位置为`my/path/to/KubeOS/scripts/dm-verity/keys`，密钥均由口令保护。不建议频繁更换密钥、证书文件，否则系统可能无法正常启动（签名验证失败）。密钥、证书生成过程见**附录**。
  - 安全启动依赖证书`rsa4BIOS.der`，每次更新需要进入BIOS重新导入证书（导入DB Options），否则系统无法启动，不建议频繁更新此证书。可选地，安全启动可以通过BIOS直接关闭
  - roothash完整性验证依赖`grub配置文件签名公钥`，该公钥在镜像制作时导入grub，对应私钥用于grub.cfg签名，为防止系统启动失败，不建议频繁更换。此处验签功能可以通过进入grub-shell（需要上述grub shell 口令）进行关闭，输入`set check_signatures=no`, `configfile (hd0,1 or 4)/EFI/openEuler/grub.cfg`进入系统

## 安全启动配置


KubeOS支持在**虚拟机**镜像制作时开启dm-verity+安全启动配置。以下介绍以HOST侧操作系统为openEuler系统为例，介绍KubeOS虚机安全启动配置步骤，参考[openEuler虚机安全启动介绍](https://docs.openeuler.org/zh/docs/24.03_LTS/docs/Virtualization/%E7%AE%A1%E7%90%86%E8%99%9A%E6%8B%9F%E6%9C%BA.html)。对于其他操作系统，参考相应安全启动配置（配置文件名称、路径、依赖等有差异）。

**xml文件修改**

虚拟机安全启动依赖于UEFI BIOS的实现，HOST侧需要安装edk2。 以aarch64为例，需安装`yum install -y edk2-aarch64`， edk2 rpm包中的组件安装于/usr/share/edk2/aarch64目录下，包括QEMU_EFI-pflash.raw和vars-template-pflash.raw。虚拟机启动UEFI BIOS部分xml配置如下：
```
<os>
    <type arch='aarch64' machine='virt'>hvm</type>
    <loader readonly='yes' type='pflash'>/usr/share/edk2/aarch64/QEMU_EFI-pflash.raw</loader>
    <nvram template='/usr/share/edk2/aarch64/vars-template-pflash.raw'>/path/to/QEMU-VARS.fd</nvram>
</os>
```
其中/usr/share/edk2/aarch64/QEMU_EFI-pflash.raw为UEFI BIOS镜像路径。/usr/share/edk2/aarch64/vars-template-pflash.raw为nvram镜像模板路径，/path/to/QEMU-VARS.fd为当前虚拟机nvram镜像文件路径，用于保存UEFI BIOS系统中的环境变量。

X86架构略有差异，需安装`yum install edk2-ovmf`，xml示例如下
```
<os>
    <type arch='x86_64' machine='pc-q35-6.2'>hvm</type>
    <loader type='pflash'>/usr/share/edk2/ovmf/OVMF_CODE.fd</loader>
    <nvram template='/usr/share/edk2/ovmf/OVMF_VARS.fd'>/path/to/OVMF_VARS.fd</nvram>
</os>
```

**BIOS导入证书文件**

当前实现中，制作KubeOS镜像时通过openssl生成自签名证书`rsa4BIOS.der`，证书文件存在BOOT分区EFI目录下。
虚拟机启动后，点击`F2`进入BIOS界面，配置路径如下
```
Device Manager
      -> Secure Boot Configuration
            -> Secure Boot Mode
                  -> Custom Mode
                        -> Custom Secure Boot Option
                              -> PK Options
                                    -> Enroll PK
                                          -> Enroll PK Using File
                                                -> BOOT / EFI / rsa4BIOS.der
                              -> DB Options
                                    -> Enroll Signature
                                          -> Enroll Signature Using File
                                                -> BOOT / EFI / rsa4BIOS.der
```
证书导入完成点击`F10`保存修改，执行`reset`，完成系统重置。

## dm-verity升级&回滚

开启dm-verity功能，升级通过`dd`命令将升级镜像导入对应分区。注意事项如下：

* 升级前后root分区的LABEL相同，均为`ROOT-A`
* 升级失败回滚
  - boot分区故障，无需手动操作，自动尝试另一个boot分区进行引导
  - root分区故障，需要手动切换另一个root分区，假如升级到A失败需要回滚到B，则手动选择从B启动，系统会重启2次：第一次手动选取B，第二次无需手动操作，系统自动选取B完成回滚

## 附录: 生成密钥/证书

KubeOS提供密钥、证书生成脚本支持用户生成自定义密钥、证书文件。当前实现基于RSA密码算法，国密SM算法支持可参考[这里](https://docs.openeuler.org/zh/docs/23.03/docs/ShangMi/%E5%AE%89%E5%85%A8%E5%90%AF%E5%8A%A8.html)，密钥生成过程如下：

```
    # 准备密钥目录
    KEYDIR="my/path/to/keys"
    CERTDB="$KEYDIR/certdb"
    BIOSkeyname="rsa4BIOS"
    PIN_PASSWORD="foo"
    keyname="$BIOSkeyname"

    # 生成RSA密钥、证书，其中PIN_PASSWORD为BIOS签名私钥口令（pesign签名数据库口令）
    mkdir -p "${CERTDB}"
    cat > "${KEYDIR}/pinfile" << EOF
$PIN_PASSWORD
EOF

    openssl genrsa -out "${KEYDIR}/${keyname}.key" 4096
    openssl req -new -key "${KEYDIR}/${keyname}.key" -out "${KEYDIR}/${keyname}.csr" -subj '/C=AA/ST=BB/O=CC/OU=DD/CN=BIOS-cert-for-kubeos-secure-boot'
    openssl x509 -req -days 365 -in "${KEYDIR}/${keyname}.csr" -signkey "${KEYDIR}/${keyname}.key" -out "${KEYDIR}/${keyname}.crt"
    openssl x509 -in "${KEYDIR}/${keyname}.crt" -out "${KEYDIR}/${keyname}.der" -outform der

    # 创建pesign签名数据库
    certutil -N -d "${CERTDB}" -f "${KEYDIR}/pinfile"
    certutil -A -n ${keyname} -d "${CERTDB}" -t CT,CT,CT -i "${KEYDIR}/${keyname}.crt" -f "${KEYDIR}/pinfile"
    openssl pkcs12 -export -out "${KEYDIR}/${keyname}.p12" -inkey "${KEYDIR}/${keyname}.key" -in "${KEYDIR}/${keyname}.crt" -password pass:"${PIN_PASSWORD}"
    pk12util -d "${CERTDB}" -i "${KEYDIR}/${keyname}.p12" -w "${KEYDIR}/pinfile" -k "${KEYDIR}/pinfile"

    # 签名示例，对shimx64.efi签名
    SHIM="my/path/to/shimx64.efi"
    pesign -n "${CERTDB}" -c ${BIOSkeyname} --pinfile "${KEYDIR}/pinfile" -s -i "$SHIM" -o "${SHIM}.signed"


    # 生成GPG签名密钥，用于对配置文件grub.cfg签名，其中GPG_PASSWORD为grub配置文件签名密钥保护口令
    GPG_PASSWORD="foo"
    GPGkeyid="gpgKey4kubeos"
    cat > "${KEYDIR}/gpg.batch.file" << EOF
Key-Type: RSA
Key-Length: 4096
Subkey-Type: RSA
Subkey-Length: 4096
Name-Real: ${GPGkeyid}
Expire-Date: 0
Passphrase: ${GPG_PASSWORD}
EOF

    gpg --batch --gen-key "${KEYDIR}/gpg.batch.file"
    gpg --list-keys --keyid-format LONG ${GPGkeyid} | grep pub > "${KEYDIR}/gpg.log"
    GPG_KEY=$(gpg --list-keys --keyid-format LONG ${GPGkeyid} | grep pub | awk -F 'rsa4096/' '{print $2}' | cut -b 1-16)
    gpg --export "$GPG_KEY" > "${KEYDIR}/gpg.key"

    # 签名示例
    GRUB_CFG="my/path/to/grub.cfg"
    gpg --pinentry-mode=loopback --passphrase "${GPG_PASSWORD}" --default-key "$GPG_KEY" --detach-sign "${GRUB_CFG}"
```
注意：密钥/证书文件生成后应及时删除口令和私钥文件