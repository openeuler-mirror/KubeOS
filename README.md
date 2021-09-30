# KubeOS
KubeOS is an OS fully designed for Cloud Native environment. It simplifies 
OS updating by utilizing an operator in kubernetes cluster. The operator 
updates the whole OS as an entirety in the form of image instead 
of software packages. So workload and system can be managed in the 
same way which reduces the complexity of updating. Operator manages OS like 
deployments in kubernetes, including rolling update.

## Build from source
Please see [quick-tart.md](docs/quick-start.md).

## Deploy
Please see [quick-start.md](docs/quick-start.md) first and must be very careful about RBAC when deploying in production. KubeOS will let kubernetes 
to manage node updates and reboots, so use at your own risk.

## How to Contribute
We always welcome new contributors. We are happy to provide guidance for the new 
contributors. You can contribute via issues and merge requests.

## Licensing
KubeOS is licensed under the Mulan PSL v2.
