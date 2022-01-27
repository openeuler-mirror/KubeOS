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
	"crypto/sha256"
	"crypto/tls"
	"crypto/x509"
	"encoding/hex"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/sirupsen/logrus"
	pb "openeuler.org/KubeOS/cmd/agent/api"
)

const (
	mainPart      = "/dev/sda2"
	partB         = "/dev/sda3"
	certPath      = "/etc/KubeOS/certs/"
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

	logrus.Infoln("start to update to imageURL " + req.ImageUrl)
	if err := s.update(req); err != nil {
		logrus.Errorln("update error " + err.Error())
		return &pb.UpdateResponse{}, err
	}

	return &pb.UpdateResponse{}, nil
}

func (s *Server) update(req *pb.UpdateRequest) error {
	imagePath, err := download(req)
	if err != nil {
		return err
	}
	if err = checkSumMatch(imagePath, req.CheckSum); err != nil {
		return err
	}
	if err = install(imagePath, mainPart, partB); err != nil {
		return err
	}
	return s.reboot()
}

func download(req *pb.UpdateRequest) (string, error) {
	resp, err := getImageURL(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("URL %s returns error %s", req.ImageUrl, resp.Status)
	}
	fs := syscall.Statfs_t{}
	if err = syscall.Statfs(PersistDir, &fs); err != nil {
		return "", err
	}
	if int64(fs.Bfree)*fs.Bsize < resp.ContentLength+buffer { // these data come from disk size, will not overflow
		return "", fmt.Errorf("space is not enough for downloaing")
	}

	out, err := os.Create(filepath.Join(PersistDir, "update.img"))
	if err != nil {
		return "", err
	}
	defer out.Close()
	err = os.Chmod(out.Name(), imgPermission)
	if err != nil {
		return "", err
	}
	logrus.Infoln("downloading to file " + out.Name())
	if _, err = io.Copy(out, resp.Body); err != nil {
		os.Remove(out.Name())
		return "", err
	}
	return out.Name(), nil
}

func checkSumMatch(filePath, checkSum string) error {
	file, err := os.Open(filePath)
	if err != nil {
		return err
	}
	defer file.Close()
	hash := sha256.New()
	if _, err := io.Copy(hash, file); err != nil {
		return err
	}
	if calSum := hex.EncodeToString(hash.Sum(nil)); calSum != checkSum {
		defer os.Remove(filePath)
		return fmt.Errorf("checkSum %s mismatch to %s", calSum, checkSum)
	}
	return nil
}

func install(imagePath string, mainPart string, partB string) error {
	out, err := exec.Command("lsblk", "-no", "MOUNTPOINT", mainPart).CombinedOutput()
	if err != nil {
		return fmt.Errorf("fail to lsblk %s out:%s err:%s", mainPart, out, err)
	}
	mountPoint := strings.TrimSpace(string(out))
	logrus.Infoln(mainPart + " mounted on " + mountPoint)

	side := mainPart
	if mountPoint == "/" {
		side = partB
	}
	logrus.Infoln("side is " + side)

	if err := runCommand("dd", "if="+imagePath, "of="+side, "bs=8M"); err != nil {
		return err
	}
	defer os.Remove(imagePath)

	next := "B"
	if side != partB {
		next = "A"
	}
	if err := runCommand("grub2-set-default", next); err != nil {
		return err
	}
	return runCommand("cp", "/boot/grub2/grubenv", "/boot/efi/EFI/openEuler")
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

func runCommand(name string, args ...string) error {
	out, err := exec.Command(name, args...).CombinedOutput()
	if err != nil {
		return fmt.Errorf("fail to run command:%s %v out:%s err:%s", name, args, out, err)
	}
	return nil
}

func getImageURL(req *pb.UpdateRequest) (*http.Response, error) {
	imageURL := req.ImageUrl
	flagSafe := req.FlagSafe
	mTLS := req.MTLS
	caCert := req.Certs.CaCaert
	clientCert := req.Certs.ClientCert
	clientKey := req.Certs.ClientKey

	if !strings.HasPrefix(imageURL, "https://") {
		if !flagSafe {
			logrus.Errorln("this imageUrl is not safe")
			return &http.Response{}, fmt.Errorf("this imageUrl is not safe")
		}
		resp, err := http.Get(imageURL)
		if err != nil {
			return &http.Response{}, err
		}
		return resp, nil
	} else if mTLS {
		client, err := loadClientCerts(caCert, clientCert, clientKey)
		if err != nil {
			return &http.Response{}, err
		}
		resp, err := client.Get(imageURL)
		if err != nil {
			return &http.Response{}, err
		}
		return resp, nil
	} else {
		client, err := loadCaCerts(caCert)
		if err != nil {
			return &http.Response{}, err
		}
		resp, err := client.Get(imageURL)
		if err != nil {
			return &http.Response{}, err
		}
		return resp, nil
	}

	return &http.Response{}, nil
}

func loadCaCerts(caCert string) (*http.Client, error) {
	pool := x509.NewCertPool()
	err := certExist(caCert)
	if err != nil {
		return &http.Client{}, err
	}
	ca, err := ioutil.ReadFile(certPath + caCert)
	if err != nil {
		return &http.Client{}, fmt.Errorf("read the ca certificate error ", err)
	}
	pool.AppendCertsFromPEM(ca)
	tr := &http.Transport{
		TLSClientConfig: &tls.Config{
			RootCAs: pool,
		},
	}
	client := &http.Client{Transport: tr}
	return client, nil
}

func loadClientCerts(caCert, clientCert, clientKey string) (*http.Client, error) {
	pool := x509.NewCertPool()
	err := certExist(caCert)
	if err != nil {
		return &http.Client{}, err
	}
	ca, err := ioutil.ReadFile(certPath + caCert)
	if err != nil {
		return &http.Client{}, err
	}
	pool.AppendCertsFromPEM(ca)
	err = certExist(clientCert)
	if err != nil {
		return &http.Client{}, err
	}
	err = certExist(clientKey)
	if err != nil {
		return &http.Client{}, err
	}
	cliCrt, err := tls.LoadX509KeyPair(certPath+clientCert, certPath+clientKey)
	if err != nil {
		return &http.Client{}, err
	}

	tr := &http.Transport{
		TLSClientConfig: &tls.Config{
			RootCAs:      pool,
			Certificates: []tls.Certificate{cliCrt},
		},
	}

	client := &http.Client{Transport: tr}
	return client, nil
}

func certExist(certFile string) error {
	if certFile == "" {
		return fmt.Errorf("please provide the certificate")
	}
	_, err := os.Stat(certPath + certFile)
	if err != nil {
		if os.IsNotExist(err) {
			return fmt.Errorf("certificate is not exist ", err)
		}
		return fmt.Errorf("certificate has an error ", err)
	}
	return nil
}
