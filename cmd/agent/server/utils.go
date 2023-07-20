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
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/sirupsen/logrus"

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

const (
	needGBSize = 3 // the max size of update files needed
	// KB is 1024 B
	KB = 1024
)

var (
	rootfsArchive = "os.tar"
	updateDir     = "KubeOS-Update"
	mountDir      = "kubeos-update"
	osImageName   = "update.img"
)

type imageDownload interface {
	downloadImage(req *pb.UpdateRequest) (string, error)
	getRootfsArchive(req *pb.UpdateRequest, neededPath preparePath) (string, error)
}

type preparePath struct {
	updatePath string
	mountPath  string
	tarPath    string
	imagePath  string
}

func runCommand(name string, args ...string) error {
	out, err := exec.Command(name, args...).CombinedOutput()
	if err != nil {
		return fmt.Errorf("fail to run command:%s %v out:%s err:%s", name, args, out, err)
	}
	return nil
}

func runCommandWithOut(name string, args ...string) (string, error) {
	out, err := exec.Command(name, args...).CombinedOutput()
	if err != nil {
		return "", fmt.Errorf("fail to run command:%s %v out:%s err:%s", name, args, out, err)
	}
	return deleteNewline(string(out)), nil
}

func deleteNewline(out string) string {
	if strings.HasSuffix(out, "\n") {
		out = strings.TrimSuffix(out, "\n")
	}
	return out
}

func install(imagePath string, side string, next string) error {
	if err := runCommand("dd", "if="+imagePath, "of="+side, "bs=8M"); err != nil {
		return err
	}
	defer os.Remove(imagePath)
	bootMode, err := getBootMode()
	if err != nil {
		return err
	}
	if bootMode == "uefi" {
		return runCommand("grub2-editenv", grubenvPath, "set", "saved_entry="+next)
	} else {
		return runCommand("grub2-set-default", next)
	}
}

func getNextPart(partA string, partB string) (string, string, error) {
	out, err := exec.Command("lsblk", "-no", "MOUNTPOINT", partA).CombinedOutput()
	if err != nil {
		return "", "", fmt.Errorf("fail to lsblk %s out:%s err:%s", partA, out, err)
	}
	mountPoint := strings.TrimSpace(string(out))
	logrus.Infoln(partA + " mounted on " + mountPoint)

	side := partA
	if mountPoint == "/" {
		side = partB
	}
	logrus.Infoln("side is " + side)
	next := "B"
	if side != partB {
		next = "A"
	}
	return side, next, nil
}

func getRootfsDisks() (string, string, error) {
	out, err := runCommandWithOut("lsblk", "-lno", "NAME,MOUNTPOINTS")
	if err != nil {
		logrus.Errorln("get rootfs disks error " + err.Error())
		return "", "", err
	}
	var diskName string
	const mountedDeviceOutLen = 2
	mounts := strings.Split(out, "\n")
	for _, m := range mounts {
		res := strings.Fields(m)
		if len(res) != mountedDeviceOutLen {
			continue
		}
		if res[1] == "/" {
			diskName = filepath.Join("/dev", res[0])
		}
	}
	if len(diskName) == 0 {
		logrus.Errorln("get rootfs disks error: not get diskName")
		return "", "", fmt.Errorf("get rootfs disks error: not get diskName")
	}
	curDiskBytes := make([]byte, len(diskName)-1)
	copy(curDiskBytes, diskName[:len(diskName)-1])
	curDisk := string(curDiskBytes)
	const partAPartitionNum = "2"
	const partBartitionNum = "3"
	partA := curDisk + partAPartitionNum
	partB := curDisk + partBartitionNum
	return partA, partB, nil
}

func getBootMode() (string, error) {
	_, err := os.Stat("/sys/firmware/efi")
	if err == nil {
		return "uefi", nil
	} else if os.IsNotExist(err) {
		return "legacy", nil
	} else {
		return "", err
	}
}

func createOSImage(neededPath preparePath) (string, error) {
	imagePath := neededPath.imagePath
	updatePath := neededPath.updatePath
	if err := runCommand("dd", "if=/dev/zero", "of="+imagePath, "bs=2M", "count=1024"); err != nil {
		return "", err
	}
	if err := os.Chmod(imagePath, imgPermission); err != nil {
		return "", err
	}
	if err := runCommand("mkfs.ext4", "-L", "ROOT-A", imagePath); err != nil {
		return "", err
	}
	mountPath := neededPath.mountPath
	if err := runCommand("mount", "-o", "loop", imagePath, mountPath); err != nil {
		return "", err
	}
	defer func() {
		if err := syscall.Unmount(mountPath, 0); err != nil {
			logrus.Errorln("umount error " + mountPath)
		}
		if err := runCommand("losetup", "-D"); err != nil {
			logrus.Errorln("delete loop device error")
		}
		if err := os.RemoveAll(updatePath); err != nil {
			logrus.Errorln("remove dir error " + updatePath)
		}
	}()
	logrus.Infoln("downloading to file " + imagePath)
	tarPath := neededPath.tarPath
	if err := runCommand("tar", "-xvf", tarPath, "-C", mountPath); err != nil {
		return "", err
	}
	return imagePath, nil
}

func prepareEnv() (preparePath, error) {
	if err := checkDiskSize(needGBSize, PersistDir); err != nil {
		return preparePath{}, err
	}
	updatePath := splicePath(PersistDir, updateDir)
	mountPath := splicePath(updatePath, mountDir)
	tarPath := splicePath(updatePath, rootfsArchive)
	imagePath := splicePath(PersistDir, osImageName)

	if err := cleanSpace(updatePath, mountPath, imagePath); err != nil {
		return preparePath{}, err
	}
	if err := os.MkdirAll(mountPath, imgPermission); err != nil {
		return preparePath{}, err
	}
	upgradePath := preparePath{
		updatePath: updatePath,
		mountPath:  mountPath,
		tarPath:    tarPath,
		imagePath:  imagePath,
	}
	return upgradePath, nil
}

func checkDiskSize(needGBSize int, path string) error {
	fs := syscall.Statfs_t{}
	if err := syscall.Statfs(path, &fs); err != nil {
		return err
	}
	needDiskSize := needGBSize * KB * KB * KB
	if int64(fs.Bfree)*fs.Bsize < int64(needDiskSize) { // these data come from disk size, will not overflow
		return fmt.Errorf("space is not enough for downloaing")
	}
	return nil
}

func splicePath(prefix string, path string) string {
	return filepath.Join(prefix, path)
}

func cleanSpace(updatePath, mountPath, imagePath string) error {
	isFileExist, err := checkFileExist(mountPath)
	if err != nil {
		return err
	}
	if isFileExist {
		var st syscall.Stat_t
		if err = syscall.Lstat(mountPath, &st); err != nil {
			return err
		}
		dev := st.Dev
		parent := filepath.Dir(mountPath)
		if err = syscall.Lstat(parent, &st); err != nil {
			return err
		}
		if dev != st.Dev {
			if err = syscall.Unmount(mountPath, 0); err != nil {
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

func checkOCIImageDigestMatch(containerRuntime string, imageName string, checkSum string) error {
	var cmdOutput string
	var err error
	switch containerRuntime {
	case "crictl":
		cmdOutput, err = runCommandWithOut("crictl", "inspecti", "--output", "go-template",
			"--template", "{{.status.repoDigests}}", imageName)
		if err != nil {
			return err
		}
	case "docker":
		cmdOutput, err = runCommandWithOut("docker", "inspect", "--format", "{{.RepoDigests}}", imageName)
		if err != nil {
			return err
		}
	case "ctr":
		cmdOutput, err = runCommandWithOut("ctr", "-n", "k8s.io", "images", "ls", "name=="+imageName)
		if err != nil {
			return err
		}
		// after Fields, we get slice like [REF TYPE DIGEST SIZE PLATFORMS LABELS x x x x x x]
		// the digest is the position 8 element
		imageDigest := strings.Split(strings.Fields(cmdOutput)[8], ":")[1]
		if imageDigest != checkSum {
			logrus.Errorln("checkSumFailed ", imageDigest, " mismatch to ", checkSum)
			return fmt.Errorf("checkSumFailed %s mismatch to %s", imageDigest, checkSum)
		}
		return nil
	default:
		logrus.Errorln("containerRuntime ", containerRuntime, " cannot be recognized")
		return fmt.Errorf("containerRuntime %s cannot be recognized", containerRuntime)
	}
	// cmdOutput format is as follows:
	// [imageRepository/imageName:imageTag@sha256:digests]
	// parse the output and get digest
	var imageDigests string
	outArray := strings.Split(cmdOutput, "@")
	if strings.HasPrefix(outArray[len(outArray)-1], "sha256") {
		pasredArray := strings.Split(strings.TrimSuffix(outArray[len(outArray)-1], "]"), ":")
		// 2 is the expected length of the array after dividing "imageName:imageTag@sha256:digests" based on ':'
		rightLen := 2
		if len(pasredArray) == rightLen {
			digestIndex := 1 // 1 is the index of digest data in pasredArray
			imageDigests = pasredArray[digestIndex]
		}
	}
	if imageDigests == "" {
		logrus.Errorln("error when get ", imageName, " digests")
		return fmt.Errorf("error when get %s digests", imageName)
	}
	if imageDigests != checkSum {
		logrus.Errorln("checkSumFailed ", imageDigests, " mismatch to ", checkSum)
		return fmt.Errorf("checkSumFailed %s mismatch to %s", imageDigests, checkSum)
	}
	return nil
}

func deepCopyConfigMap(m map[string]*pb.KeyInfo) map[string]*pb.KeyInfo {
	result := make(map[string]*pb.KeyInfo)
	for key, val := range m {
		result[key] = &pb.KeyInfo{
			Value:     val.Value,
			Operation: val.Operation,
		}
	}
	return result
}

func isCommandAvailable(name string) bool {
	cmd := exec.Command("/bin/sh", "-c", "command -v"+name)
	if err := cmd.Run(); err != nil {
		return false
	}
	return true
}
