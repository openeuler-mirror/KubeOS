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
	"context"
	"fmt"
	"io/ioutil"
	"os"
	"path/filepath"
	"syscall"

	"github.com/docker/docker/api/types"
	"github.com/docker/docker/api/types/container"
	"github.com/docker/docker/client"
	"github.com/docker/docker/pkg/archive"
	"github.com/sirupsen/logrus"

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

func pullOSImage(req *pb.UpdateRequest) (string, error) {
	ctx := context.Background()
	cli, err := client.NewEnvClient()
	if err != nil {
		return "", err
	}
	imageName := req.DockerImage
	logrus.Infof("start pull %s", imageName)
	out, err := cli.ImagePull(ctx, imageName, types.ImagePullOptions{})
	if err != nil {
		return "", err
	}
	defer out.Close()
	if _, err = ioutil.ReadAll(out); err != nil {
		return "", err
	}

	info, err := cli.ContainerCreate(ctx, &container.Config{
		Image: imageName,
	}, nil, nil, "kubeos-temp")
	if err != nil {
		return "", err
	}
	defer cli.ContainerRemove(ctx, info.ID, types.ContainerRemoveOptions{})
	tarStream, stat, err := cli.CopyFromContainer(ctx, info.ID, "/os.tar")
	if err != nil {
		return "", err
	}
	defer tarStream.Close()

	fs := syscall.Statfs_t{}
	if err = syscall.Statfs(PersistDir, &fs); err != nil {
		return "", err
	}
	needGBSize := 3
	kb := 1024
	needDiskSize := needGBSize * kb * kb * kb
	if int64(fs.Bfree)*fs.Bsize < int64(needDiskSize) { // these data come from disk size, will not overflow
		return "", fmt.Errorf("space is not enough for downloaing")
	}

	srcInfo := archive.CopyInfo{
		Path:   "/",
		Exists: true,
		IsDir:  stat.Mode.IsDir(),
	}
	if err = archive.CopyTo(tarStream, srcInfo, PersistDir); err != nil {
		return "", err
	}

	tmpMountPath := filepath.Join(PersistDir, "/kubeos-update")
	if err = os.Mkdir(tmpMountPath, imgPermission); err != nil {
		return "", err
	}
	defer os.Remove(tmpMountPath)
	imagePath := filepath.Join(PersistDir, "/update.img")
	if err = runCommand("dd", "if=/dev/zero", "of="+imagePath, "bs=2M", "count=1024"); err != nil {
		return "", err
	}
	_, next, err := getNextPart(partA, partB)
	if err = runCommand("mkfs.ext4", "-L", "ROOT-"+next, imagePath); err != nil {
		return "", err
	}
	if err = runCommand("mount", "-o", "loop", imagePath, tmpMountPath); err != nil {
		return "", err
	}
	defer func() {
		syscall.Unmount(tmpMountPath, 0)
		runCommand("losetup", "-D")
	}()

	logrus.Infoln("downloading to file " + imagePath)
	tmpTarPath := filepath.Join(PersistDir, "/os.tar")
	if err = runCommand("tar", "-xvf", tmpTarPath, "-C", tmpMountPath); err != nil {
		return "", err
	}
	defer os.Remove(tmpTarPath)
	return imagePath, nil
}
