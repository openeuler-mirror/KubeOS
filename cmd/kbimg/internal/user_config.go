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

var hostnameContent string
var grubContent string
var serviceContent string

// path of userConfig.sh
var userScripts = filepath.Join("scripts", "userConfig.sh")

func applyUserConfig() error {

	err := generateHostScript(config.Host)
	if err != nil {
		return fmt.Errorf("creating host script error: %s", err)
	}

	err = generateGrubScript(config.Grub)
	if err != nil {
		return fmt.Errorf("creating grub script error: %s", err)
	}

	err = generateServiceScript(config.Systemd)
	if err != nil {
		return fmt.Errorf("creating service script error: %s", err)
	}

	content, err := generateUserScript(config.Users)
	if err != nil {
		return fmt.Errorf("creating user script error: %s", err)
	}

	err = writeFile(userScripts, content, ownerPermission)
	if err != nil {
		return fmt.Errorf("writing userConfig.sh error: %s", err)
	}
	return nil
}

func generateUserScript(users []UserConfig) (string, error) {
	var content string
	content += `#!/bin/bash` + "\n" + "\n"
	content += "function user_config() {\n"
	var length int = len(config.Users)
	for i := 0; i < length; i++ {
		content += fmt.Sprintf("local user%d=%s\n", i, config.Users[i].Name)
		content += fmt.Sprintf("local passwd%d='%s'\n", i, config.Users[i].Passwd)
		content += fmt.Sprintf("local group%d=%s\n", i, config.Users[i].Groups)
		// Check if the group exists
		content += `	if ! getent group $group` + strconv.Itoa(i) + ` > /dev/null; then
		groupadd $group` + strconv.Itoa(i) + `
		echo "group $group` + strconv.Itoa(i) + ` created successfully"
		fi` + "\n"

		// Check if the user exists
		content += `	if ! id $user` + strconv.Itoa(i) + ` > /dev/null 2>&1; then 
		useradd $user` + strconv.Itoa(i) + ` -p $passwd` + strconv.Itoa(i) + `
		echo "user $user` + strconv.Itoa(i) + ` created successfully"
		fi` + "\n"

		// addd user into group
		content += fmt.Sprintf("usermod -a -G $group%d $user%d \n", i, i)
		content += fmt.Sprintf(`echo "user $user%d have been added into $group%d successfully"`+"\n\n", i, i)

	}

	content += `}` + "\n"

	content += `user_config` + "\n"

	content += hostnameContent + "\n"
	content += grubContent + "\n"
	content += serviceContent + "\n"

	return content, nil
}

func generateHostScript(host HostConfig) error {
	hostnameContent += `sh -c "echo '` + config.Host.HostName + `' > /etc/hostname"`

	return nil
}

func generateGrubScript(grub GrubConfig) error {

	grubContent += `
	echo "set superusers=\"root\"" >> /etc/grub.d/00_header
    echo "password_pbkdf2 root ` + config.Grub.Password + `" >> /etc/grub.d/00_header`

	return nil
}

func generateServiceScript(service []ServiceConfig) error {
	for i := 0; i < len(config.Systemd); i++ {
		if config.Systemd[i].Start == true {
			serviceContent += `systemctl enable ` + config.Systemd[i].Name
		}
	}

	return nil
}
