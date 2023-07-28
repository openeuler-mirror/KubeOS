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
	"regexp"
	"strings"
	"sync"

	"github.com/sirupsen/logrus"

	agent "openeuler.org/KubeOS/cmd/agent/api"
)

const (
	defaultProcPath            = "/proc/sys/"
	defaultKernelConPath       = "/etc/sysctl.conf"
	defalutGrubCfgPath         = "/boot/efi/EFI/openEuler/grub.cfg"
	defaultKernelConPermission = 0644
	defaultGrubCfgPermission   = 0751
)

// Configuration defines interface of configuring
type Configuration interface {
	SetConfig(config *agent.SysConfig) error
}

// KernelSysctl represents kernel.sysctl configuration
type KernelSysctl struct{}

// SetConfig sets kernel.sysctl configuration
func (k KernelSysctl) SetConfig(config *agent.SysConfig) error {
	logrus.Info("start set kernel.sysctl")
	for key, keyInfo := range config.Contents {
		procPath := getProcPath(key)
		if keyInfo.Operation == "delete" {
			logrus.Errorf("Failed to delete kernel.sysctl config with key %s", key)
		} else if keyInfo.Operation == "add" || keyInfo.Operation == "update" || keyInfo.Operation == "" {
			if err := os.WriteFile(procPath, []byte(keyInfo.Value), defaultKernelConPermission); err != nil {
				logrus.Errorf("Failed to write kernel.sysctl with key %s: %v", key, err)
				return err
			}
		} else {
			logrus.Errorf("Failed to parse kernel.sysctl config operation %s", keyInfo.Operation)
		}
	}
	return nil
}

// KerSysctlPersist represents kernel.sysctl.persist configuration
type KerSysctlPersist struct{}

// SetConfig sets kernel.sysctl.persist configuration
func (k KerSysctlPersist) SetConfig(config *agent.SysConfig) error {
	logrus.Info("start set kernel.sysctl.persist")
	configPath := config.ConfigPath
	if configPath == "" {
		configPath = getKernelConPath()
	}
	if err := createConfigPath(configPath); err != nil {
		logrus.Errorf("Failed to find config path: %v", err)
		return err
	}
	configs, err := getAndSetConfigsFromFile(config.Contents, configPath)
	if err != nil {
		logrus.Errorf("Failed to set persist kernel configs: %v", err)
		return err
	}
	if err = writeConfigToFile(configPath, configs); err != nil {
		logrus.Errorf("Failed to write configs to file: %v", err)
		return err
	}
	return nil
}

// GrubCmdline represents grub.cmdline configuration
type GrubCmdline struct {
	// it represents which partition the user want to configure
	isCurPartition bool
}

// SetConfig sets grub.cmdline configuration
func (g GrubCmdline) SetConfig(config *agent.SysConfig) error {
	logrus.Info("start set grub.cmdline configuration")
	fileExist, err := checkFileExist(getGrubCfgPath())
	if err != nil {
		logrus.Errorf("Failed to find config path: %v", err)
		return err
	}
	if !fileExist {
		return fmt.Errorf("failed to find grub.cfg %s", getGrubCfgPath())
	}
	configPartition, err := getConfigPartition(g.isCurPartition)
	if err != nil {
		logrus.Errorf("Failed to get config partition: %v", err)
		return err
	}
	lines, err := getAndSetGrubCfg(config.Contents, configPartition)
	if err != nil {
		logrus.Errorf("Failed to set grub configs: %v", err)
		return err
	}
	if err := writeConfigToFile(getGrubCfgPath(), lines); err != nil {
		return err
	}
	return nil
}

// getConfigPartition return false if the user want to configure partition A,
// return true if the user want to configure partition B
func getConfigPartition(isCurPartition bool) (bool, error) {
	partA, partB, err := getRootfsDisks()
	if err != nil {
		return false, err
	}
	_, next, err := getNextPart(partA, partB)
	if err != nil {
		return false, err
	}
	var flag bool
	if next == "B" {
		flag = true
	}
	return isCurPartition != flag, nil
}

func getAndSetGrubCfg(expectConfigs map[string]*agent.KeyInfo, configPartition bool) ([]string, error) {
	file, err := os.OpenFile(getGrubCfgPath(), os.O_RDWR, defaultGrubCfgPermission)
	if err != nil {
		return []string{}, err
	}
	defer file.Close()

	reFindCurLinux := `^\s*linux.*root=.*`
	r, err := regexp.Compile(reFindCurLinux)
	if err != nil {
		return []string{}, err
	}

	var lines []string
	var matchCount bool
	configScanner := bufio.NewScanner(file)
	for configScanner.Scan() {
		line := configScanner.Text()
		if r.MatchString(line) {
			if matchCount == configPartition {
				line, err = modifyLinuxCfg(expectConfigs, line)
				if err != nil {
					return []string{}, fmt.Errorf("error modify grub.cfg %v", err)
				}
			}
			matchCount = true
		}
		lines = append(lines, line)
	}
	return lines, nil
}

func modifyLinuxCfg(m map[string]*agent.KeyInfo, line string) (string, error) {
	expectConfigs := deepCopyConfigMap(m)
	newConfigs := []string{"      "}
	oldConfigs := strings.Split(line, " ")
	// Config has two format: key or key=value. Following variables stand for the length after splitting
	onlyKey, KVpair := 1, 2
	for _, oldConfig := range oldConfigs {
		if oldConfig == "" {
			continue
		}
		// At most 2 substrings can be returned to satisfy the case like root=UUID=xxxx
		config := strings.SplitN(oldConfig, "=", KVpair)
		if len(config) != onlyKey && len(config) != KVpair {
			return "", fmt.Errorf("cannot parse grub.cfg linux line %s", oldConfig)
		}
		if keyInfo, ok := expectConfigs[config[0]]; ok {
			if keyInfo.Operation == "delete" {
				if len(config) == KVpair && keyInfo.Value != config[1] {
					logrus.Warnf("Failed to delete key %s with inconsistent values "+
						"%s and %s", config[0], config[1], keyInfo.Value)
					newConfigs = append(newConfigs, oldConfig)
				}
				delete(expectConfigs, config[0])
				continue
			}
			if keyInfo.Operation != "delete" && len(config) == KVpair {
				config[1] = keyInfo.Value
			}
		}
		if len(config) == onlyKey {
			newConfigs = append(newConfigs, config[0])
		} else if len(config) == KVpair {
			newConfigs = append(newConfigs, strings.Join(config, "="))
		}
		delete(expectConfigs, config[0])
	}
	for key, keyInfo := range expectConfigs {
		if keyInfo.Operation == "delete" {
			logrus.Warnf("Failed to delete inexistent key %s", key)
			continue
		}
		if keyInfo.Value == "" {
			newConfigs = append(newConfigs, key)
		} else {
			newConfigs = append(newConfigs, fmt.Sprintf("%s=%s", key, keyInfo.Value))
		}
	}
	return convertNewConfigsToString(newConfigs)
}

func convertNewConfigsToString(newConfigs []string) (string, error) {
	var newLine strings.Builder
	for _, newConfig := range newConfigs {
		if newConfig == "" {
			continue
		}
		if _, err := fmt.Fprintf(&newLine, " %s", newConfig); err != nil {
			return "", err
		}
	}
	return newLine.String(), nil
}

func startConfig(configs []*agent.SysConfig) error {
	for _, config := range configs {
		if err := ConfigFactoryTemplate(config.Model, config); err != nil {
			return err
		}
	}
	return nil
}

var doConfig sync.Once
var configTemplate = make(map[string]Configuration)

// ConfigFactoryTemplate returns the corresponding struct that implements the Configuration
func ConfigFactoryTemplate(configType string, config *agent.SysConfig) error {
	doConfig.Do(func() {
		configTemplate[KernelSysctlName.String()] = new(KernelSysctl)
		configTemplate[KerSysctlPersistName.String()] = new(KerSysctlPersist)
		configTemplate[GrubCmdlineCurName.String()] = &GrubCmdline{isCurPartition: true}
		configTemplate[GrubCmdlineNextName.String()] = &GrubCmdline{isCurPartition: false}
	})
	if _, ok := configTemplate[configType]; ok {
		return configTemplate[configType].SetConfig(config)
	}
	return fmt.Errorf("get configTemplate error : cannot recoginze configType %s", configType)
}

func getProcPath(key string) string {
	return filepath.Join(getDefaultProcPath(), strings.Replace(key, ".", "/", -1))
}

func getAndSetConfigsFromFile(expectConfigs map[string]*agent.KeyInfo, path string) ([]string, error) {
	var configsWrite []string
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	configScanner := bufio.NewScanner(file)
	for configScanner.Scan() {
		line := configScanner.Text()
		// if line is comment or blank
		if strings.HasPrefix(line, "#") || strings.HasPrefix(line, ";") || line == "" {
			configsWrite = append(configsWrite, line)
			continue
		}
		configKV := strings.Split(line, "=")
		requiredLen := 2 // If it is in the key=value format, the length after splitting is 2
		if len(configKV) != requiredLen {
			logrus.Errorf("could not parse systctl config %s", line)
			return nil, fmt.Errorf("could not parse systctl config %s", line)
		}
		key := strings.TrimSpace(configKV[0])
		value := strings.TrimSpace(configKV[1])
		if newKeyInfo, ok := expectConfigs[key]; ok {
			if newKeyInfo.Operation != "delete" {
				config := key + " = " + newKeyInfo.Value
				configsWrite = append(configsWrite, config)
			} else if newKeyInfo.Operation == "delete" && newKeyInfo.Value != value {
				logrus.Warnf("Failed to delete key %s with inconsistent values "+
					"%s and %s", key, value, newKeyInfo.Value)
				configsWrite = append(configsWrite, line)
			}
			delete(expectConfigs, key)
			continue
		}
		configsWrite = append(configsWrite, line)
	}
	if err = configScanner.Err(); err != nil {
		return nil, err
	}

	for newKey, newKeyInfo := range expectConfigs {
		if newKeyInfo.Operation != "delete" {
			config := newKey + " = " + newKeyInfo.Value
			configsWrite = append(configsWrite, config)
		}
	}
	return configsWrite, nil
}

func writeConfigToFile(path string, configs []string) error {
	logrus.Info("write configuration to file ", path)
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

func createConfigPath(configPath string) error {
	fileExist, err := checkFileExist(configPath)
	if err != nil {
		return err
	}
	if fileExist {
		return nil
	}

	f, err := os.Create(configPath)
	if err != nil {
		return err
	}
	defer f.Close()
	return nil
}

func getDefaultProcPath() string {
	return "/proc/sys/"
}

func getKernelConPath() string {
	return "/etc/sysctl.conf"
}

func getGrubCfgPath() string {
	return "/boot/efi/EFI/openEuler/grub.cfg"
}
