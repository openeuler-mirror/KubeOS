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
	"net"
	"os"
	"testing"
)

func TestNewListener(t *testing.T) {
	type args struct {
		dir  string
		name string
	}
	tests := []struct {
		name    string
		args    args
		wantL   net.Listener
		wantErr bool
	}{
		{name: "errordir", args: args{dir: "", name: "aaa"}, wantErr: true},
		{name: "normal", args: args{dir: "mmm", name: "aaa"}, wantErr: false},
		{name: "errorname", args: args{dir: "mmm", name: ""}, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := NewListener(tt.args.dir, tt.args.name)
			if (err != nil) != tt.wantErr {
				t.Errorf("NewListener() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
		})
	}
	if err := os.RemoveAll("mmm"); err != nil {
		t.Errorf("remove mmm error %s", err)
	}
}
