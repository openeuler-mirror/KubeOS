#!/bin/bash
#######################################################################
##- @Copyright (C) Huawei Technologies., Ltd. 2021. All rights reserved.
# - KubeOS licensed under the Mulan PSL v2.
# - You can use this software according to the terms and conditions of the Mulan PSL v2.
# - You may obtain a copy of Mulan PSL v2 at:
# -     http://license.coscl.org.cn/MulanPSL2
# - THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
# - IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
# - PURPOSE.
# - See the Mulan PSL v2 for more details.
#######################################################################

#!/bin/bash

function get_release_notes()
{
        LAST_RELEASE=$(git describe --tags --abbrev=0)
        # Prepare proposed delease notes
        echo "$(date "+%Y-%m-%d") $USER release $1"
        git log --first-parent --oneline $LAST_RELEASE.. | cut -d' ' -f 2- | sed 's/^/    - /'
        echo ""
        echo "    dev stats:"
        echo "      -$(git diff --shortstat $LAST_RELEASE)"
        echo -n "      - contributors: "
        git shortlog -ns --no-merges $LAST_RELEASE..HEAD | cut -d$'\t' -f 2 | sed -e ':a' -e 'N' -e '$!ba' -e 's/\n/, /g'
        echo ""
}

if [ $# -ne 1 ];then
        echo "Usage:"
        echo "./hack/releasenote.sh v1.0.0"
        exit 0
fi
get_release_notes $1
