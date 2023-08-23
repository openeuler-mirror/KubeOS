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
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/hex"
	"encoding/pem"
	"fmt"
	"io"
	"math/big"
	"net/http"
	"os"
	"reflect"
	"strings"
	"syscall"
	"testing"
	"time"

	"github.com/agiledragon/gomonkey/v2"
	pb "openeuler.org/KubeOS/cmd/agent/api"
)

func Test_download(t *testing.T) {
	tmpDir := t.TempDir()
	tmpFileForDownload := tmpDir + "/tmpFileForDownload"
	tmpFile, err := os.Create(tmpFileForDownload)
	if err != nil {
		t.Errorf("open file error: %v", err)
	}
	defer tmpFile.Close()
	type args struct {
		req *pb.UpdateRequest
	}
	tests := []struct {
		name    string
		args    args
		want    string
		wantErr bool
	}{
		{name: "errornil", args: args{&pb.UpdateRequest{Certs: &pb.CertsInfo{}}}, want: "", wantErr: true},
		{name: "error response", args: args{&pb.UpdateRequest{ImageUrl: "http://www.openeuler.abc", FlagSafe: true, Certs: &pb.CertsInfo{}}}, want: "", wantErr: true},
		{
			name: "normal",
			args: args{
				req: &pb.UpdateRequest{
					ImageUrl: "http://www.openeuler.org/zh/",
					FlagSafe: true,
					Certs:    &pb.CertsInfo{},
				},
			},
			want:    tmpFileForDownload,
			wantErr: false,
		},
		{
			name: "disk space not enough",
			args: args{
				req: &pb.UpdateRequest{
					ImageUrl: "http://www.openeuler.org/zh/",
					FlagSafe: true,
					Certs:    &pb.CertsInfo{},
				},
			},
			want:    "",
			wantErr: true,
		},
	}
	var patchStatfs *gomonkey.Patches
	patchStatfs = gomonkey.ApplyFunc(syscall.Statfs, func(path string, stat *syscall.Statfs_t) error {
		stat.Bfree = 3000
		stat.Bsize = 4096
		return nil
	})
	defer patchStatfs.Reset()
	patchGetImageUrl := gomonkey.ApplyFuncSeq(getImageURL,
		[]gomonkey.OutputCell{
			{Values: gomonkey.Params{&http.Response{}, fmt.Errorf("error")}},
			{Values: gomonkey.Params{&http.Response{StatusCode: http.StatusBadRequest, Body: io.NopCloser(strings.NewReader(""))}, nil}},
			{
				Values: gomonkey.Params{
					&http.Response{
						StatusCode:    http.StatusOK,
						ContentLength: 5,
						Body:          io.NopCloser(strings.NewReader("hello")),
					},
					nil,
				},
			},
			{
				Values: gomonkey.Params{
					&http.Response{
						StatusCode:    http.StatusOK,
						ContentLength: 5,
						Body:          io.NopCloser(strings.NewReader("hello")),
					},
					nil,
				},
			},
		},
	)
	defer patchGetImageUrl.Reset()
	patchOSCreate := gomonkey.ApplyFuncReturn(os.Create, tmpFile, nil)
	defer patchOSCreate.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.name == "disk space not enough" {
				patchStatfs = gomonkey.ApplyFunc(syscall.Statfs, func(path string, stat *syscall.Statfs_t) error {
					stat.Bfree = 1
					stat.Bsize = 4096
					return nil
				})
			}
			got, err := download(tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("download() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("download() got = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_checkSumMatch(t *testing.T) {
	tmpDir := t.TempDir()
	tmpFileForCheckSum := tmpDir + "/tmpFileForCheckSum"
	err := os.WriteFile(tmpFileForCheckSum, []byte("hello"), 0644)
	if err != nil {
		t.Errorf("open file error: %v", err)
	}
	type args struct {
		filePath string
		checkSum string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{
			name:    "normal",
			args:    args{filePath: tmpFileForCheckSum, checkSum: calculateChecksum("hello")},
			wantErr: false,
		},
		{name: "error", args: args{filePath: tmpFileForCheckSum, checkSum: "aaa"}, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := checkSumMatch(tt.args.filePath, tt.args.checkSum); (err != nil) != tt.wantErr {
				t.Errorf("checkSumMatch() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func Test_getImageURL(t *testing.T) {
	type args struct {
		req *pb.UpdateRequest
	}
	tests := []struct {
		name    string
		args    args
		want    *http.Response
		wantErr bool
	}{
		{name: "httpNotSafe", args: args{req: &pb.UpdateRequest{
			ImageUrl: "http://www.openeuler.abc/zh/",
			FlagSafe: false,
			MTLS:     false,
			Certs:    &pb.CertsInfo{},
		}}, want: &http.Response{}, wantErr: true},
		{name: "httpSuccess", args: args{req: &pb.UpdateRequest{
			ImageUrl: "http://www.openeuler.abc/zh/",
			FlagSafe: true,
			MTLS:     false,
			Certs:    &pb.CertsInfo{},
		}}, want: &http.Response{StatusCode: http.StatusOK}, wantErr: false},
		{name: "mTLSGetSuccess", args: args{req: &pb.UpdateRequest{
			ImageUrl: "https://www.openeuler.abc/zh/",
			FlagSafe: true,
			MTLS:     true,
			Certs:    &pb.CertsInfo{},
		}}, want: &http.Response{StatusCode: http.StatusOK}, wantErr: false},
		{name: "httpsGetSuccess", args: args{req: &pb.UpdateRequest{
			ImageUrl: "https://www.openeuler.abc/zh/",
			FlagSafe: true,
			MTLS:     false,
			Certs:    &pb.CertsInfo{},
		}}, want: &http.Response{StatusCode: http.StatusOK}, wantErr: false},
		{name: "httpsLoadCertsError", args: args{req: &pb.UpdateRequest{
			ImageUrl: "https://www.openeuler.abc/zh/",
			FlagSafe: true,
			MTLS:     false,
			Certs:    &pb.CertsInfo{},
		}}, want: &http.Response{}, wantErr: true},
		{name: "httpsMLTSLoadCertsError", args: args{req: &pb.UpdateRequest{
			ImageUrl: "https://www.openeuler.abc/zh/",
			FlagSafe: true,
			MTLS:     true,
			Certs:    &pb.CertsInfo{},
		}}, want: &http.Response{}, wantErr: true},
	}
	patchLoadClientCerts := gomonkey.ApplyFuncSeq(loadClientCerts, []gomonkey.OutputCell{
		{Values: gomonkey.Params{&http.Client{}, nil}},
		{Values: gomonkey.Params{&http.Client{}, fmt.Errorf("error")}},
	})
	defer patchLoadClientCerts.Reset()
	patchLoadCaCerts := gomonkey.ApplyFuncSeq(loadCaCerts, []gomonkey.OutputCell{
		{Values: gomonkey.Params{&http.Client{}, nil}},
		{Values: gomonkey.Params{&http.Client{}, fmt.Errorf("error")}},
	})
	defer patchLoadCaCerts.Reset()
	patchGet := gomonkey.ApplyFunc(http.Get, func(url string) (resp *http.Response, err error) {
		return &http.Response{StatusCode: http.StatusOK}, nil
	})
	defer patchGet.Reset()
	patchClientGet := gomonkey.ApplyMethod(reflect.TypeOf(&http.Client{}), "Get", func(_ *http.Client, url string) (resp *http.Response, err error) {
		return &http.Response{StatusCode: http.StatusOK}, nil
	})
	defer patchClientGet.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.name == "httpSuccess" {
				patchGet := gomonkey.ApplyFuncReturn(http.Get, &http.Response{StatusCode: http.StatusOK}, nil)
				defer patchGet.Reset()
			}
			got, err := getImageURL(tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("getImageURL() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("getImageURL() got = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_loadCaCerts(t *testing.T) {
	tmpDir := t.TempDir()
	caPath := tmpDir + "/fake.crt"
	createFakeCertKey(caPath, "")
	type args struct {
		caCert string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{
			name: "normal",
			args: args{
				caCert: caPath,
			},
			wantErr: false,
		},
	}
	patchGetCertPath := gomonkey.ApplyFuncReturn(getCertPath, "")
	defer patchGetCertPath.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := loadCaCerts(tt.args.caCert)
			if (err != nil) != tt.wantErr {
				t.Errorf("loadCaCerts() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got == nil {
				t.Errorf("loadCaCerts() = %v", got)
			}
		})
	}

}

func Test_loadClientCerts(t *testing.T) {
	tmpDir := t.TempDir()
	clientCertPath := tmpDir + "/fakeClientCert.crt"
	clientKeyPath := tmpDir + "/fakeClientKey.crt"
	createFakeCertKey(clientCertPath, clientKeyPath)
	type args struct {
		caCert     string
		clientCert string
		clientKey  string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{
			name: "normal",
			args: args{
				caCert: clientCertPath, clientCert: clientCertPath, clientKey: clientKeyPath,
			},
			wantErr: false,
		},
	}
	patchGetCertPath := gomonkey.ApplyFuncReturn(getCertPath, "")
	defer patchGetCertPath.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := loadClientCerts(tt.args.caCert, tt.args.clientCert, tt.args.clientKey)
			if (err != nil) != tt.wantErr {
				t.Errorf("loadClientCerts() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got == nil {
				t.Errorf("loadClientCerts() got = %v", got)
			}
		})
	}
}

func Test_certExist(t *testing.T) {
	type args struct {
		certFile string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{name: "fileEmpty", args: args{certFile: ""}, wantErr: true},
		{name: "fileNotExist", args: args{certFile: "bb.txt"}, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := certExist(tt.args.certFile); (err != nil) != tt.wantErr {
				t.Errorf("certExist() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
	defer os.RemoveAll("/etc/KubeOS/")
}

func createFakeCertKey(certPath, keyPath string) {
	privateKey, _ := rsa.GenerateKey(rand.Reader, 2048)
	template := x509.Certificate{
		SerialNumber: big.NewInt(1),
		Subject: pkix.Name{
			CommonName: "Fake Client Certificate",
		},
		NotBefore:             time.Now(),
		NotAfter:              time.Now().AddDate(1, 0, 0),
		KeyUsage:              x509.KeyUsageKeyEncipherment | x509.KeyUsageDigitalSignature,
		ExtKeyUsage:           []x509.ExtKeyUsage{x509.ExtKeyUsageClientAuth},
		BasicConstraintsValid: true,
	}
	certBytes, _ := x509.CreateCertificate(rand.Reader, &template, &template, &privateKey.PublicKey, privateKey)
	certPEM := pem.EncodeToMemory(&pem.Block{Type: "CERTIFICATE", Bytes: certBytes})
	keyPEM := pem.EncodeToMemory(&pem.Block{Type: "RSA PRIVATE KEY", Bytes: x509.MarshalPKCS1PrivateKey(privateKey)})
	os.WriteFile(certPath, certPEM, 0644)
	if keyPath != "" {
		os.WriteFile(keyPath, keyPEM, 0644)
	}
}

func calculateChecksum(data string) string {
	hash := sha256.New()
	hash.Write([]byte(data))
	return hex.EncodeToString(hash.Sum(nil))
}

func Test_diskHandler_getRootfsArchive(t *testing.T) {
	type args struct {
		req        *pb.UpdateRequest
		neededPath preparePath
	}
	tests := []struct {
		name    string
		d       diskHandler
		args    args
		want    string
		wantErr bool
	}{
		{
			name: "normal", d: diskHandler{},
			args:    args{req: &pb.UpdateRequest{ImageUrl: "http://www.openeuler.org/zh/"}, neededPath: preparePath{}},
			want:    "/persist/update.img",
			wantErr: false,
		},
	}
	patchDownload := gomonkey.ApplyFuncReturn(download, "/persist/update.img", nil)
	defer patchDownload.Reset()
	patchCheckSumMatch := gomonkey.ApplyFuncReturn(checkSumMatch, nil)
	defer patchCheckSumMatch.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			d := diskHandler{}
			got, err := d.getRootfsArchive(tt.args.req, tt.args.neededPath)
			if (err != nil) != tt.wantErr {
				t.Errorf("diskHandler.getRootfsArchive() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("diskHandler.getRootfsArchive() = %v, want %v", got, tt.want)
			}
		})
	}
}
