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
	"errors"
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

	containerName := "kubeos-temp"
	containers, err := cli.ContainerList(ctx, types.ContainerListOptions{All: true})
	for _, container := range containers {
		if container.Names[0] == "/"+containerName {
			if err = cli.ContainerRemove(ctx, container.ID, types.ContainerRemoveOptions{}); err != nil {
				return "", err
			}
		}
	}
	info, err := cli.ContainerCreate(ctx, &container.Config{
		Image: imageName,
	}, nil, nil, containerName)
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

	tmpUpdatePath := filepath.Join(PersistDir, "/KubeOS-Update")
	tmpMountPath := filepath.Join(tmpUpdatePath, "/kubeos-update")
	tmpTarPath := filepath.Join(tmpUpdatePath, "/os.tar")
	imagePath := filepath.Join(PersistDir, "/update.img")

	if err = cleanSpace(tmpUpdatePath, tmpMountPath, imagePath); err != nil {
		return "", err
	}
	if err = os.MkdirAll(tmpMountPath, imgPermission); err != nil {
		return "", err
	}
	defer os.RemoveAll(tmpUpdatePath)

	srcInfo := archive.CopyInfo{
		Path:   "/",
		Exists: true,
		IsDir:  stat.Mode.IsDir(),
	}
	if err = archive.CopyTo(tarStream, srcInfo, tmpUpdatePath); err != nil {
		return "", err
	}
	if err = runCommand("dd", "if=/dev/zero", "of="+imagePath, "bs=2M", "count=1024"); err != nil {
		return "", err
	}
	if err = os.Chmod(imagePath, imgPermission); err != nil {
		return "", err
	}
	if err = runCommand("mkfs.ext4", "-L", "ROOT-A", imagePath); err != nil {
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
	if err = runCommand("tar", "-xvf", tmpTarPath, "-C", tmpMountPath); err != nil {
		return "", err
	}
	return imagePath, nil
}

func cleanSpace(updatePath, mountPath, imagePath string) error {
	isFileExist, err := checkFileExist(mountPath)
	if err != nil {
		return err
	}
	if isFileExist {
		var st syscall.Stat_t
		if err := syscall.Lstat(mountPath, &st); err != nil {
			return err
		}
		dev := st.Dev
		parent := filepath.Dir(mountPath)
		if err := syscall.Lstat(parent, &st); err != nil {
			return err
		}
		if dev != st.Dev {
			if err := syscall.Unmount(mountPath, 0); err != nil {
				return err
			}
		}
	}

	if err = deleteFile(updatePath); err != nil {
		return err
	}

	if err = deleteFile(imagePath); err != nil {
		return err
	}
	return nil
}

func deleteFile(path string) error {
	isFileExist, err := checkFileExist(path)
	if err != nil {
		return err
	}
	if isFileExist {
		if err = os.RemoveAll(path); err != nil {
			return err
		}
	}
	return nil
}
func checkFileExist(path string) (bool, error) {
	if _, err := os.Stat(path); err == nil {
		return true, nil
	} else if errors.Is(err, os.ErrNotExist) {
		return false, nil
	} else {
		return false, err
	}
}
