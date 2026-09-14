# KubeOS ISO 镜像手动构建模式（generate_only）设计方案

## 1. 背景与需求

KubeOS ISO 镜像的制作基于 [elemental-toolkit](https://github.com/rancher/elemental-toolkit)，由 kbimg 工具自动完成以下两步：

1. **构建 ISO 专用容器镜像**：以 OCI 基础镜像（`kubeos-oci`）为底，通过 Dockerfile 注入KubeOS对elemental适配必须的文件配置和elemental init 命令执行（`docker build`）；
2. **生成 ISO**：基于 elemental 的 `manifest.yaml` 配置，调用 `elemental build-iso` 产出可启动 ISO。

当前 kbimg 完全接管上述流程，`Dockerfile` 与 `manifest.yaml` 均由 kbimg 内部模板生成，构建命令参数在 kbimg 中硬编码。客户在实际使用中往往需要对以下内容进行定制：

- `Dockerfile`：如调整内核/驱动、修改 dracut 模块、改变系统裁剪；
- elemental `manifest.yaml`：如追加内核启动参数（`extra-cmdline`）、修改 grub entry、调整 squashfs 压缩算法等；
- elemental 自身的构建参数（`build-iso` 的 CLI flag）。

若由 KubeOS 为每种定制场景提供配置项，将导致配置体系过度膨胀且无法覆盖 elemental 的全部能力。因此需要一种机制，将 ISO 构建的**定制能力完整交还给客户**，同时尽量降低使用门槛。

## 2. 设计目标

- **保持默认行为不变**：未开启新特性时，ISO 制作流程与现有版本完全一致（一键自动构建）；
- **可定制**：客户可自由修改 Dockerfile、manifest.yaml 及 elemental 构建命令，覆盖全部自定义场景；
- **低门槛**：kbimg 仍然负责"准备好一切"（生成模板文件、准备好构建依赖），客户只需修改文件并执行构建；
- **最小侵入**：kbimg 代码改动量小，不改变其他镜像类型（OCI、VM、PXE、升级包等）的行为。

## 3. 方案总体设计

在 `kbimg.toml` 的 `[iso_img]` 配置节中新增布尔参数 `generate_only`（默认 `false`），提供两种 ISO 构建模式：

| 模式 | `generate_only` | 行为 |
| ---- | --------------- | ---- |
| 自动构建（默认） | `false` | 与现有版本一致：生成脚本后自动执行 `docker build` 与 `elemental build-iso` |
| 手动构建 | `true` | kbimg 只做准备工作（生成模板文件、拷贝构建依赖），**不执行任何构建命令**，由客户修改后自行构建 |

两种模式仅在 `[iso_img]` 上切换，其余配置（`oci_image`、`iso_image`、`elemental_cli_path`、`output_dir` 等）语义一致。

## 4. generate_only 模式详细设计

### 4.1 配置示例

```toml
[iso_img]
oci_image = "<registry>/kubeos-oci:v1"   # OCI 基础镜像
iso_image = "kubeos-iso:v2"              # ISO 专用镜像 tag（docker build 输出）
elemental_cli_path = "./elemental"       # elemental-cli 二进制路径
output_dir = "./output"                  # ISO 输出目录
generate_only = true                     # 手动构建模式
```

### 4.2 工作流程

执行 `./kbimg create -f kbimg.toml iso-img` 时，generate_only 模式下 kbimg 完成以下准备工作（不执行任何构建）：

1. 生成 `scripts-auto/iso/Dockerfile`：ISO 专用镜像构建文件（基础 OCI 镜像 + elemental init），作为客户定制的起点模板；
2. 生成 `scripts-auto/iso/manifest.yaml`：elemental `build-iso` 的配置（grub entry、内核启动参数、ISO 名称等），作为定制的起点模板；
3. 将 `elemental_cli_path` 指定的 elemental-cli 二进制**拷贝**到 `scripts-auto/iso/` 目录——`Dockerfile` 中的 `COPY elemental-cli /usr/bin/elemental` 依赖它，保证客户拿到目录即可直接 `docker build`；
4. 生成 `scripts-auto/iso/build-iso.sh`：手动构建辅助脚本，内含下述两条构建命令。

流程对比：

```
自动构建模式（generate_only=false）：
  kbimg → 生成 Dockerfile/manifest.yaml/kbimg.sh → 自动执行 docker build → 自动执行 elemental build-iso → 产出 ISO

手动构建模式（generate_only=true）：
  kbimg → 生成 Dockerfile/manifest.yaml/build-iso.sh + 拷贝 elemental-cli → 停止
  客户 → 修改 Dockerfile / manifest.yaml → 执行 bash build-iso.sh（或手动执行两条命令）→ 产出 ISO
```

### 4.3 产物目录

`scripts-auto/iso/` 目录内容：

```
scripts-auto/iso/
├── Dockerfile        # ISO 专用镜像构建文件（模板，可修改）
├── manifest.yaml     # elemental build-iso 配置（模板，可修改）
├── elemental-cli     # 已由 kbimg 拷入的 elemental-cli 二进制
└── build-iso.sh      # 手动构建辅助脚本（含两条构建命令）
```

### 4.4 手动构建命令

`build-iso.sh` 内含的两条命令（客户可参考手动执行）：

```bash
# 第 1 步：构建 ISO 专用镜像（基础 OCI 镜像 + elemental init）
docker build -t kubeos-iso:v2 -f scripts-auto/iso/Dockerfile scripts-auto/iso

# 第 2 步：使用 elemental 生成 ISO（--local 表示使用本地 docker 缓存中的镜像）
elemental --debug build-iso \
    --local \
    --config-dir scripts-auto/iso \
    -o ./output \
    docker:kubeos-iso:v2
```

### 4.5 代码实现要点

- `IsoImgInfo` 新增字段 `generate_only: bool`（serde 默认 `false`，向后兼容既有配置文件）；
- `prepare()`：generate_only 模式下跳过 ISO 构建所需的磁盘空间预检（不做构建），仍校验 elemental-cli 二进制存在（需要拷贝）；
- `generate_scripts()`：generate_only 模式下生成模板文件与辅助脚本后直接返回，不再生成自动执行脚本；
- 主流程：generate_only 模式下跳过脚本执行（复用 `--debug` 的"只生成不执行"机制）。

## 5. 使用示例

```bash
# 1. 配置 kbimg.toml，开启 generate_only
# 2. 生成构建产物
./kbimg create -f kbimg.toml iso-img

# 3. 按需修改
vi scripts-auto/iso/Dockerfile       # 如调整 dracut 模块、系统裁剪
vi scripts-auto/iso/manifest.yaml    # 如追加 extra-cmdline 内核参数、修改 grub entry

# 4. 手动构建（前置条件：oci_image 对应镜像已存在，如已执行 kbimg create -f kbimg.toml oci-img）
bash scripts-auto/iso/build-iso.sh

# 5. 产出
ls ./output/KubeOS-*.iso
```

## 6. 兼容性与影响面

- **默认行为完全不变**：`generate_only` 缺省为 `false`，现有客户的配置文件和构建流程无需任何改动；
- **仅影响 ISO 镜像类型**：不影响 OCI、VM、PXE、升级镜像等其他类型；
- **产物自包含**：`scripts-auto/iso/` 目录包含构建所需全部文件（含 elemental-cli），可整体拷贝到构建环境使用。

## 7. 注意事项与边界

- `--local` 要求 ISO 专用镜像已构建到本地 docker 缓存，即需先执行第 1 步 `docker build`；
- elemental-cli 版本建议与 KubeOS 验证过的版本保持一致，避免不同版本间 ISO layout 差异；
- 客户对 `Dockerfile` / `manifest.yaml` 的修改完全由客户负责，kbimg 不再校验其正确性；
- 手动构建模式下 kbimg 不执行任何容器/镜像构建操作，所需磁盘空间由客户自行保障。
