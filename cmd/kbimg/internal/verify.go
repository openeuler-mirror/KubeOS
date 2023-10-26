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
	"os"
	"os/exec"

	_ "github.com/mitchellh/mapstructure"
	"github.com/sirupsen/logrus"
	"github.com/spf13/viper"
)

const ownerPermission = os.FileMode(0600)

var config Configs
var cmd *exec.Cmd
var workDir = "./scripts"

var imgTypeMiniSize = map[string]uint64{
	"docker": 6 * 1024 * 1024,
	"vm":     25 * 1024 * 1024,
	"pxe":    5 * 1024 * 1024,
}

// Configs defines user configs
type Configs struct {
	Option     OptionConfig      `mapstructure:"option_config"`
	Partitions []PartitionConfig `mapstructure:"partition_config"`
	Files      []FileConfig      `mapstructure:"file_config"`
	Users      []UserConfig      `mapstructure:"user_config"`
	Host       HostConfig        `mapstructure:"host_config"`
	Grub       GrubConfig        `mapstructure:"grub_config"`
	Systemd    []ServiceConfig   `mapstructure:"systemd_service_config"`
}

// OptionConfig defines the image creating options
type OptionConfig struct {
	Image string `mapstructure:"image"`
	P     string `mapstructure:"p"`
	V     string `mapstructure:"v"`
	B     string `mapstructure:"b"`
	E     string `mapstructure:"e"`
	D     string `mapstructure:"d"`
	L     string `mapstructure:"l"`
}

// PartitionConfig defines the partition options
type PartitionConfig struct {
	Label string `mapstructure:"label"`
	Limit int    `mapstructure:"limit"`
	Type  string `mapstructure:"type"`
}

// FileConfig defines the file options
type FileConfig struct {
	SourcePath string `mapstructure:"sourcePath"`
	TargetPath string `mapstructure:"targetPath"`
}

// UserConfig defines the users/groups options
type UserConfig struct {
	Name   string `mapstructure:"name"`
	Passwd string `mapstructure:"passwd"`
	Groups string `mapstructure:"groups"`
}

// HostConig defines the hostname option
type HostConfig struct {
	HostName string `mapstructure:"hostname"`
}

// GrubConfig defines password option
type GrubConfig struct {
	Password string `mapstructure:"password"`
}

// ServiceConfig defines the systemd service options
type ServiceConfig struct {
	Name  string `mapstructure:"name"`
	Start bool   `mapstructure:"start"`
}

func initConfig() {

	viper.SetConfigFile(cfgFile)

	// read cfgFile
	if err := viper.ReadInConfig(); err != nil {
		logrus.Errorln("Failed to read the configuration file: " + err.Error())
		return
	}

	// decode kbimg.yaml
	if err := viper.Unmarshal(&config); err != nil {
		logrus.Errorln("Failed to decode:" + err.Error())
		return
	}
	logrus.Info("Successed to decode")

	// check image creat mode is legacy or efi, and show error if it went wrong
	// if config.Option.L == "legacy" {
	// 	err := applyPartitionScriptLegacy()
	// 	if err != nil {
	// 		logrus.Errorln("Error while writing partCreate.sh: ", err.Error())
	// 		return
	// 	}
	// } else {
	// if err := applyPartitionScriptEfi(); err != nil {
	// 		logrus.Errorln("Error while writing partCreate.sh: ", err.Error())
	// 		return
	// 	}
	// // }

	// if err := applyCpFilesScript(); err != nil {
	// 	logrus.Errorln("Error while writing cpFiles.sh: ", err.Error())
	// 	return
	// }

	// if err := applyUserConfig(); err != nil {
	// 	logrus.Errorln("Applying user configurations error:", err.Error())
	// 	return
	// }

	if err := verifyCreateInput(config); err != nil {
		logrus.Errorln("Error while creating image:", err.Error())
		return
	}
}

func verifyCreateInput(config Configs) error {

	switch config.Option.Image {

	// verify upgrade_image args
	case "upgrade_image":

		defer deleteFiles()

		logrus.Info("Creating KubeOS [docker image]")

		if err := checkDiskSpace("docker"); err != nil {
			return err
		}

		if err := verifyRepoInput(config); err != nil {
			return err
		}

		if err := verifyDockerInput(config.Option.D); err != nil {
			return err
		}

		if err := checkRepoPath(config.Option.P); err != nil {
			return err
		}

		if err := checkFileValid(config.Option.B); err != nil {
			return err
		}

		if err := writeShells(); err != nil {
			return err
		}

		cmd = exec.Command("bash", "kbimg.sh", "create", "upgrade-image",
			"-p", config.Option.P, "-v", config.Option.V,
			"-b", config.Option.B, "-e", config.Option.E,
			"-d", config.Option.D)
		cmd.Dir = workDir
		logrus.Infoln("Cmd Args:", cmd.Args)

		if err := cmdPipe(cmd); err != nil {
			return err
		}

	// verify vm_image_repo args
	case "vm_image_repo":

		defer deleteFiles()

		logrus.Info("Creating KubeOS vitural machine image by [repo]")

		if err := checkDiskSpace("vm"); err != nil {
			return err
		}

		if err := verifyRepoInput(config); err != nil {
			return err
		}

		if err := checkRepoPath(config.Option.P); err != nil {
			return err
		}

		if err := checkFileValid(config.Option.B); err != nil {
			return err
		}

		if err := writeShells(); err != nil {
			return err
		}

		cmd = exec.Command("bash", "kbimg.sh", "create", "vm-image",
			"-p", config.Option.P, "-v", config.Option.V,
			"-b", config.Option.B, "-e", config.Option.E)
		cmd.Dir = workDir

		logrus.Infoln("Cmd Args:", cmd.Args)

		if err := cmdPipe(cmd); err != nil {
			return err
		}

	// verify vm_image_docker args
	case "vm_image_docker":

		logrus.Info("Creating KubeOS vitural machine image by [docker image]")

		if err := verifyDockerInput(config.Option.D); err != nil {
			return err
		}

		if err := checkDockerExist(config.Option.D); err != nil {
			return err
		}

		cmd = exec.Command("bash", "kbimg.sh", "create", "vm-image", "-d", config.Option.D)
		cmd.Dir = workDir

		logrus.Infoln("Cmd Args:", cmd.Args)

		if err := cmdPipe(cmd); err != nil {
			return err
		}

	// verify pxe_image_repo args
	case "pxe_image_repo":

		defer deleteFiles()

		logrus.Info("Creating KubeOS pxe image by [repo]")

		if err := checkDiskSpace("pxe"); err != nil {
			return err
		}
		if err := verifyRepoInput(config); err != nil {
			return err
		}
		if err := checkRepoPath(config.Option.P); err != nil {
			return err
		}
		if err := checkFileValid(config.Option.B); err != nil {
			return err
		}
		if err := writeShells(); err != nil {
			return err
		}

		cmd = exec.Command("bash", "kbimg.sh", "create", "pxe-image",
			"-p", config.Option.P, "-v", config.Option.V,
			"-b", config.Option.B, "-e", config.Option.E)
		cmd.Dir = workDir

		logrus.Infoln("Cmd Args:", cmd.Args)

		if err := cmdPipe(cmd); err != nil {
			return err
		}

	// verify pxe_image_docker args
	case "pxe_image_docker":

		fmt.Println("Creating KubeOS pxe image by [docker image]")

		if err := verifyDockerInput(config.Option.D); err != nil {
			return err
		}
		if err := checkDockerExist(config.Option.D); err != nil {
			return err
		}

		cmd = exec.Command("bash", "kbimg.sh", "create", "pxe-image", "-d", config.Option.D)
		cmd.Dir = workDir

		logrus.Infoln("Cmd Args:", cmd.Args)

		if err := cmdPipe(cmd); err != nil {
			return err
		}
	}

	return nil
}
