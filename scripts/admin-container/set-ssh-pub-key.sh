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

ssh_pub=$(cat /etc/secret-volume/ssh-pub-key)
ssh_dir="/root/.ssh"
authorized_file="$ssh_dir/authorized_keys"

if [ ! -d "$ssh_dir" ]; then
    mkdir "$ssh_dir"
    chmod 700 "$ssh_dir"
fi

if [ ! -f "$authorized_file" ]; then
    touch "$authorized_file"
    chmod 600 "$authorized_file"
fi

echo "$ssh_pub" >> "$authorized_file"
