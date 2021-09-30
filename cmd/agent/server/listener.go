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

// Package server implements server of os-agent and listener of os-agent server. The server uses gRPC interface.
package server

import (
	"github.com/sirupsen/logrus"
	"net"
	"os"
	"path/filepath"
	"syscall"
)

// NewListener is used to listen the socket
func NewListener(dir, name string) (l net.Listener, err error) {
	if err := os.MkdirAll(dir, 0750); err != nil {
		return nil, err
	}

	addr := filepath.Join(dir, name)
	gid := os.Getgid()
	if err = syscall.Unlink(addr); err != nil && !os.IsNotExist(err) {
		return nil, err
	}

	const socketPermission = 0640
	mask := syscall.Umask(^socketPermission & int(os.ModePerm))
	defer syscall.Umask(mask)

	l, err = net.Listen("unix", addr)
	if err != nil {
		return nil, err
	}

	if err := os.Chown(addr, 0, gid); err != nil {
		if errClose := l.Close(); errClose != nil {
			logrus.Errorln("close listener error" + errClose.Error())
		}
		return nil, err
	}
	return l, nil
}
