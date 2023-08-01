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
	"context"
	"fmt"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/sirupsen/logrus"

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

const (
	certPath      = "/etc/KubeOS/certs/"
	grubenvPath   = "/boot/efi/EFI/openEuler/grubenv"
	locked        = 1
	unLocked      = 0
	buffer        = 1024 * 10240
	imgPermission = 0600
)

// Lock is a custom Lock to implement a spin lock
type Lock struct {
	state uint32
}

// TryLock acquires the lock. On success returns true. On failure return false.
func (l *Lock) TryLock() bool {
	return atomic.CompareAndSwapUint32(&l.state, unLocked, locked)
}

// Unlock unlocks for lock.
func (l *Lock) Unlock() {
	atomic.StoreUint32(&l.state, unLocked)
}

// Server implements the OSServer
type Server struct {
	pb.UnimplementedOSServer
	mutex         Lock
	disableReboot bool
}

// Update implements the OSServer.Update
func (s *Server) Update(_ context.Context, req *pb.UpdateRequest) (*pb.UpdateResponse, error) {
	if !s.mutex.TryLock() {
		return &pb.UpdateResponse{}, fmt.Errorf("server is processing another request")
	}
	defer s.mutex.Unlock()

	logrus.Infoln("start to update to " + req.Version)
	if err := s.update(req); err != nil {
		logrus.Errorln("update error " + err.Error())
		return &pb.UpdateResponse{}, err
	}

	return &pb.UpdateResponse{}, nil
}

// Rollback implements the OSServer.Rollback
func (s *Server) Rollback(_ context.Context, req *pb.RollbackRequest) (*pb.RollbackResponse, error) {
	if !s.mutex.TryLock() {
		return &pb.RollbackResponse{}, fmt.Errorf("server is processing another request")
	}
	defer s.mutex.Unlock()

	logrus.Infoln("start to rollback ")
	if err := s.rollback(); err != nil {
		return &pb.RollbackResponse{}, err
	}
	return &pb.RollbackResponse{}, nil
}

// Configure implements the OSServer.Configure
func (s *Server) Configure(_ context.Context, req *pb.ConfigureRequest) (*pb.ConfigureResponse, error) {
	if !s.mutex.TryLock() {
		return &pb.ConfigureResponse{}, fmt.Errorf("server is processing another request")
	}
	defer s.mutex.Unlock()
	if err := s.configure(req); err != nil {
		return &pb.ConfigureResponse{}, err
	}
	return &pb.ConfigureResponse{}, nil
}

func (s *Server) update(req *pb.UpdateRequest) error {
	action := req.ImageType
	var handler imageDownload
	switch action {
	case "docker":
		handler = dockerImageHandler{}
	case "containerd":
		handler = conImageHandler{}
	case "disk":
		handler = diskHandler{}
	default:
		return fmt.Errorf("image type %s cannot be recognized", action)
	}
	imagePath, err := handler.downloadImage(req)
	if err != nil {
		return err
	}
	partA, partB, err := getRootfsDisks()
	if err != nil {
		return err
	}
	side, next, err := getNextPart(partA, partB)
	logrus.Infoln("switching to " + side + " partition " + next)
	if err != nil {
		return err
	}
	if err = install(imagePath, side, next); err != nil {
		return err
	}
	return s.reboot()
}

func (s *Server) rollback() error {
	partA, partB, err := getRootfsDisks()
	if err != nil {
		return err
	}
	_, next, err := getNextPart(partA, partB)
	if err != nil {
		return err
	}
	bootMode, err := getBootMode()
	if err != nil {
		return err
	}
	if bootMode == "uefi" {
		if err = runCommand("grub2-editenv", grubenvPath, "set", "saved_entry="+next); err != nil {
			return err
		}
	} else {
		if err = runCommand("grub2-set-default", next); err != nil {
			return err
		}
	}
	return s.reboot()
}

func (s *Server) configure(req *pb.ConfigureRequest) error {
	if err := startConfig(req.Configs); err != nil {
		return err
	}
	return nil
}

func (s *Server) reboot() error {
	logrus.Infoln("wait to reboot")
	time.Sleep(time.Second)
	syscall.Sync()
	if s.disableReboot {
		return nil
	}
	return syscall.Reboot(syscall.LINUX_REBOOT_CMD_RESTART)
}
