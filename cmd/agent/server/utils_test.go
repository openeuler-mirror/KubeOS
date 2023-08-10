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
	"archive/tar"
	"os"
	"os/exec"
	"reflect"
	"testing"
	"time"

	"github.com/agiledragon/gomonkey/v2"
)

func Test_runCommand(t *testing.T) {
	type args struct {
		name string
		args []string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{name: "error", args: args{name: "/mmm", args: []string{"", ""}}, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := runCommand(tt.args.name, tt.args.args...); (err != nil) != tt.wantErr {
				t.Errorf("runCommand() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func Test_install(t *testing.T) {
	type args struct {
		imagePath string
		side      string
		next      string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{name: "normal uefi", args: args{imagePath: "aa.txt", side: "/dev/sda3", next: "A"}, wantErr: false},
		{name: "normal legacy", args: args{imagePath: "aa.txt", side: "/dev/sda3", next: "A"}, wantErr: false},
	}
	patchRunCommand := gomonkey.ApplyFuncReturn(runCommand, nil)
	defer patchRunCommand.Reset()
	patchGetBootMode := gomonkey.ApplyFuncSeq(getBootMode, []gomonkey.OutputCell{
		{Values: gomonkey.Params{"uefi", nil}},
		{Values: gomonkey.Params{"legacy", nil}},
	})
	defer patchGetBootMode.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := install(tt.args.imagePath, tt.args.side, tt.args.next); (err != nil) != tt.wantErr {
				t.Errorf("install() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func Test_getNextPart(t *testing.T) {
	type args struct {
		partA string
		partB string
	}
	tests := []struct {
		name    string
		args    args
		want    string
		want1   string
		wantErr bool
	}{
		{name: "switch to sda3", args: args{partA: "/dev/sda2", partB: "/dev/sda3"}, want: "/dev/sda3", want1: "B", wantErr: false},
		{name: "switch to sda2", args: args{partA: "/dev/sda2", partB: "/dev/sda3"}, want: "/dev/sda2", want1: "A", wantErr: false},
	}
	patchExecCommand := gomonkey.ApplyMethodSeq(&exec.Cmd{}, "CombinedOutput", []gomonkey.OutputCell{
		{Values: gomonkey.Params{[]byte("/"), nil}},
		{Values: gomonkey.Params{[]byte(""), nil}},
	})
	defer patchExecCommand.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, got1, err := getNextPart(tt.args.partA, tt.args.partB)
			if (err != nil) != tt.wantErr {
				t.Errorf("getNextPart() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("getNextPart() got = %v, want %v", got, tt.want)
			}
			if got1 != tt.want1 {
				t.Errorf("getNextPart() got1 = %v, want %v", got1, tt.want1)
			}
		})
	}
}

func Test_prepareEnv(t *testing.T) {
	mountPath := "/persist/KubeOS-Update/kubeos-update"
	if err := os.MkdirAll(mountPath, 0644); err != nil {
		t.Fatalf("mkdir err %v", err)
	}
	defer os.RemoveAll("/persist")
	tests := []struct {
		name    string
		want    preparePath
		wantErr bool
	}{
		{
			name: "success",
			want: preparePath{
				updatePath: "/persist/KubeOS-Update",
				mountPath:  "/persist/KubeOS-Update/kubeos-update",
				tarPath:    "/persist/KubeOS-Update/os.tar",
				imagePath:  "/persist/update.img",
				rootfsFile: "os.tar",
			},
			wantErr: false,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := prepareEnv()
			if (err != nil) != tt.wantErr {
				t.Errorf("prepareEnv() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("prepareEnv() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_createOSImage(t *testing.T) {
	mountPath := "/persist/KubeOS-Update/kubeos-update"
	if err := os.MkdirAll(mountPath, 0644); err != nil {
		t.Fatalf("mkdir err %v", err)
	}
	defer os.RemoveAll("/persist")
	tarPath := "/persist/KubeOS-Update/os.tar"
	path, err := createTmpTarFile(tarPath)
	if path != tarPath && err != nil {
		t.Fatalf("create temp zip file err %v", err)
	}
	type args struct {
		neededPath preparePath
	}
	tests := []struct {
		name    string
		args    args
		want    string
		wantErr bool
	}{
		{
			name: "normal",
			args: args{
				neededPath: preparePath{
					updatePath: "/persist/KubeOS-Update",
					mountPath:  "/persist/KubeOS-Update/kubeos-update",
					tarPath:    "/persist/KubeOS-Update/os.tar",
					imagePath:  "/persist/update.img",
				},
			},
			want:    "/persist/update.img",
			wantErr: false,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := createOSImage(tt.args.neededPath)
			if (err != nil) != tt.wantErr {
				t.Errorf("createOSImage() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("createOSImage() = %v, want %v", got, tt.want)
			}
		})
	}
}

func createTmpTarFile(tarPath string) (string, error) {
	tempFile, err := os.Create(tarPath)
	if err != nil {
		return "", err
	}
	defer tempFile.Close()

	tarWriter := tar.NewWriter(tempFile)
	fakeData := []byte("This is a fake file")
	fakeFile := "fakefile.txt"
	header := &tar.Header{
		Name:    fakeFile,
		Size:    int64(len(fakeData)),
		Mode:    0644,
		ModTime: time.Now(),
	}

	if err = tarWriter.WriteHeader(header); err != nil {
		return "", err
	}
	if _, err := tarWriter.Write(fakeData); err != nil {
		return "", err
	}
	if err := tarWriter.Flush(); err != nil {
		return "", err
	}
	return tempFile.Name(), nil
}

func Test_getBootMode(t *testing.T) {
	tests := []struct {
		name    string
		want    string
		wantErr bool
	}{
		{
			name:    "uefi",
			want:    "uefi",
			wantErr: false,
		},
		{
			name:    "legacy",
			want:    "legacy",
			wantErr: false,
		},
	}
	patchOSStat := gomonkey.ApplyFuncSeq(os.Stat, []gomonkey.OutputCell{
		{Values: gomonkey.Params{nil, nil}},
		{Values: gomonkey.Params{nil, os.ErrNotExist}},
	})
	defer patchOSStat.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := getBootMode()
			if (err != nil) != tt.wantErr {
				t.Errorf("getBootMode() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("getBootMode() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_isValidImageName(t *testing.T) {
	type args struct {
		image string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{name: "valid", args: args{image: "alpine"}, wantErr: false},
		{name: "valid", args: args{image: "alpine:latest"}, wantErr: false},
		{name: "valid", args: args{image: "localhost:1234/test"}, wantErr: false},
		{name: "valid", args: args{image: "alpine:3.7"}, wantErr: false},
		{name: "valid", args: args{image: "docker.example.edu/gmr/alpine:3.7"}, wantErr: false},
		{name: "valid", args: args{image: "docker.example.com:5000/gmr/alpine@sha256:11111111111111111111111111111111"}, wantErr: false},
		{name: "valid", args: args{image: "registry.dobby.org/dobby/dobby-servers/arthound:2019-08-08"}, wantErr: false},
		{name: "valid", args: args{image: "registry.dobby.org/dobby/dobby-servers/lerphound:latest"}, wantErr: false},
		{name: "valid", args: args{image: "registry.dobby.org/dobby/dobby-servers/loophole@sha256:5a156ff125e5a12ac7ff43ee5120fa249cf62248337b6d04abc574c8"}, wantErr: false},
		{name: "valid", args: args{image: "sosedoff/pgweb@sha256:5a156ff125e5a12ac7ff43ee5120fa249cf62248337b6d04574c8"}, wantErr: false},
		{name: "valid", args: args{image: "registry.dobby.org/dobby/antique-penguin:release-production"}, wantErr: false},
		{name: "valid", args: args{image: "dalprodictus/halcon:6.7.5"}, wantErr: false},
		{name: "valid", args: args{image: "antigua/antigua:v31"}, wantErr: false},
		{name: "invalid ;", args: args{image: "alpine;v1.0"}, wantErr: true},
		{name: "invalid tag and digest1", args: args{image: "alpine:latest@sha256:11111111111111111111111111111111"}, wantErr: true},
		{name: "invalid |", args: args{image: "alpine|v1.0"}, wantErr: true},
		{name: "invalid &", args: args{image: "alpine&v1.0"}, wantErr: true},
		{name: "invalid tag and digest2", args: args{image: "sosedoff/pgweb:latest@sha256:5a156ff125e5a12ac7ff43ee5120fa249cf62248337b6d04574c8"}, wantErr: true},
		{name: "invalid tag and digest3", args: args{image: "192.168.122.123:5000/kubeos_uefi-x86_64:euleros_v2_docker-2023-01@sha256:1a1a1a1a1a1a1a1a1a1a1a1a1a1a"}, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := isValidImageName(tt.args.image); (err != nil) != tt.wantErr {
				t.Errorf("isValidImageName() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func Test_checkOCIImageDigestMatch(t *testing.T) {
	type args struct {
		containerRuntime string
		imageName        string
		checkSum         string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{name: "invalid container runtion", args: args{containerRuntime: "dockctl", imageName: "docker.io/library/hello-world:latest", checkSum: "1abf18abf9bf9baa0a4a38d1afad4abf0d7da4544e163186e036c906c09c94fe"}, wantErr: true},
		{name: "nil image digets", args: args{containerRuntime: "crictl", imageName: "docker.io/library/hello-world:latest", checkSum: "1abf18abf9bf9baa0a4a38d1afad4abf0d7da4544e163186e036c906c09c94fe"}, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.name == "nil image digets" {
				patchGetOCIImageDigest := gomonkey.ApplyFuncReturn(getOCIImageDigest, "", nil)
				defer patchGetOCIImageDigest.Reset()
			}
			if err := checkOCIImageDigestMatch(tt.args.containerRuntime, tt.args.imageName, tt.args.checkSum); (err != nil) != tt.wantErr {
				t.Errorf("checkOCIImageDigestMatch() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}
