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
	"crypto/tls"
	"crypto/x509"
	"net/http"
	"os"
	"reflect"
	"testing"

	"github.com/agiledragon/gomonkey/v2"

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

func Testdownload(t *testing.T) {
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
		{name: "normal", args: args{&pb.UpdateRequest{ImageUrl: "http://www.openeuler.org/zh/", FlagSafe: true, Certs: &pb.CertsInfo{}}}, want: "/persist/update.img", wantErr: false},
		{name: "errornodir", args: args{&pb.UpdateRequest{ImageUrl: "http://www.openeuler.org/zh/", FlagSafe: true, Certs: &pb.CertsInfo{}}}, want: "", wantErr: true},
	}
	for _, tt := range tests {
		if tt.name == "normal" {
			os.Mkdir("/persist", os.ModePerm)
		}
		t.Run(tt.name, func(t *testing.T) {
			got, err := download(tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("download() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("download() got = %v, want %v", got, tt.want)
			}
		})
		if tt.name == "normal" {
			os.RemoveAll("/persist")
		}
	}
}

func TestcheckSumMatch(t *testing.T) {
	type args struct {
		filePath string
		checkSum string
	}
	ff, _ := os.Create("aa.txt")
	ff.Chmod(os.ModePerm)
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{name: "error", args: args{filePath: "aaa", checkSum: "aaa"}, wantErr: true},
		{name: "errordir", args: args{filePath: "/aaa", checkSum: "/aaa"}, wantErr: true},
		{name: "errortxt", args: args{filePath: "aa.txt", checkSum: "aa.txt"}, wantErr: true},
	}
	for _, tt := range tests {
		if tt.name == "errordir" {
			os.Mkdir("/aaa", os.ModePerm)
		}
		t.Run(tt.name, func(t *testing.T) {
			if err := checkSumMatch(tt.args.filePath, tt.args.checkSum); (err != nil) != tt.wantErr {
				t.Errorf("checkSumMatch() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
		if tt.name == "errordir" {
			os.RemoveAll("/aaa")
		}
	}
	defer os.Remove("aa.txt")
	defer ff.Close()

}

func TestgetImageURL(t *testing.T) {
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
			ImageUrl: "http://www.openeuler.org/zh/",
			FlagSafe: false,
			MTLS:     false,
			Certs:    &pb.CertsInfo{},
		}}, want: &http.Response{}, wantErr: true},
		{name: "mTLSError", args: args{req: &pb.UpdateRequest{
			ImageUrl: "http://www.openeuler.org/zh/",
			FlagSafe: true,
			MTLS:     true,
			Certs:    &pb.CertsInfo{},
		}}, want: &http.Response{}, wantErr: true},
		{name: "httpsError", args: args{req: &pb.UpdateRequest{
			ImageUrl: "https://www.openeuler.org/zh/",
			FlagSafe: true,
			MTLS:     false,
			Certs:    &pb.CertsInfo{},
		}}, want: &http.Response{}, wantErr: true},
	}
	patchLoadClientCerts := gomonkey.ApplyFunc(loadClientCerts, func(caCert, clientCert, clientKey string) (*http.Client, error) {
		return &http.Client{}, nil
	})
	defer patchLoadClientCerts.Reset()
	patchLoadCaCerts := gomonkey.ApplyFunc(loadCaCerts, func(caCert string) (*http.Client, error) {
		return &http.Client{}, nil
	})
	defer patchLoadCaCerts.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
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

func TestloadCaCerts(t *testing.T) {
	type args struct {
		caCert string
	}
	tests := []struct {
		name    string
		args    args
		want    *http.Client
		wantErr bool
	}{
		{name: "noCaCertError", args: args{caCert: "bb.txt"}, want: &http.Client{}, wantErr: true},
	}
	os.MkdirAll(certPath, 0644)
	defer os.RemoveAll(certPath)
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := loadCaCerts(tt.args.caCert)
			if (err != nil) != tt.wantErr {
				t.Errorf("loadCaCerts() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("loadCaCerts() = %v, want %v", got, tt.want)
			}
		})
	}

}

func TestloadClientCerts(t *testing.T) {
	type args struct {
		caCert     string
		clientCert string
		clientKey  string
	}
	pool := &x509.CertPool{}
	tests := []struct {
		name    string
		args    args
		want    *http.Client
		wantErr bool
	}{
		{name: "noCaCertError", args: args{" dd.txt", "bb.txt", "cc.txt"}, want: &http.Client{}, wantErr: true},
		{name: "noClientCertError", args: args{"ca.crt", "bb.txt", "cc.txt"}, want: &http.Client{}, wantErr: true},
		{name: "noClientKeyError", args: args{"ca.crt", "client.crt", "cc.txt"}, want: &http.Client{}, wantErr: true},
	}
	os.MkdirAll(certPath, 0644)
	caFile, _ := os.Create(certPath + "ca.crt")
	clientCertFile, _ := os.Create(certPath + "client.crt")
	clientKeyFile, _ := os.Create(certPath + "client.key")

	patchNewCertPool := gomonkey.ApplyFunc(x509.NewCertPool, func() *x509.CertPool {
		return pool
	})
	defer patchNewCertPool.Reset()
	patchAppendCertsFromPEM := gomonkey.ApplyMethod(reflect.TypeOf(pool), "AppendCertsFromPEM", func(_ *x509.CertPool, _ []byte) (ok bool) {
		return true
	})
	defer patchAppendCertsFromPEM.Reset()
	patchLoadX509KeyPair := gomonkey.ApplyFunc(tls.LoadX509KeyPair, func(certFile string, keyFile string) (tls.Certificate, error) {
		return tls.Certificate{}, nil
	})
	defer patchLoadX509KeyPair.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := loadClientCerts(tt.args.caCert, tt.args.clientCert, tt.args.clientKey)
			if (err != nil) != tt.wantErr {
				t.Errorf("loadClientCerts() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("loadClientCerts() got = %v, want %v", got, tt.want)
			}
		})
	}
	caFile.Close()
	clientCertFile.Close()
	clientKeyFile.Close()
	defer os.RemoveAll("/etc/KubeOS")

}

func TestcertExist(t *testing.T) {
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
		{name: "normal", args: args{certFile: "aa.txt"}, wantErr: false},
	}
	os.MkdirAll(certPath, 0644)
	ff, _ := os.Create(certPath + "aa.txt")
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := certExist(tt.args.certFile); (err != nil) != tt.wantErr {
				t.Errorf("certExist() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
	ff.Close()
	defer os.RemoveAll("/etc/KubeOS/")
}
