/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2022. All rights reserved.
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
	"github.com/sirupsen/logrus"

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

type dockerImageHandler struct{}

func (d dockerImageHandler) downloadImage(req *pb.UpdateRequest) (string, error) {
	neededPath, err := prepareEnv()
	if err != nil {
		return "", err
	}
	if _, err = d.getRootfsArchive(req, neededPath); err != nil {
		return "", err
	}
	return createOSImage(neededPath)
}

func (d dockerImageHandler) getRootfsArchive(req *pb.UpdateRequest, neededPath preparePath) (string, error) {
	imageName := req.ContainerImage
	if err := isValidImageName(imageName); err != nil {
		return "", err
	}
	logrus.Infof("start pull %s", imageName)
	if err := runCommand("docker", "pull", imageName); err != nil {
		return "", err
	}
	if err := checkOCIImageDigestMatch("docker", imageName, req.CheckSum); err != nil {
		return "", err
	}
	containerName := "kubeos-temp"
	dockerPsCmd := "docker ps -a -f=name=" + containerName + "| awk 'NR==2' | awk '{print $1}'"
	existId, err := runCommandWithOut("bash", "-c", dockerPsCmd)
	if err != nil {
		return "", err
	}
	if existId != "" {
		logrus.Infoln("kubeos-temp container exist,start clean environment first")
		if err := runCommand("docker", "rm", existId); err != nil {
			return "", err
		}
	}
	logrus.Infof("start get rootfs")
	containerId, err := runCommandWithOut("docker", "create", "--name", containerName, imageName)
	if err != nil {
		return "", err
	}
	if err := runCommand("docker", "cp", containerId+":/"+rootfsArchive, neededPath.updatePath); err != nil {
		return "", err
	}
	defer func() {
		if err := runCommand("docker", "rm", containerId); err != nil {
			logrus.Errorln("remove kubeos-temp container error", err)
		}
	}()
	return neededPath.tarPath, nil
}
