#! /bin/bash
## Copyright (c) Huawei Technologies Co., Ltd. 2026. All rights reserved.
 # KubeOS is licensed under the Mulan PSL v2.
 # You can use this software according to the terms and conditions of the Mulan PSL v2.
 # You may obtain a copy of Mulan PSL v2 at:
 #     http://license.coscl.org.cn/MulanPSL2
 # THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 # IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 # PURPOSE.
## See the Mulan PSL v2 for more details.

depends() {
    return 0
}

install() {
    inst_simple "$moddir/persist-mount.service" \
        "$systemdsystemunitdir/persist-mount.service"
    systemctl -q --root="$initdir" enable persist-mount.service
}

installkernel() {
    hostonly='' instmods ext4 overlay =fs/nls 
}