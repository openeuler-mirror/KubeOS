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
	"bufio"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"

	_ "github.com/mitchellh/mapstructure"
	"github.com/shirou/gopsutil/v3/disk"
	"github.com/sirupsen/logrus"
)

// prevent when file exists but deleted by defer deleteFile()
var override bool

func checkParam(arg string) error {
	checkRegex := `\||;|&|&&|\|\||>|>>|<|,|#|!|\$`
	match, err := regexp.MatchString(checkRegex, arg)

	if err != nil {
		return err
	}

	if match {
		return fmt.Errorf("params %s is invalid, please check it", arg)
	}

	return nil
}

func checkDiskSpace(imgType string) error {
	path := "."

	usage, err := disk.Usage(path)
	if err != nil {
		return fmt.Errorf("can not check disk usage")
	}

	minSize := imgTypeMiniSize[imgType]
	if usage.Free < minSize {
		return fmt.Errorf("the available disk space is not enough, at least %d GiB", minSize/(1024*1024))
	}

	return nil
}

func checkFileValid(path string) error {
	fileinfo, err := os.Stat(path)
	if err != nil {
		return fmt.Errorf("error checking file validity: %s", err)
	}

	fileMode := fileinfo.Mode()

	judge := fileMode.IsRegular()
	if judge != true {
		return fmt.Errorf("file is not a file: %s", err)
	}
	return nil
}

// check if repo path is valid, and check the rootfs directory
func checkRepoPath(repoPath string) error {

	if err := checkFileValid(repoPath); err != nil {
		return err
	}

	dir, err := os.Getwd()
	if err != nil {
		return err
	}

	rpmRoot := filepath.Join(dir, "rootfs")

	_, err = os.Stat(rpmRoot)
	if err == nil {
		// the rootfs folder exist
		return fmt.Errorf("there is a rootfs folder %s. please confirm if it is being used, and remove it first", rpmRoot)

	} else if !os.IsNotExist(err) {
		// some other errors
		return err
	}

	return nil
}

// check if docker image exist
func checkDockerExist(d string) error {

	// execute docker images -q d
	cmd := exec.Command("docker", "images", "-q", d)
	cmd.Stderr = os.Stderr

	if out, err := cmd.Output(); err != nil {
		return fmt.Errorf("docker image %s not exist, please pull it first", d)
	} else if len(out) == 0 {
		return fmt.Errorf("docker image %s not exist, please pull it first", d)
	}

	return nil
}

func verifyRepoInput(config Configs) error {

	required := []string{config.Option.Image, config.Option.V, config.Option.B}
	mention := []string{"image", "version", "bianry address"}

	for i := 0; i <= 2; i++ {
		value := required[i]
		err := checkParam(value)
		if value == "" || err != nil {
			return fmt.Errorf("%s option is invalid : %s", mention[i], err)
		}
	}

	return nil
}

func verifyDockerInput(d string) error {
	err := checkParam(d)
	if d == "" || err != nil {
		return fmt.Errorf("docker param %s is invalid : %s", d, err)
	}
	return nil
}

// recive what happened while shell is executing and print out
func cmdPipe(cmd *exec.Cmd) error {

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return fmt.Errorf("error while creating stdout pipe: %s", err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return fmt.Errorf("error while creating stderr pipe: %s", err)
	}

	err = cmd.Start()
	if err != nil {
		return fmt.Errorf("error while starting command: %s", err)
	}

	go printOutput(stdout)
	go printOutput(stderr)

	err = cmd.Wait()
	if err != nil {
		return fmt.Errorf("error while executing kbimg.sh: %s", err)
	}

	return nil
}

// print out the process while executing kbimg.sh
func printOutput(reader io.Reader) {
	scanner := bufio.NewScanner(reader)
	for scanner.Scan() {
		logrus.Info(scanner.Text())
	}
}

// check if file exists before write file
func writeFile(path string, content string, permission os.FileMode) error {
	if _, err := os.Stat(path); !os.IsNotExist(err) {
		logrus.Infof("file %s already exists, do you want to override it? (y/n)", path)
		var answer string
		fmt.Scanln(&answer)
		if answer == "n" {
			override = false
			return fmt.Errorf("file already exists at %s, please check it first", path)
		}
		override = true
	}

	err := os.WriteFile(path, []byte(content), permission)
	if err != nil {
		return fmt.Errorf("can not write file at %s", path)
	}
	return nil
}

func deleteFile(filePath string) error {
	if override == true {
		err := os.Remove(filePath)
		if err != nil {
			return fmt.Errorf("failed to delete file: %s", err)
		}
		return nil
	}
	return fmt.Errorf("user chose to keep file")
}

func deleteFiles() {
	deleteFile(copyScripts)
	deleteFile(partitionScripts)
	deleteFile(userScripts)
}

func writeShells() error {
	if err := applyPartitionScriptEfi(); err != nil {
		return err
	}

	if err := applyCpFilesScript(); err != nil {
		return err
	}

	if err := applyUserConfig(); err != nil {
		return err
	}
	override = true
	return nil
}
