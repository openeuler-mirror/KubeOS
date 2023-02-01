# KubeOS
## Introduction
KubeOS 是针对业务以容器的形式运行的场景，专门设计的一种轻量级操作系统。KubeOS 通过 kubernetes CRD + operator 扩展机制将将 OS 作为组件接入 kubernetes，使 OS 和业务处于同等地位，用户通过 kubernetes 集群统一管理节点上的容器和节点 OS，实现一套系统管理容器和 OS。
## Architecture
KubeOS架构的介绍请见： [architecture](docs/design/architecture.md)
## Getting Started
### Build from source and deploy
从源码构建指南请见： [quick-start.md](docs/quick-start.md).
### User Guide
用户指南请见：[user guide](https://docs.openeuler.org/zh/docs/22.03_LTS_SP1/docs/KubeOS/overview.html)
## How to Contribute
我们非常欢迎新贡献者加入到项目中来，也非常高兴能为新加入贡献者提供指导和帮助。您可以通过issue或者合入PR来贡献
## Licensing
KubeOS 使用 Mulan PSL v2.
