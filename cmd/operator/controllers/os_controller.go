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

// Package controllers contains the Reconcile of operator
package controllers

import (
	"context"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/selection"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/util/workqueue"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/event"
	"sigs.k8s.io/controller-runtime/pkg/handler"
	"sigs.k8s.io/controller-runtime/pkg/source"

	upgradev1 "openeuler.org/KubeOS/api/v1alpha1"
	"openeuler.org/KubeOS/pkg/common"
	"openeuler.org/KubeOS/pkg/values"
)

// OSReconciler reconciles an OS object
type OSReconciler struct {
	Scheme *runtime.Scheme
	client.Client
}

var log = ctrl.Log.WithName("operator").WithName("OS")

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
func (r *OSReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	if r.Client == nil {
		return values.NoRequeue, nil
	}
	ctx = context.Background()
	return Reconcile(ctx, r, req)
}

// Reconcile compares the actual state with the desired and updates the status of the resources e.g. nodes
func Reconcile(ctx context.Context, r common.ReadStatusWriter, req ctrl.Request) (ctrl.Result, error) {
	os, nodeNum, err := getAndUpdateOS(ctx, r, req.NamespacedName)
	if err != nil {
		if errors.IsNotFound(err) {
			return values.NoRequeue, nil
		}
		return values.RequeueNow, err
	}

	ops := os.Spec.OpsType
	switch ops {
	case "upgrade", "rollback":
		limit, err := checkUpgrading(ctx, r, min(os.Spec.MaxUnavailable, nodeNum)) // adjust maxUnavailable if need
		if err != nil {
			return values.RequeueNow, err
		}
		if needRequeue, err := assignUpgrade(ctx, r, os, limit, req.Namespace); err != nil {
			return values.RequeueNow, err
		} else if needRequeue {
			return values.Requeue, nil
		}
	case "config":
		limit, err := checkConfig(ctx, r, min(os.Spec.MaxUnavailable, nodeNum))
		if err != nil {
			return values.RequeueNow, err
		}
		if needRequeue, err := assignConfig(ctx, r, os.Spec.SysConfigs, os.Spec.SysConfigs.Version, limit); err != nil {
			return values.RequeueNow, err
		} else if needRequeue {
			return values.Requeue, nil
		}
	default:
		log.Error(nil, "operation "+ops+" cannot be recognized")
	}
	return values.Requeue, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *OSReconciler) SetupWithManager(mgr ctrl.Manager) error {
	if err := mgr.GetFieldIndexer().IndexField(context.Background(), &upgradev1.OSInstance{}, values.OsiStatusName,
		func(rawObj client.Object) []string {
			osi := rawObj.(*upgradev1.OSInstance)
			return []string{osi.Spec.NodeStatus}
		}); err != nil {
		return err
	}
	return ctrl.NewControllerManagedBy(mgr).
		For(&upgradev1.OS{}).
		Watches(&source.Kind{Type: &corev1.Node{}}, handler.Funcs{DeleteFunc: r.DeleteOSInstance}).
		Complete(r)
}

// DeleteOSInstance delete osInstance when delete nodes in cluster
func (r *OSReconciler) DeleteOSInstance(e event.DeleteEvent, q workqueue.RateLimitingInterface) {
	ctx := context.Background()
	hostname := e.Object.GetName()
	osInstance := upgradev1.OSInstance{}
	if err := r.Get(ctx, types.NamespacedName{
		Namespace: "default",
		Name:      hostname,
	}, &osInstance); err != nil {
		log.Error(err, "unable to get osinstance")
		return
	}
	if err := r.Delete(ctx, &osInstance); err != nil {
		log.Error(err, "unable to delete osinstance")
	}
}

func getAndUpdateOS(ctx context.Context, r common.ReadStatusWriter, name types.NamespacedName) (os upgradev1.OS,
	nodeNum int, err error) {
	if err = r.Get(ctx, name, &os); err != nil {
		log.Error(err, "unable to fetch OS")
		return
	}

	requirement, err := labels.NewRequirement(values.LabelMaster, selection.DoesNotExist, nil)
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelMaster)
		return
	}
	nodesItems, err := getNodes(ctx, r, 0, *requirement)
	if err != nil {
		log.Error(err, "get slave nodes fail")
		return
	}
	nodeNum = len(nodesItems)
	return
}

func assignUpgrade(ctx context.Context, r common.ReadStatusWriter, os upgradev1.OS, limit int,
	nameSpace string) (bool, error) {
	requirement, err := labels.NewRequirement(values.LabelUpgrading, selection.DoesNotExist, nil)
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelUpgrading)
		return false, err
	}
	reqMaster, err := labels.NewRequirement(values.LabelMaster, selection.DoesNotExist, nil)
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelMaster)
		return false, err
	}

	nodes, err := getNodes(ctx, r, limit+1, *requirement, *reqMaster) // one more to see if all node updated
	if err != nil {
		return false, err
	}

	var count = 0
	for _, node := range nodes {
		if count >= limit {
			break
		}
		osVersionNode := node.Status.NodeInfo.OSImage
		if os.Spec.OSVersion != osVersionNode {
			var osInstance upgradev1.OSInstance
			if err = r.Get(ctx, types.NamespacedName{Namespace: nameSpace, Name: node.Name}, &osInstance); err != nil {
				if err = client.IgnoreNotFound(err); err != nil {
					log.Error(err, "failed to get osInstance "+node.Name)
					return false, err
				}
				log.Error(err, "not found osInstance "+node.Name)
				continue
			}
			count++
			node.Labels[values.LabelUpgrading] = ""
			expUpVersion := os.Spec.UpgradeConfigs.Version
			osiUpVersion := osInstance.Spec.UpgradeConfigs.Version
			if osiUpVersion != expUpVersion {
				osInstance.Spec.UpgradeConfigs = os.Spec.UpgradeConfigs
			}
			expSysVersion := os.Spec.SysConfigs.Version
			osiSysVersion := osInstance.Spec.SysConfigs.Version
			if osiSysVersion != expSysVersion {
				osInstance.Spec.SysConfigs = os.Spec.SysConfigs
			}
			osInstance.Spec.NodeStatus = values.NodeStatusUpgrade.String()
			if err = r.Update(ctx, &osInstance); err != nil {
				log.Error(err, "unable to update", "osInstance", osInstance.Name)
			}
			if err = r.Update(ctx, &node); err != nil {
				log.Error(err, "unable to label", "node", node.Name)
			}
		}
	}
	return count >= limit, nil
}

func assignConfig(ctx context.Context, r common.ReadStatusWriter, sysConfigs upgradev1.SysConfigs,
	configVersion string, limit int) (bool, error) {
	osInstances, err := getIdleOSInstances(ctx, r, limit+1) // one more to see if all node updated
	if err != nil {
		return false, err
	}
	var count = 0
	for _, osInstance := range osInstances {
		if count >= limit {
			break
		}
		configVersionNode := osInstance.Spec.SysConfigs.Version
		if configVersion != configVersionNode {
			count++
			osInstance.Spec.SysConfigs = sysConfigs
			osInstance.Spec.NodeStatus = values.NodeStatusConfig.String()
			if err = r.Update(ctx, &osInstance); err != nil {
				log.Error(err, "unable update osInstance ", "osInstanceName ", osInstance.Name)
			}
		}
	}
	return count >= limit, nil
}

func getNodes(ctx context.Context, r common.ReadStatusWriter, limit int,
	reqs ...labels.Requirement) ([]corev1.Node, error) {
	var nodeList corev1.NodeList
	opts := client.ListOptions{LabelSelector: labels.NewSelector().Add(reqs...), Limit: int64(limit)}
	if err := r.List(ctx, &nodeList, &opts); err != nil {
		log.Error(err, "unable to list nodes with requirements")
		return nil, err
	}
	return nodeList.Items, nil
}

func getIdleOSInstances(ctx context.Context, r common.ReadStatusWriter, limit int) ([]upgradev1.OSInstance, error) {
	var osInstanceList upgradev1.OSInstanceList
	opt := []client.ListOption{
		client.MatchingFields{values.OsiStatusName: values.NodeStatusIdle.String()},
		&client.ListOptions{Limit: int64(limit)},
	}
	if err := r.List(ctx, &osInstanceList, opt...); err != nil {
		log.Error(err, "unable to list nodes with requirements")
		return nil, err
	}
	return osInstanceList.Items, nil
}

func getConfigOSInstances(ctx context.Context, r common.ReadStatusWriter) ([]upgradev1.OSInstance, error) {
	var osInstanceList upgradev1.OSInstanceList
	if err := r.List(ctx, &osInstanceList,
		client.MatchingFields{values.OsiStatusName: values.NodeStatusConfig.String()}); err != nil {
		log.Error(err, "unable to list nodes with requirements")
		return nil, err
	}
	return osInstanceList.Items, nil
}

func checkUpgrading(ctx context.Context, r common.ReadStatusWriter, maxUnavailable int) (int, error) {
	requirement, err := labels.NewRequirement(values.LabelUpgrading, selection.Exists, nil)
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelUpgrading)
		return 0, err
	}
	nodes, err := getNodes(ctx, r, 0, *requirement)
	if err != nil {
		return 0, err
	}
	return maxUnavailable - len(nodes), nil
}

func checkConfig(ctx context.Context, r common.ReadStatusWriter, maxUnavailable int) (int, error) {
	osInstances, err := getConfigOSInstances(ctx, r)
	if err != nil {
		return 0, err
	}
	return maxUnavailable - len(osInstances), nil
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
