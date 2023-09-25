/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2023. All rights reserved.
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
	"os"
	"testing"

	"github.com/agiledragon/gomonkey/v2"
	pb "openeuler.org/KubeOS/cmd/agent/api"
)

func Test_conImageHandler_downloadImage(t *testing.T) {
	type args struct {
		req *pb.UpdateRequest
	}
	tests := []struct {
		name    string
		c       conImageHandler
		args    args
		want    string
		wantErr bool
	}{
		{
			name: "pullImageError",
			c:    conImageHandler{},
			args: args{
				req: &pb.UpdateRequest{ContainerImage: "testError"},
			},
			want:    "",
			wantErr: true,
		},
		{
			name: "checkSumError",
			c:    conImageHandler{},
			args: args{
				req: &pb.UpdateRequest{ContainerImage: "docker.io/library/hello-world:latest"},
			},
			want:    "",
			wantErr: true,
		},
		{
			name: "normal",
			c:    conImageHandler{},
			args: args{
				req: &pb.UpdateRequest{
					ContainerImage: "docker.io/library/hello-world:latest",
				},
			},
			want:    "update-test1/upadte.img",
			wantErr: false,
		},
		{
			name: "invalid image name",
			c:    conImageHandler{},
			args: args{
				req: &pb.UpdateRequest{ContainerImage: "nginx;v1"},
			},
			want:    "",
			wantErr: true,
		},
	}
	patchPrepareEnv := gomonkey.ApplyFunc(prepareEnv, func() (preparePath, error) {
		return preparePath{updatePath: "update-test1/",
			mountPath:  "update-test1/mountPath",
			tarPath:    "update-test1/mountPath/hello",
			imagePath:  "update-test1/upadte.img",
			rootfsFile: "hello"}, nil
	})
	defer patchPrepareEnv.Reset()
	patchCreateOSImage := gomonkey.ApplyFunc(createOSImage, func(neededPath preparePath) (string, error) {
		return "update-test1/upadte.img", nil
	})
	defer patchCreateOSImage.Reset()

	if err := os.MkdirAll("update-test1/mountPath", os.ModePerm); err != nil {
		t.Errorf("create test dir error = %v", err)
		return
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			c := conImageHandler{}
			if tt.name == "normal" {
				imageDigests, err := getOCIImageDigest("crictl", "docker.io/library/hello-world:latest")
				if err != nil {
					t.Errorf("conImageHandler.getRootfsArchive() get oci image digests error = %v", err)
				}
				tt.args.req.CheckSum = imageDigests
			}
			got, err := c.downloadImage(tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("conImageHandler.downloadImage() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("conImageHandler.downloadImage() = %v, want %v", got, tt.want)
			}
		})
	}
	defer func() {
		if err := runCommand("crictl", "rmi", "docker.io/library/hello-world:latest"); err != nil {
			t.Errorf("remove kubeos-temp container error = %v", err)
		}
		if err := os.RemoveAll("update-test1"); err != nil {
			t.Errorf("remove update-test error = %v", err)
		}
	}()
}

func Test_copyFile(t *testing.T) {
	type args struct {
		dstFileName string
		srcFileName string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{
			name: "srcFileNotExist",
			args: args{
				dstFileName: "bbb.txt",
				srcFileName: "aaa.txt",
			},
			wantErr: true,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := copyFile(tt.args.dstFileName, tt.args.srcFileName); (err != nil) != tt.wantErr {
				t.Errorf("copyFile() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}
