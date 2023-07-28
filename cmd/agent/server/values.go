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

// Package server implements server of os-agent. The server uses gRPC interface.
package server

const (
	// PersistDir is the path which could persist when os updating
	PersistDir = "/persist"
	// SockDir is the path of the sock file
	SockDir = "/run/os-agent"
	// SockName is the path of the socket file
	SockName = "os-agent.sock"
)

// ConfigType defines type of configurations
type ConfigType int32

const (
	// KernelSysctlName is the configuration name of the kernel parameter set by using sysctl
	KernelSysctlName ConfigType = 0
	// KerSysctlPersistName is the configuration name of the kernel parameter
	// set by writing /etc/sysctl.conf or other files
	KerSysctlPersistName ConfigType = 1
	// GrubCmdlineCurName is configuration name of current partition's grub cmdline
	GrubCmdlineCurName ConfigType = 2
	// GrubCmdlineNextName is configuration name of next partition's grub cmdline
	GrubCmdlineNextName ConfigType = 3
)

func (c ConfigType) String() string {
	switch c {
	case KernelSysctlName:
		return "kernel.sysctl"
	case KerSysctlPersistName:
		return "kernel.sysctl.persist"
	case GrubCmdlineCurName:
		return "grub.cmdline.current"
	case GrubCmdlineNextName:
		return "grub.cmdline.next"
	default:
		return "unknown"
	}
}
