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
	"fmt"
	"os"
	"reflect"
	"regexp"
	"sort"
	"strings"
	"testing"

	"github.com/agiledragon/gomonkey/v2"

	agent "openeuler.org/KubeOS/cmd/agent/api"
)

func TestKernelSysctl_SetConfig(t *testing.T) {
	type args struct {
		config *agent.SysConfig
	}
	tests := []struct {
		name    string
		k       KernelSysctl
		args    args
		wantErr bool
	}{
		{
			name: "add configs",
			k:    KernelSysctl{},
			args: args{config: &agent.SysConfig{
				Contents: map[string]*agent.KeyInfo{
					"a": {Value: "1"},
					"b": {Value: "2"},
				},
			}},
			wantErr: false,
		},
		{
			name: "delete",
			k:    KernelSysctl{},
			args: args{config: &agent.SysConfig{
				Contents: map[string]*agent.KeyInfo{
					"a": {Operation: "delete"},
				},
			}},
			wantErr: false,
		},
		{
			name: "invalide operation",
			k:    KernelSysctl{},
			args: args{config: &agent.SysConfig{
				Contents: map[string]*agent.KeyInfo{
					"c": {Operation: "ad"},
				},
			}},
		},
	}
	tmpDir := t.TempDir()
	patchGetProcPath := gomonkey.ApplyFuncReturn(getDefaultProcPath, tmpDir+"/")
	defer patchGetProcPath.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			k := KernelSysctl{}
			if err := k.SetConfig(tt.args.config); (err != nil) != tt.wantErr {
				t.Errorf("KernelSysctl.SetConfig() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestKerSysctlPersist_SetConfig(t *testing.T) {
	tmpDir := t.TempDir()
	persistPath := tmpDir + "/test-persist.conf"
	type args struct {
		config *agent.SysConfig
	}
	tests := []struct {
		name    string
		k       KerSysctlPersist
		args    args
		want    []string
		wantErr bool
	}{
		{
			name: "add configs",
			args: args{
				config: &agent.SysConfig{
					ConfigPath: persistPath,
					Contents: map[string]*agent.KeyInfo{
						"a": {Value: "1"},
						"b": {Value: "2"},
					},
				},
			},
			want: []string{
				"a = 1",
				"b = 2",
			},
			wantErr: false,
		},
		{
			name: "update",
			args: args{
				config: &agent.SysConfig{
					ConfigPath: persistPath,
					Contents: map[string]*agent.KeyInfo{
						"a": {Value: "2"},
					},
				},
			},
			want: []string{
				"a = 2",
				"b = 2",
			},
			wantErr: false,
		},
		{
			name: "delete",
			args: args{
				config: &agent.SysConfig{
					ConfigPath: persistPath,
					Contents: map[string]*agent.KeyInfo{
						"a": {Value: "1", Operation: "delete"},
						"b": {Value: "2", Operation: "delete"},
					},
				},
			},
			want: []string{
				"a = 2",
			},
			wantErr: false,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			k := KerSysctlPersist{}
			if err := k.SetConfig(tt.args.config); (err != nil) != tt.wantErr {
				t.Errorf("KerSysctlPersist.SetConfig() error = %v, wantErr %v", err, tt.wantErr)
			}
			data, err := os.ReadFile(persistPath)
			if err != nil {
				t.Errorf("failed to read file %s", persistPath)
			}
			lines := strings.Split(string(data), "\n")
			// remove the last empty line
			lines = lines[:len(lines)-1]
			sort.Strings(lines)
			if !reflect.DeepEqual(lines, tt.want) {
				t.Errorf("KerSysctlPersist file contents not equal, expect: %v, get: %v", tt.want, lines)
			}
		})
	}
}

func TestGrubCmdline_SetConfig(t *testing.T) {
	grubContent := `menuentry 'A' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-A' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        set root='hd0,gpt2'
        linux   /boot/vmlinuz root=UUID=0 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3
        initrd  /boot/initramfs.img
}

menuentry 'B' --class KubeOS --class gnu-linux --class gnu --class os --unrestricted $menuentry_id_option 'KubeOS-B' {
        load_video
        set gfxpayload=keep
        insmod gzio
        insmod part_gpt
        insmod ext2
        set root='hd0,gpt3'
        linux   /boot/vmlinuz root=UUID=1 ro rootfstype=ext4 nomodeset quiet oops=panic softlockup_panic=1 nmi_watchdog=1 rd.shell=0 selinux=0 crashkernel=256M panic=3
        initrd  /boot/initramfs.img
}`
	tmpDir := t.TempDir()
	grubCfgPath := tmpDir + "/grub.cfg"
	if err := copyGrub(grubContent, grubCfgPath); err != nil {
		t.Fatalf("failed to copy grub file %v", err)
	}
	type args struct {
		config *agent.SysConfig
	}
	tests := []struct {
		name    string
		g       GrubCmdline
		args    args
		pattern string
		wantErr bool
	}{
		{
			name: "add, update and delete kernel boot parameters",
			g:    GrubCmdline{},
			args: args{
				config: &agent.SysConfig{
					Contents: map[string]*agent.KeyInfo{
						"panic":   {Value: "5"},
						"quiet":   {Value: "", Operation: "delete"},
						"selinux": {Value: "1", Operation: "delete"},
						"acpi":    {Value: "off", Operation: "delete"},
						"debug":   {},
						"pci":     {Value: "nomis"},
					},
				},
			},
			pattern: `(?m)^\s+linux\s+\/boot\/vmlinuz\s+root=UUID=[0-1]\s+ro\s+rootfstype=ext4\s+nomodeset\s+oops=panic\s+softlockup_panic=1\s+nmi_watchdog=1\s+rd\.shell=0\s+selinux=0\s+crashkernel=256M\s+panic=5\s+(debug\spci=nomis|pci=nomis\sdebug)$`,
			wantErr: false,
		},
	}
	patchGetGrubPath := gomonkey.ApplyFuncReturn(getGrubCfgPath, grubCfgPath)
	defer patchGetGrubPath.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			g := GrubCmdline{}
			if err := g.SetConfig(tt.args.config); (err != nil) != tt.wantErr {
				t.Errorf("GrubCmdline.SetConfig() error = %v, wantErr %v", err, tt.wantErr)
			}
			contents, err := os.ReadFile(grubCfgPath)
			if err != nil {
				t.Fatalf("failed to read grub.cfg")
			}
			re := regexp.MustCompile(tt.pattern)
			match := re.FindAllStringIndex(string(contents), -1)
			// it should match partition A and B in total twice
			if len(match) != 2 {
				t.Fatalf("expected pattern not found in grub.cfg")
			}
		})
	}
}

func copyGrub(src string, dst string) error {
	// Write data to dst
	err := os.WriteFile(dst, []byte(src), 0644)
	if err != nil {
		return fmt.Errorf("failed to read file %s", dst)
	}
	return nil
}

func Test_startConfig(t *testing.T) {
	type args struct {
		configs []*agent.SysConfig
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{
			name: "KernelSysctl",
			args: args{
				configs: []*agent.SysConfig{
					{Model: KernelSysctlName.String()},
					{Model: KerSysctlPersistName.String()},
					{Model: GrubCmdlineName.String()},
				},
			},
			wantErr: false,
		},
	}
	patchKerSysctl := gomonkey.ApplyMethodReturn(KernelSysctl{}, "SetConfig", nil)
	patchKerSysctlPersist := gomonkey.ApplyMethodReturn(KerSysctlPersist{}, "SetConfig", nil)
	patchGrub := gomonkey.ApplyMethodReturn(GrubCmdline{}, "SetConfig", nil)
	defer patchKerSysctl.Reset()
	defer patchKerSysctlPersist.Reset()
	defer patchGrub.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := startConfig(tt.args.configs); (err != nil) != tt.wantErr {
				t.Errorf("startConfig() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func Test_getDefaultProcPath(t *testing.T) {
	tests := []struct {
		name string
		want string
	}{
		{
			name: "get correct path",
			want: "/proc/sys/",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := getDefaultProcPath()
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("getAndSetConfigsFromFile() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_getKernelConPath(t *testing.T) {
	tests := []struct {
		name string
		want string
	}{
		{
			name: "get correct path",
			want: "/etc/sysctl.conf",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := getKernelConPath()
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("getAndSetConfigsFromFile() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_getGrubCfgPath(t *testing.T) {
	tests := []struct {
		name string
		want string
	}{
		{
			name: "get correct path",
			want: "/boot/efi/EFI/openEuler/grub.cfg",
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := getGrubCfgPath()
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("getAndSetConfigsFromFile() = %v, want %v", got, tt.want)
			}
		})
	}
}
