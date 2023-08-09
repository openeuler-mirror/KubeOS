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
	"os"
	"reflect"
	"testing"

	"github.com/agiledragon/gomonkey/v2"

	pb "openeuler.org/KubeOS/cmd/agent/api"
)

func TestLockTryLock(t *testing.T) {
	type fields struct {
		state uint32
	}
	tests := []struct {
		name   string
		fields fields
		want   bool
	}{
		{name: "normal", fields: fields{state: 0}, want: true},
		{name: "error", fields: fields{state: 1}, want: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			l := &Lock{
				state: tt.fields.state,
			}
			if got := l.TryLock(); got != tt.want {
				t.Errorf("Lock.TryLock() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestLockUnlock(t *testing.T) {
	type fields struct {
		state uint32
	}
	tests := []struct {
		name   string
		fields fields
	}{
		{name: "normal", fields: fields{state: 1}},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			l := &Lock{
				state: tt.fields.state,
			}
			l.Unlock()
		})
	}
}

func TestServerUpdate(t *testing.T) {
	type fields struct {
		UnimplementedOSServer pb.UnimplementedOSServer
		mutex                 Lock
		disableReboot         bool
	}
	type args struct {
		in0 context.Context
		req *pb.UpdateRequest
	}
	tests := []struct {
		name    string
		fields  fields
		args    args
		want    *pb.UpdateResponse
		wantErr bool
	}{
		{name: "error", fields: fields{UnimplementedOSServer: pb.UnimplementedOSServer{}, disableReboot: true},
			args: args{in0: context.Background(), req: &pb.UpdateRequest{Version: "test", Certs: &pb.CertsInfo{}}},
			want: &pb.UpdateResponse{}, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			s := &Server{
				UnimplementedOSServer: tt.fields.UnimplementedOSServer,
				mutex:                 tt.fields.mutex,
				disableReboot:         tt.fields.disableReboot,
			}
			got, err := s.Update(tt.args.in0, tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("Server.Update() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("Server.Update() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestServerRollback(t *testing.T) {
	type fields struct {
		UnimplementedOSServer pb.UnimplementedOSServer
		mutex                 Lock
		disableReboot         bool
	}
	type args struct {
		in0 context.Context
		req *pb.RollbackRequest
	}
	tests := []struct {
		name    string
		fields  fields
		args    args
		want    *pb.RollbackResponse
		wantErr bool
	}{
		{name: "error", fields: fields{UnimplementedOSServer: pb.UnimplementedOSServer{}, disableReboot: true},
			args: args{in0: context.Background(), req: &pb.RollbackRequest{}},
			want: &pb.RollbackResponse{}, wantErr: true},
	}
	patchGetNextPart := gomonkey.ApplyFunc(getNextPart, func(partA string, partB string) (string, string, error) {
		return "", "", fmt.Errorf("rollbak test error")
	})
	defer patchGetNextPart.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			s := &Server{
				UnimplementedOSServer: tt.fields.UnimplementedOSServer,
				mutex:                 tt.fields.mutex,
				disableReboot:         tt.fields.disableReboot,
			}
			got, err := s.Rollback(tt.args.in0, tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("Server.Rollback() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("Server.Rollback() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestServerupdate(t *testing.T) {
	type fields struct {
		UnimplementedOSServer pb.UnimplementedOSServer
		mutex                 Lock
		disableReboot         bool
	}
	type args struct {
		req *pb.UpdateRequest
	}
	tests := []struct {
		name    string
		fields  fields
		args    args
		wantErr bool
	}{
		{name: "errortype", args: args{&pb.UpdateRequest{Certs: &pb.CertsInfo{}}}, wantErr: true},

		{name: "errordisk", args: args{&pb.UpdateRequest{
			ImageUrl:  "http://w3.huawei.com/",
			FlagSafe:  true,
			CheckSum:  "",
			ImageType: "disk",
			Certs:     &pb.CertsInfo{},
		}},
			wantErr: true},
		{name: "errordocker", args: args{&pb.UpdateRequest{
			ContainerImage: "",
			ImageType:      "docker",
			Certs:          &pb.CertsInfo{},
		}},
			wantErr: true},
	}
	for _, tt := range tests {
		if tt.name == "errordisk" {
			os.Mkdir("/persist", os.ModePerm)
		}
		t.Run(tt.name, func(t *testing.T) {
			s := &Server{
				UnimplementedOSServer: tt.fields.UnimplementedOSServer,
				mutex:                 tt.fields.mutex,
				disableReboot:         tt.fields.disableReboot,
			}
			if err := s.update(tt.args.req); (err != nil) != tt.wantErr {
				t.Errorf("Server.update() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
		if tt.name == "errordisk" {
			os.RemoveAll("/persist")
		}
	}
}

func TestServerrollback(t *testing.T) {
	type fields struct {
		UnimplementedOSServer pb.UnimplementedOSServer
		mutex                 Lock
		disableReboot         bool
	}
	tests := []struct {
		name    string
		fields  fields
		wantErr bool
	}{
		{name: "error", fields: fields{UnimplementedOSServer: pb.UnimplementedOSServer{}, disableReboot: true},
			wantErr: true},
	}
	patchGetNextPart := gomonkey.ApplyFunc(getNextPart, func(partA string, partB string) (string, string, error) {
		return "", "", fmt.Errorf("rollbak test error")
	})
	defer patchGetNextPart.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			s := &Server{
				UnimplementedOSServer: tt.fields.UnimplementedOSServer,
				mutex:                 tt.fields.mutex,
				disableReboot:         tt.fields.disableReboot,
			}
			if err := s.rollback(); (err != nil) != tt.wantErr {
				t.Errorf("Server.rollback() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestServerreboot(t *testing.T) {
	type fields struct {
		UnimplementedOSServer pb.UnimplementedOSServer
		mutex                 Lock
		disableReboot         bool
	}
	tests := []struct {
		name    string
		fields  fields
		wantErr bool
	}{
		{name: "normal", fields: fields{UnimplementedOSServer: pb.UnimplementedOSServer{}, disableReboot: true},
			wantErr: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			s := &Server{
				UnimplementedOSServer: tt.fields.UnimplementedOSServer,
				mutex:                 tt.fields.mutex,
				disableReboot:         tt.fields.disableReboot,
			}
			if err := s.reboot(); (err != nil) != tt.wantErr {
				t.Errorf("Server.reboot() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}
