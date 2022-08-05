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
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/sirupsen/logrus"
)

func runCommand(name string, args ...string) error {
	out, err := exec.Command(name, args...).CombinedOutput()
	if err != nil {
		return fmt.Errorf("fail to run command:%s %v out:%s err:%s", name, args, out, err)
	}
	return nil
}

func install(imagePath string, side string, next string) error {
	if err := runCommand("dd", "if="+imagePath, "of="+side, "bs=8M"); err != nil {
		return err
	}
	defer os.Remove(imagePath)
	return runCommand("grub2-editenv", grubenvPath, "set", "saved_entry="+next)
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
