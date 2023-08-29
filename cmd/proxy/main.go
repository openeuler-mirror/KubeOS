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

package main

import (
	"os"
	"path/filepath"

	"k8s.io/apimachinery/pkg/runtime"
	utilruntime "k8s.io/apimachinery/pkg/util/runtime"
	clientgoscheme "k8s.io/client-go/kubernetes/scheme"
	_ "k8s.io/client-go/plugin/pkg/client/auth"
	ctrl "sigs.k8s.io/controller-runtime"

	upgradev1 "openeuler.org/KubeOS/api/v1alpha1"
	"openeuler.org/KubeOS/cmd/agent/server"
	"openeuler.org/KubeOS/cmd/proxy/controllers"
	"openeuler.org/KubeOS/pkg/agentclient"
	"openeuler.org/KubeOS/pkg/common"
	"openeuler.org/KubeOS/pkg/version"
	//+kubebuilder:scaffold:imports
)

var (
	scheme   = runtime.NewScheme()
	setupLog = ctrl.Log.WithName("setup")
)

func init() {
	utilruntime.Must(clientgoscheme.AddToScheme(scheme))

	utilruntime.Must(upgradev1.AddToScheme(scheme))
	//+kubebuilder:scaffold:scheme
}

func main() {
	var err error
	mgr, err := common.NewControllerManager(setupLog, scheme)
	if err != nil {
		setupLog.Error(err, "unable to start manager")
		os.Exit(1)
	}

	reconciler := controllers.NewOSReconciler(mgr)
	if reconciler.Connection, err = agentclient.New("unix://" + filepath.Join(server.SockDir, server.SockName)); err != nil {
		setupLog.Error(err, "Error running proxy")
	}

	if err = reconciler.SetupWithManager(mgr); err != nil {
		setupLog.Error(err, "unable to create controller", "controller", "OS")
		os.Exit(1)
	}

	//+kubebuilder:scaffold:builder
	setupLog.WithValues("version", version.Version).Info("starting proxy manager")
	if err := mgr.Start(ctrl.SetupSignalHandler()); err != nil {
		setupLog.Error(err, "problem running proxy manager")
		os.Exit(1)
	}
}
