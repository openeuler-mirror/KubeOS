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

// Package main is the main.go of hostshell
package main

import (
	"os"
	"strconv"
	"strings"
	"syscall"

	"github.com/sirupsen/logrus"
)

const (
	bashPath               = "/usr/bin/bash"
	usrBin                 = "/usr/bin"
	usrSbin                = "/usr/sbin"
	localBin               = "/usr/local/bin"
	localSbin              = "/usr/local/sbin"
	usrLib                 = "/usr/lib"
	usrLib64               = "/usr/lib64"
	lib                    = "/lib"
	lib64                  = "/lib64"
	envPathPrefix          = "PATH=$PATH:"
	envLdLibraryPathPrefix = "LD_LIBRARY_PATH=$LD_LIBRARY_PATH:"
)

func main() {
	EUID := os.Geteuid()
	rootEUID := 0 // 0 indicates that the process has the permission of the root user.
	if EUID != rootEUID {
		logrus.Error("please use root to run hostshell")

	}
	PPID := os.Getppid()
	rootFsPath := "/proc/" + strconv.Itoa(PPID) + "/root"
	path := concatenateEnvPath(rootFsPath, envPathPrefix, []string{usrBin, usrSbin, localBin, localSbin})
	libPath := concatenateEnvPath(rootFsPath, envLdLibraryPathPrefix, []string{usrLib, usrLib64, lib, lib64})
	if err := syscall.Exec("/usr/bin/nsenter", []string{"nsenter", "-t", "1", "-a",
		"env", "-i", path, libPath, rootFsPath + bashPath}, os.Environ()); err != nil {
		logrus.Error("nsenter excute error", err)
	}
}

func concatenateEnvPath(prefix string, envVarPrefix string, paths []string) string {
	for i, p := range paths {
		paths[i] = prefix + p
	}
	pathLine := envVarPrefix + strings.Join(paths, ":")
	pathEnv := os.ExpandEnv(pathLine)
	return pathEnv
}
