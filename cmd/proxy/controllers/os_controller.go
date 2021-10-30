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

// Package controllers contains the Reconcile of proxy
package controllers

import (
	"context"
	"os"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/kubernetes"
	"k8s.io/kubectl/pkg/drain"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/manager"

	upgradev1 "openeuler.org/KubeOS/api/v1alpha1"
	"openeuler.org/KubeOS/pkg/agentclient"
	"openeuler.org/KubeOS/pkg/common"
	"openeuler.org/KubeOS/pkg/values"
)

// OSReconciler reconciles a OS object
type OSReconciler struct {
	Scheme     *runtime.Scheme
	Connection *agentclient.Client
	client.Client
	kubeclientset kubernetes.Interface
	hostName      string
}

var log = ctrl.Log.WithName("proxy").WithName("OS")

// NewOSReconciler returns a new os reconciler
func NewOSReconciler(mgr manager.Manager) *OSReconciler {
	kubeClientSet, err := kubernetes.NewForConfig(mgr.GetConfig())
	if err != nil {
		log.Error(err, "Error building kubernetes clientset: ", "err")
	}
	reconciler := &OSReconciler{
		Client:        mgr.GetClient(),
		Scheme:        mgr.GetScheme(),
		kubeclientset: kubeClientSet,
		hostName:      os.Getenv("NODE_NAME"),
	}
	log.Info("Setting up event handlers")
	return reconciler
}

//+kubebuilder:rbac:groups=upgrade.openeuler.org,resources=os,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=upgrade.openeuler.org,resources=os/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=upgrade.openeuler.org,resources=os/finalizers,verbs=update
//+kubebuilder:rbac:groups="",resources=nodes,verbs=get;list;update;watch;patch
//+kubebuilder:rbac:groups="",resources=pods,verbs=get;list
//+kubebuilder:rbac:groups="",resources=pods/eviction,verbs=create
//+kubebuilder:rbac:groups="apps",resources=daemonsets,verbs=get;delete

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
func (r *OSReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ctx = context.Background()
	osInstance, node := getOSAndNodeStatus(ctx, r, req.NamespacedName, r.hostName)
	osVersionSpec := osInstance.Spec.OSVersion
	osVersionNode := node.Status.NodeInfo.OSImage

	drainer := &drain.Helper{
		Client:              r.kubeclientset,
		Force:               true,
		GracePeriodSeconds:  -1,
		IgnoreAllDaemonSets: true,
		Out:                 os.Stdout,
		ErrOut:              os.Stderr,
	}
	if osVersionNode == osVersionSpec {
		delete(node.Labels, values.LabelUpgrading)
		if err := r.Update(ctx, &node); err != nil {
			log.Error(err, "unable to label", "node", node.Name)
			return values.RequeueNow, err
		}
		if node.Spec.Unschedulable { // update done, uncordon the node
			if err := drain.RunCordonOrUncordon(drainer, &node, false); err != nil {
				return values.RequeueNow, err
			}
			log.Info("Uncordon successfully", "node", node.Name)
		}

	}
	if _, ok := node.Labels[values.LabelUpgrading]; ok {
		if err := evictNode(drainer, &node); err != nil {
			return values.RequeueNow, err
		}
		version := osVersionSpec
		imageURL := osInstance.Spec.ImageURL
		checkSum := osInstance.Spec.CheckSum
		flagSafe:= osInstance.Spec.FlagSafe
		if err := r.Connection.UpdateSpec(version, imageURL, flagSafe,checkSum); err != nil {
			return values.RequeueNow, err
		}
	}
	return values.Requeue, nil
}

func getOSAndNodeStatus(ctx context.Context, r common.ReadStatusWriter, name types.NamespacedName,
	hostName string) (OS upgradev1.OS, node corev1.Node) {
	if err := r.Get(ctx, name, &OS); err != nil {
		log.Error(err, "unable to fetch OS")
		return
	}
	if err := r.Get(ctx, client.ObjectKey{Name: hostName}, &node); err != nil {
		log.Error(err, "unable to fetch node")
		return
	}
	return
}

func evictNode(drainer *drain.Helper, node *corev1.Node) error {
	if node.Spec.Unschedulable {
		return nil
	}
	log.Info("Evicting Node ", "nodeName", node.Name)
	// Mark node unschedulable and evict all pods on it
	err := drain.RunCordonOrUncordon(drainer, node, true)
	if err != nil {
		return err
	}
	if err := drain.RunNodeDrain(drainer, node.Name); err != nil {
		log.Error(err, "unable to drain node")
		if terr := drain.RunCordonOrUncordon(drainer, node, false); terr != nil {
			log.Error(terr, "unable to uncordon node when an error occurs in draining node")
			return terr
		}
		return err
	}
	return nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *OSReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&upgradev1.OS{}).
		Complete(r)
}
