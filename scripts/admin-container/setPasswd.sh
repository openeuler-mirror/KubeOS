#!/bin/bash
## Copyright (c) Huawei Technologies Co., Ltd. 2023. All rights reserved.
# KubeOS is licensed under the Mulan PSL v2.
# You can use this software according to the terms and conditions of the Mulan PSL v2.
# You may obtain a copy of Mulan PSL v2 at:
#     http://license.coscl.org.cn/MulanPSL2
# THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
# PURPOSE.
## See the Mulan PSL v2 for more details.

passwd=$(cat /etc/secret-volume/password)
str=`sed -n '/^root:/p' /etc/shadow | awk -F "root:" '{print $2}'`
umask 0666
mv /etc/shadow /etc/shadow_bak
sed -i '/^root:/d' /etc/shadow_bak
echo "root:"${passwd}":"${str#*:} > /etc/shadow
cat /etc/shadow_bak >> /etc/shadow
rm -rf /etc/shadow_bak