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
	"os/exec"
	"strings"
	"testing"
)

func TestrunCommand(t *testing.T) {
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

func Testinstall(t *testing.T) {
	type args struct {
		imagePath string
		side      string
		next      string
	}
	out, _ := exec.Command("bash", "-c", "df -h | grep '/$' | awk '{print $1}'").CombinedOutput()
	mountPart := strings.TrimSpace(string(out))
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{name: "normal", args: args{imagePath: "aa.txt", side: mountPart, next: ""}, wantErr: false},
	}
	ff, _ := os.Create("aa.txt")
	ff.Chmod(os.ModePerm)
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := install(tt.args.imagePath, tt.args.side, tt.args.next); (err != nil) != tt.wantErr {
				t.Errorf("install() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
	ff.Close()
	defer os.Remove("aa.txt")
}

func TestgetNextPart(t *testing.T) {
	type args struct {
		partA string
		partB string
	}
	out, _ := exec.Command("bash", "-c", "df -h | grep '/$' | awk '{print $1}'").CombinedOutput()
	mountPart := strings.TrimSpace(string(out))
	tests := []struct {
		name    string
		args    args
		want    string
		want1   string
		wantErr bool
	}{
		{name: "normal", args: args{partA: mountPart, partB: "testB"}, want: "testB", want1: "B", wantErr: false},
	}
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
