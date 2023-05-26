## Copyright (c) Huawei Technologies Co., Ltd. 2023. All rights reserved.
# KubeOS is licensed under the Mulan PSL v2.
# You can use this software according to the terms and conditions of the Mulan PSL v2.
# You may obtain a copy of Mulan PSL v2 at:
#     http://license.coscl.org.cn/MulanPSL2
# THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
# PURPOSE.
## See the Mulan PSL v2 for more details.

FROM openeuler-22.03-lts
MAINTAINER <shenyangyang4@huawei.com>

RUN yum -y install openssh-clients util-linux


ADD ./sysmaster-0.2.3-1.oe2203.aarch64.rpm /home
RUN rpm -ivh  /home/sysmaster-0.2.3-1.oe2203.aarch64.rpm

ADD ./hostshell /usr/bin/
EXPOSE 22
# set sshd.service pulled up by default
RUN sed -i 's/sysinit.target/sysinit.target;sshd.service/g' /usr/lib/sysmaster/basic.target

CMD ["/usr/lib/sysmaster/init"]
