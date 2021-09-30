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
	LabelMaster   = "node-role.kubernetes.io/control-plane"
	defaultPeriod = 15 * time.Second
)

var (
	// NoRequeue indicates controller do not requeue the reconcile key
	NoRequeue = ctrl.Result{}
	// RequeueNow indicates controller requeue the reconcile key now
	RequeueNow = ctrl.Result{Requeue: true}
	// Requeue indicates controller requeue the reconcile key after defaultPeriod
	Requeue = ctrl.Result{Requeue: true, RequeueAfter: defaultPeriod}
)
