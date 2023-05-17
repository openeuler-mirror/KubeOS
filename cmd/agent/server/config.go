/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2023. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

// Package server implements server of os-agent and listener of os-agent server. The server uses gRPC interface.
package server

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"

	"github.com/sirupsen/logrus"

	agent "openeuler.org/KubeOS/cmd/agent/api"
)

const (
	defaultProcPath            = "/proc/sys/"
	defaultKernelConPath       = "/etc/sysctl.conf"
	defaultKernelConPermission = 0644
)

type Configuration interface {
	SetConfig(config *agent.SysConfig) error
}

type KernelSysctl struct{}

func (k KernelSysctl) SetConfig(config *agent.SysConfig) error {
	logrus.Info("start set kernel.sysctl")
	for key, value := range config.Contents {
		procPath := getProcPath(key)
		if err := os.WriteFile(procPath, []byte(value), 0644); err != nil {
			return err
		}
	}
	return nil
}

type KerSysctlPersist struct{}

func (k KerSysctlPersist) SetConfig(config *agent.SysConfig) error {
	logrus.Info("start set kernel.sysctl")
	configPath := config.ConfigPath
	if configPath == "" {
		configPath = defaultKernelConPath
	}
	fileExist, err := checkConfigPath(configPath)
	if err != nil {
		return err
	}
	configs, err := getAndSetConfigsFromFile(config.Contents, configPath, fileExist)
	if err != nil {
		return err
	}
	if err = writeConfigToFile(configPath, configs); err != nil {
		return err
	}
	return nil
}

type GrubCmdline struct{}

func (g GrubCmdline) SetConfig(config *agent.SysConfig) error {
	logrus.Info("start set kernel.sysctl")
	for key, value := range config.Contents {
		fmt.Println(key + "=" + value)
	}
	return nil
}

func startConfig(configs []*agent.SysConfig) error {
	for _, config := range configs {
		fmt.Println(config.Model)
		if err := ConfigFactoryTemplate(config.Model, config); err != nil {
			return err
		}
	}
	return nil
}

var doConfig sync.Once
var configTemplate = make(map[string]Configuration)

func ConfigFactoryTemplate(configType string, config *agent.SysConfig) error {
	doConfig.Do(func() {
		configTemplate[KernelSysctlName.String()] = new(KernelSysctl)
		configTemplate[KerSysctlPersistName.String()] = new(KerSysctlPersist)
		configTemplate[GrubCmdlineName.String()] = new(GrubCmdline)
	})
	return configTemplate[configType].SetConfig(config)
}

func getProcPath(key string) string {
	return filepath.Join(defaultProcPath, strings.Replace(key, ".", "/", -1))
}

func getAndSetConfigsFromFile(expectConfigs map[string]string, path string, fileExist bool) ([]string, error) {
	var configsWrite []string
	if fileExist {
		file, err := os.Open(path)
		if err != nil {
			return nil, err
		}
		defer file.Close()

		configScanner := bufio.NewScanner(file)
		for configScanner.Scan() {
			line := configScanner.Text()
			//if comment or blank
			if strings.HasPrefix(line, "#") || strings.HasPrefix(line, ";") || line == "" {
				configsWrite = append(configsWrite, line)
				continue
			}
			configKV := strings.Split(line, "=")
			if len(configKV) != 2 {
				logrus.Errorln("could not parse systctl config %s", line)
				return nil, fmt.Errorf("could not parse systctl config %s", line)
			}
			key := strings.TrimSpace(configKV[0])
			if newValue, ok := expectConfigs[key]; ok {
				config := key + " = " + newValue
				configsWrite = append(configsWrite, config)
				delete(expectConfigs, key) //删除已经赋值的
				continue
			}
			configsWrite = append(configsWrite, line)
		}
		if err = configScanner.Err(); err != nil {
			return nil, err
		}
	}
	for newKey, newValue := range expectConfigs {
		config := newKey + " = " + newValue
		configsWrite = append(configsWrite, config)
	}

	return configsWrite, nil
}

func writeConfigToFile(path string, configs []string) error {
	logrus.Info("write configuration to file", path)
	f, err := os.OpenFile(path, os.O_RDWR|os.O_TRUNC, defaultKernelConPermission)
	if err != nil {
		return err
	}
	defer f.Close()
	w := bufio.NewWriter(f)
	for _, line := range configs {
		if _, err = w.WriteString(line + "\n"); err != nil {
			return err
		}
	}
	if err = w.Flush(); err != nil {
		return err
	}
	return nil
}

func checkConfigPath(configPath string) (bool, error) {
	fileExist, err := checkFileExist(configPath)
	if err != nil {
		return false, err
	}
	if !fileExist {
		f, err := os.Create(configPath)
		if err != nil {
			return false, err
		}
		defer f.Close()
		return false, nil
	}
	return true, nil
}
