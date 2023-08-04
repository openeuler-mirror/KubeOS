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
		{name: "normal", args: args{imagePath: "aa.txt", side: "/dev/sda3", next: "A"}, wantErr: false},
	}
	patchRunCommand := gomonkey.ApplyFuncReturn(runCommand, nil)
	defer patchRunCommand.Reset()
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
