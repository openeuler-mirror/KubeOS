# kbosctl 使用指导

kbosctl 是 KubeOS 的运维命令行工具，通过 Unix Socket 与 os-agent 通信，用于在单节点上执行**升级**和**回滚**操作。

## 命令介绍

```text
Usage: kbosctl <COMMAND>

Commands:
  upgrade   Upgrade KubeOS to a new version via OCI image
  rollback  Rollback KubeOS to the previous version
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## 前置条件

* kbosctl 需要在 KubeOS 节点上以 root 权限执行
* 节点上的 os-agent 服务需正常运行（Unix Socket 位于 `/run/os-agent/os-agent.sock`）
* 升级需要节点能够访问容器镜像仓库（目标 OCI 镜像所在）
* 升级和回滚需要挂载在`/persist`目录的磁盘有足够的剩余空间

## kbosctl upgrade — 升级

使用 skopeo 从容器镜像仓库拉取 KubeOS OCI 镜像并升级到非活跃分区。

```text
Usage: kbosctl upgrade [OPTIONS] --os-image <OS_IMAGE>

Options:
      --os-image <OS_IMAGE>          OCI 镜像地址（skopeo transport 格式），例如 docker://192.168.1.100:5000/kubeos-oci:v1
      --config <SRC> <DST>           注入配置文件（可多次指定），SRC 为本地文件或 URL，DST 为升级后 rootfs 内目标路径
      --skip-tls                     从 URL 下载配置文件时跳过 TLS 证书校验
      --reboot                       升级完成后重启
  -h, --help                         Print help
```

### 使用示例

* 升级到新版本（不重启）

  ```bash
  kbosctl upgrade --os-image docker://192.168.1.100:5000/kubeos-oci:v1.0.1
  ```

* 升级并重启

  ```bash
  kbosctl upgrade --os-image docker://192.168.1.100:5000/kubeos-oci:v1.0.1 --reboot
  ```

* 升级并注入 cloud-init 配置

  ```bash
  kbosctl upgrade \
    --os-image docker://192.168.1.100:5000/kubeos-oci:v1.0.1 \
    --config https://example.com/user-data /etc/cloud/cloud.cfg.d/99_kubeos.cfg \
    --config /path/to/config.ign /boot/efi/ignition/config.ign \
    --skip-tls \
    --reboot
  ```

* 结果说明
  * 升级完成后，新系统写入非活跃分区（ROOT-A 或 ROOT-B）
  * 若指定 `--reboot`，系统重启后进入新版本；否则需手动重启生效
  * 升级配置的 cloud-init/ignition 等文件会注入到新系统对应路径

## kbosctl rollback — 回滚

回滚到上一个版本（切换引导分区到另一个分区）。

```text
Usage: kbosctl rollback [OPTIONS]

Options:
      --reboot  回滚完成后重启
  -h, --help    Print help
```

### 使用示例

* 回滚到上一个版本（不重启）

  ```bash
  kbosctl rollback
  ```

* 回滚并重启

  ```bash
  kbosctl rollback --reboot
  ```

* 结果说明
  * 回滚将 GRUB 引导切换到另一个分区
  * 若指定 `--reboot`，系统重启后进入上一个版本；否则需手动重启生效

## 注意事项

* 升级/回滚过程中不支持并发执行，需等待当前操作完成
* 升级会将新系统写入非活跃分区，原活跃分区不受影响，回滚可回到原版本
* 升级时若配置了 `--config` 注入且来源为 URL，可通过 `--skip-tls` 跳过自签名证书校验
