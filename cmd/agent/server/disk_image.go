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
	"crypto/sha256"
	"crypto/tls"
	"crypto/x509"
	"encoding/hex"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/sirupsen/logrus"

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

type diskHandler struct{}

func (d diskHandler) downloadImage(req *pb.UpdateRequest) (string, error) {
	imagePath, err := d.getRootfsArchive(req, preparePath{})
	if err != nil {
		return "", err
	}
	return imagePath, nil
}

func (d diskHandler) getRootfsArchive(req *pb.UpdateRequest, neededPath preparePath) (string, error) {
	imagePath, err := download(req)
	if err != nil {
		return "", err
	}
	if err = checkSumMatch(imagePath, req.CheckSum); err != nil {
		return "", err
	}
	return imagePath, nil
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
		if errRemove := os.Remove(out.Name()); errRemove != nil {
			logrus.Errorln("remove " + out.Name() + " error " + errRemove.Error())
		}
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
}

func loadCaCerts(caCert string) (*http.Client, error) {
	pool := x509.NewCertPool()
	err := certExist(caCert)
	if err != nil {
		return &http.Client{}, err
	}
	ca, err := ioutil.ReadFile(getCertPath() + caCert)
	if err != nil {
		return &http.Client{}, fmt.Errorf("read the ca certificate error %s", err)
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
	ca, err := ioutil.ReadFile(getCertPath() + caCert)
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
	cliCrt, err := tls.LoadX509KeyPair(getCertPath()+clientCert, getCertPath()+clientKey)
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
	_, err := os.Stat(getCertPath() + certFile)
	if err != nil {
		if os.IsNotExist(err) {
			return fmt.Errorf("certificate is not exist %s ", err)
		}
		return fmt.Errorf("certificate has an error %s", err)
	}
	return nil
}
