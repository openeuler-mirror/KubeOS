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

// Package values contains values used by proxy and operator
package values

import (
	"time"

	ctrl "sigs.k8s.io/controller-runtime"
)

const (
	// LabelUpgrading is the key of the upgrading label for nodes
	LabelUpgrading = "upgrade.openeuler.org/upgrading"
	// LabelMaster is the key of the master-node label for nodes
	LabelMaster = "node-role.kubernetes.io/control-plane"
	// LabelOSinstance is used to select the osinstance with the nodeName by label
	LabelOSinstance = "upgrade.openeuler.org/osinstance-node"
	// LabelNodeSelector is used to filter the nodes that need to be upgraded or configured.
	LabelNodeSelector = "upgrade.openeuler.org/node-selector"
	// LabelConfiguring is the key of the configuring label for nodes
	LabelConfiguring = "upgrade.openeuler.org/configuring"
	// LabelSerial is the key of the serial label for nodes
	LabelSerial = "upgrade.openeuler.org/serial"

	defaultPeriod = 15 * time.Second
	// OsiStatusName is param name of nodeStatus in osInstance
	OsiStatusName = "nodestatus"
	// UpgradeConfigName is param name of UpgradeConfig
	UpgradeConfigName = "UpgradeConfig"
	// SysConfigName is param name of SysConfig
	SysConfigName = "SysConfig"
	// OsiNamespace is the namespace of osinstance
	OsiNamespace = "default"
)

// NodeStatus defines state of nodes
type NodeStatus int32

const (
	// NodeStatusIdle represents idle state of nodes
	NodeStatusIdle NodeStatus = 0
	// NodeStatusUpgrade represents upgrade state of nodes
	NodeStatusUpgrade NodeStatus = 1
	// NodeStatusRollback represents rollback state of nodes
	NodeStatusRollback NodeStatus = 2
	// NodeStatusConfig represents config state of nodes
	NodeStatusConfig NodeStatus = 3
)

func (n NodeStatus) String() string {
	switch n {
	case NodeStatusIdle:
		return "idle"
	case NodeStatusUpgrade:
		return "upgrade"
	case NodeStatusRollback:
		return "rollback"
	case NodeStatusConfig:
		return "config"
	default:
		return "unknown"
	}
}

var (
	// NoRequeue indicates controller do not requeue the reconcile key
	NoRequeue = ctrl.Result{}
	// RequeueNow indicates controller requeue the reconcile key now
	RequeueNow = ctrl.Result{Requeue: true}
	// Requeue indicates controller requeue the reconcile key after defaultPeriod
	Requeue = ctrl.Result{Requeue: true, RequeueAfter: defaultPeriod}
)

// ConfigOperation defines operations about configuration when check config version
type ConfigOperation string

const (
	// DoNothing represents config version is same, continue
	DoNothing ConfigOperation = "doNothing"
	// Reassign represents config version is  not same, reassgin config
	Reassign ConfigOperation = "reassgin"
	// UpdateConfig represents config version is  not same, update config
	UpdateConfig ConfigOperation = "updateConfig"
)
