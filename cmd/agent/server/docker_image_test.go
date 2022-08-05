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

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

func TestpullOSImage(t *testing.T) {
	type args struct {
		req *pb.UpdateRequest
	}
	os.Mkdir("/persist", os.ModePerm)
	tests := []struct {
		name    string
		args    args
		want    string
		wantErr bool
	}{
		{name: "pull image error", args: args{req: &pb.UpdateRequest{
			DockerImage: "test",
		}}, want: "", wantErr: true},
		{name: "normal", args: args{req: &pb.UpdateRequest{
			DockerImage: "centos",
		}}, want: "/persist/update.img", wantErr: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := pullOSImage(tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("pullOSImage() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("pullOSImage() = %v, want %v", got, tt.want)
			}
		})
	}
	defer os.RemoveAll("/persist")
}

func TestrandStringBytesRmndr(t *testing.T) {
	type args struct {
		n int
	}
	tests := []struct {
		name string
		args args
		want string
	}{
		{name: "normal", args: args{n: 6}, want: ""},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := randStringBytesRmndr(tt.args.n); got == "" {
				t.Errorf("randStringBytesRmndr() not generatre random string")
			}

		})
	}
}
