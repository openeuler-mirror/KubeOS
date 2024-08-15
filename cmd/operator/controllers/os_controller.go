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
	"encoding/json"
	"fmt"

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
	var opsInsatnce operation
	switch ops {
	case "upgrade", "rollback":
		opsInsatnce = upgradeOps{}
	case "config":
		opsInsatnce = configOps{}
	default:
		log.Error(nil, "operation "+ops+" cannot be recognized")
		return values.Requeue, nil
	}
	limit, err := calNodeLimit(ctx, r, opsInsatnce, min(os.Spec.MaxUnavailable, nodeNum), os.Spec.NodeSelector) // adjust maxUnavailable if need
	if err != nil {
		return values.RequeueNow, err
	}
	if needRequeue, err := assignOperation(ctx, r, os, limit, opsInsatnce); err != nil {
		return values.RequeueNow, err
	} else if needRequeue {
		return values.Requeue, nil
	}
	return values.Requeue, nil
}

// SetupWithManager sets up the controller with the Manager.
func (r *OSReconciler) SetupWithManager(mgr ctrl.Manager) error {
	if err := mgr.GetFieldIndexer().IndexField(context.Background(), &upgradev1.OSInstance{}, values.OsiStatusName,
		func(rawObj client.Object) []string {
			osi, ok := rawObj.(*upgradev1.OSInstance)
			if !ok {
				log.Error(nil, "failed to convert to osInstance")
				return []string{}
			}
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
	labelSelector := labels.SelectorFromSet(labels.Set{values.LabelOSinstance: hostname})
	osInstanceList := &upgradev1.OSInstanceList{}
	if err := r.List(ctx, osInstanceList, client.MatchingLabelsSelector{Selector: labelSelector}); err != nil {
		log.Error(err, "unable to list osInstances")
		return
	}
	for _, osInstance := range osInstanceList.Items {
		if err := r.Delete(ctx, &osInstance); err != nil {
			log.Error(err, "unable to delete osInstance")
		}
		log.Info("Delete osinstance successfully", "name", hostname)
	}
}

func getAndUpdateOS(ctx context.Context, r common.ReadStatusWriter, name types.NamespacedName) (upgradev1.OS,
	int, error) {
	var os upgradev1.OS
	if err := r.Get(ctx, name, &os); err != nil {
		log.Error(err, "unable to fetch OS")
		return upgradev1.OS{}, 0, err
	}

	requirement, err := labels.NewRequirement(values.LabelMaster, selection.DoesNotExist, nil)
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelMaster)
		return upgradev1.OS{}, 0, err
	}
	var requirements []labels.Requirement
	requirements = append(requirements, *requirement)
	if os.Spec.NodeSelector != "" {
		reqSelector, err := labels.NewRequirement(values.LabelNodeSelector, selection.Exists, nil)
		if err != nil {
			log.Error(err, "unable to create requirement "+values.LabelNodeSelector)
			return upgradev1.OS{}, 0, err
		}
		requirements = append(requirements, *requirement, *reqSelector)
	}
	nodesItems, err := getNodes(ctx, r, 0, requirements...)
	if err != nil {
		log.Error(err, "get slave nodes fail")
		return upgradev1.OS{}, 0, err
	}
	nodeNum := len(nodesItems)
	return os, nodeNum, nil
}

func assignOperation(ctx context.Context, r common.ReadStatusWriter, os upgradev1.OS, limit int,
	ops operation) (bool, error) {
	requirement, err := ops.newNotExistRequirement()
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelUpgrading)
		return false, err
	}
	reqMaster, err := labels.NewRequirement(values.LabelMaster, selection.DoesNotExist, nil)
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelMaster)
		return false, err
	}
	var requirements []labels.Requirement
	requirements = append(requirements, requirement, *reqMaster)
	if os.Spec.NodeSelector != "" {
		reqSelector, err := labels.NewRequirement(values.LabelNodeSelector, selection.Equals, []string{os.Spec.NodeSelector})
		if err != nil {
			log.Error(err, "unable to create requirement "+values.LabelNodeSelector)
			return false, err
		}
		requirements = append(requirements, *reqSelector)
	}

	nodes, err := getNodes(ctx, r, limit+1, requirements...) // one more to see if all nodes updated
	if err != nil {
		return false, err
	}
	// Upgrade OS for selected nodes
	count, err := ops.updateNodes(ctx, r, &os, nodes, limit)
	if err != nil {
		return false, err
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

func calNodeLimit(ctx context.Context, r common.ReadStatusWriter,
	ops operation, maxUnavailable int, nodeSelector string) (int, error) {
	requirement, err := ops.newExistRequirement()
	if err != nil {
		log.Error(err, "unable to create requirement "+values.LabelUpgrading)
		return 0, err
	}
	var requirements []labels.Requirement
	requirements = append(requirements, requirement)
	if nodeSelector != "" {
		reqSelector, err := labels.NewRequirement(values.LabelNodeSelector, selection.Equals, []string{nodeSelector})
		if err != nil {
			log.Error(err, "unable to create requirement "+values.LabelNodeSelector)
			return 0, err
		}
		requirements = append(requirements, *reqSelector)
	}
	nodes, err := getNodes(ctx, r, 0, requirements...)
	if err != nil {
		return 0, err

	}
	return maxUnavailable - len(nodes), nil
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}

func deepCopySpecConfigs(os *upgradev1.OS, osinstance *upgradev1.OSInstance, configType string) error {
	switch configType {
	case values.UpgradeConfigName:
		data, err := json.Marshal(os.Spec.UpgradeConfigs)
		if err != nil {
			return err
		}
		if err = json.Unmarshal(data, &osinstance.Spec.UpgradeConfigs); err != nil {
			return err
		}
	case values.SysConfigName:
		data, err := json.Marshal(os.Spec.SysConfigs)
		if err != nil {
			return err
		}
		if err = json.Unmarshal(data, &osinstance.Spec.SysConfigs); err != nil {
			return err
		}
	default:
		log.Error(nil, "configType "+configType+" cannot be recognized")
		return fmt.Errorf("configType %s cannot be recognized", configType)
	}
	return nil
}
