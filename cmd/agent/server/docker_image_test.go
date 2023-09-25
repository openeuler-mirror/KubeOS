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
	"os"
	"testing"

	"github.com/agiledragon/gomonkey/v2"
	pb "openeuler.org/KubeOS/cmd/agent/api"
)

func Test_dockerImageHandler_downloadImage(t *testing.T) {
	type args struct {
		req *pb.UpdateRequest
	}
	tests := []struct {
		name    string
		d       dockerImageHandler
		args    args
		want    string
		wantErr bool
	}{
		{
			name: "pullImageError",
			d:    dockerImageHandler{},
			args: args{
				req: &pb.UpdateRequest{ContainerImage: "testError"},
			},
			want:    "",
			wantErr: true,
		},

		{
			name: "checkSumError",
			d:    dockerImageHandler{},
			args: args{
				req: &pb.UpdateRequest{ContainerImage: "hello-world", CheckSum: "aaaaaa"},
			},
			want:    "",
			wantErr: true,
		},

		{
			name: "normal",
			d:    dockerImageHandler{},
			args: args{
				req: &pb.UpdateRequest{ContainerImage: "hello-world"},
			},
			want:    "update-test/upadte.img",
			wantErr: false,
		},
	}
	patchPrepareEnv := gomonkey.ApplyFunc(prepareEnv, func() (preparePath, error) {
		return preparePath{updatePath: "update-test/",
			mountPath:  "update-test/mountPath",
			tarPath:    "update-test/mountPath/hello",
			imagePath:  "update-test/upadte.img",
			rootfsFile: "hello"}, nil
	})
	defer patchPrepareEnv.Reset()

	patchCreateOSImage := gomonkey.ApplyFunc(createOSImage, func(neededPath preparePath) (string, error) {
		return "update-test/upadte.img", nil
	})
	defer patchCreateOSImage.Reset()

	if err := os.MkdirAll("update-test/mountPath", os.ModePerm); err != nil {
		t.Errorf("create test dir error = %v", err)
		return
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.name == "normal" {
				_, err := runCommandWithOut("docker", "create", "--name", "kubeos-temp", "hello-world")
				if err != nil {
					t.Errorf("Test_dockerImageHandler_getRootfsArchive create container error = %v", err)
					return
				}
				imageDigests, err := getOCIImageDigest("docker", "hello-world")

				if err != nil {
					t.Errorf("Test_dockerImageHandler_getRootfsArchive get oci image digests error = %v", err)
				}
				tt.args.req.CheckSum = imageDigests
			}
			d := dockerImageHandler{}
			got, err := d.downloadImage(tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("dockerImageHandler.downloadImage() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("dockerImageHandler.downloadImage() = %v, want %v", got, tt.want)
			}
		})
	}
	defer func() {
		if err := runCommand("docker", "rmi", "hello-world"); err != nil {
			t.Errorf("remove kubeos-temp container error = %v", err)
		}
		if err := os.RemoveAll("update-test"); err != nil {
			t.Errorf("remove update-test error = %v", err)
		}
	}()
}
