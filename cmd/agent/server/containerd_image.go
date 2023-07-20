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
	"io"
	"os"

	"github.com/sirupsen/logrus"

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

var (
	defaultNamespace = "k8s.io"
)

type conImageHandler struct{}

func (c conImageHandler) downloadImage(req *pb.UpdateRequest) (string, error) {
	neededPath, err := prepareEnv()
	if err != nil {
		return "", err
	}
	if _, err = c.getRootfsArchive(req, neededPath); err != nil {
		return "", err
	}
	return createOSImage(neededPath)
}

func (c conImageHandler) getRootfsArchive(req *pb.UpdateRequest, neededPath preparePath) (string, error) {
	imageName := req.ContainerImage
	mountPath := neededPath.mountPath
	var containerdCommand string
	logrus.Infof("start pull %s", imageName)

	if isCommandAvailable("crictl") {
		containerdCommand = "crictl"
		if err := runCommand("crictl", "pull", imageName); err != nil {
			return "", err
		}
	} else {
		containerdCommand = "ctr"
		if err := runCommand("ctr", "-n", defaultNamespace, "images", "pull", "--host-dir",
			"/etc/containerd/certs.d", imageName); err != nil {
			return "", err
		}
	}

	if err := checkOCIImageDigestMatch(containerdCommand, imageName, req.CheckSum); err != nil {
		return "", err
	}

	if err := checkAndCleanMount(mountPath); err != nil {
		logrus.Errorln("containerd clean environment error", err)
		return "", err
	}
	logrus.Infof("start get rootfs %s", imageName)
	if err := runCommand("ctr", "-n="+defaultNamespace, "images", "mount", "--rw",
		imageName, mountPath); err != nil {
		return "", err
	}
	defer checkAndCleanMount(mountPath)
	if err := copyFile(neededPath.tarPath, mountPath+"/"+rootfsArchive); err != nil {
		return "", err
	}
	return "", nil
}

func checkAndCleanMount(mountPath string) error {
	ctrSnapshotCmd := "ctr " + "-n=" + defaultNamespace + " snapshots ls | grep " + mountPath + " | awk '{print $1}'"
	existSnapshot, err := runCommandWithOut("bash", "-c", ctrSnapshotCmd)
	if err != nil {
		return err
	}
	if existSnapshot != "" {
		if err = runCommand("ctr", "-n="+defaultNamespace, "images", "unmount", mountPath); err != nil {
			return err
		}
		if err = runCommand("ctr", "-n="+defaultNamespace, "snapshots", "remove", mountPath); err != nil {
			return err
		}
	}
	return nil
}

func copyFile(dstFileName string, srcFileName string) error {
	srcFile, err := os.Open(srcFileName)
	if err != nil {
		return err
	}
	defer srcFile.Close()

	reader := bufio.NewReader(srcFile)

	dstFile, err := os.OpenFile(dstFileName, os.O_WRONLY|os.O_CREATE, imgPermission)
	if err != nil {
		return err
	}
	writer := bufio.NewWriter(dstFile)

	defer dstFile.Close()
	if _, err = io.Copy(writer, reader); err != nil {
		return err
	}
	return nil
}
