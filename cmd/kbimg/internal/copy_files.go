/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2021. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

// Package internal implements the scripts and invocation of KubeOS image customization.
package internal

import (
	"fmt"
	"path/filepath"
	"strconv"

	_ "github.com/mitchellh/mapstructure"
)

var copyScripts = filepath.Join("scripts", "create", "cpFiles.sh")

func applyCpFilesScript() error {
	content := generateCpFilesScript(config.Files)

	err := writeFile(copyScripts, content, ownerPermission)
	if err != nil {
		defer deleteFile(copyScripts)
		return fmt.Errorf("writing cpFiles.sh error :%s", err)
	}

	return nil
}

func generateCpFilesScript(files []FileConfig) string {
	var content string
	content += "function install_files() {\n"

	var length int = len(config.Files)
	for i := 0; i < length; i++ {
		content += fmt.Sprintf("local src_path%d=%s\n", i, config.Files[i].SourcePath)
		content += fmt.Sprintf("local target_path%d=%s\n", i, config.Files[i].TargetPath)
		content += `target_path` + strconv.Itoa(i) + `=${RPM_ROOT}$` + `{target_path` + strconv.Itoa(i) + `}` + "\n"
		content += `if [ -d "$src_path` + strconv.Itoa(i) + `" ]; then
		cp -r "$src_path` + strconv.Itoa(i) + `" "$target_path` + strconv.Itoa(i) + `" 
		echo "-----------------------copy directory successed-------------------------"
	  else
		cp "$src_path` + strconv.Itoa(i) + `" "$target_path` + strconv.Itoa(i) + `"
		echo "-----------------------copy file successed--------------------------"
	  fi` + "\n" + "\n"
	}

	content += `echo "*** source files copied ***"` + "\n"

	content += "}\n"

	return content
}
