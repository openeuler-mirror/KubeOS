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
	"time"

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
	os, err := getOSCr(ctx, r, req.NamespacedName)
	if err != nil {
		if errors.IsNotFound(err) {
			return values.NoRequeue, nil
		}
		return values.RequeueNow, err
	}

	isWithinTimeWindow, err := isWithinTimeWindow(os.Spec.TimeWindow.StartTime, os.Spec.TimeWindow.EndTime)
	if err != nil {
		return values.RequeueNow, err
	}
	if !isWithinTimeWindow {
		//Todo consider time interval
		return values.RequeueNow, nil
	}

	ops := os.Spec.OpsType
	var opsInsatnce operation
	switch ops {
	case "upgrade", "rollback":
		opsInsatnce = upgradeOps{label: opsLabel{label: values.LabelUpgrading, op: selection.DoesNotExist}}
	case "config":
		opsInsatnce = configOps{label: opsLabel{label: values.LabelConfiguring, op: selection.DoesNotExist}}
	default:
		log.Error(nil, "operation "+ops+" cannot be recognized")
		return values.Requeue, nil
	}
	nodeNum, err := getNodeNum(ctx, r, os.Spec.NodeSelector)
	if err != nil {
		return values.RequeueNow, err
	}
	limit, err := calNodeLimit(ctx, r, opsInsatnce.getOpsLabel(), min(os.Spec.MaxUnavailable, nodeNum), os.Spec.NodeSelector) // adjust maxUnavailable if need
	if err != nil {
		return values.RequeueNow, err
	}
	if needRequeue, err := assignOperation(ctx, r, os, limit, opsInsatnce); err != nil {
		return values.RequeueNow, err
	} else if needRequeue {
		return values.Requeue, nil
	}
	return setTimeInterval(os.Spec.TimeInterval), nil
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
	if err := mgr.GetFieldIndexer().IndexField(context.Background(), &upgradev1.OS{}, "metadata.name",
		func(rawObj client.Object) []string {
			os, ok := rawObj.(*upgradev1.OS)
			if !ok {
				log.Error(nil, "failed to convert to osInstance")
				return []string{}
			}
			return []string{os.Name}
		}); err != nil {
		return err
	}
	if err := mgr.GetFieldIndexer().IndexField(context.Background(), &upgradev1.OS{}, "metadata.namespace",
		func(rawObj client.Object) []string {
			os, ok := rawObj.(*upgradev1.OS)
			if !ok {
				log.Error(nil, "failed to convert to osInstance")
				return []string{}
			}
			return []string{os.Namespace}
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

func getOSCr(ctx context.Context, r common.ReadStatusWriter, name types.NamespacedName) (upgradev1.OS, error) {
	var os upgradev1.OS
	if err := r.Get(ctx, name, &os); err != nil {
		log.Error(err, "unable to fetch OS")
		return upgradev1.OS{}, err
	}
	if err := checkNodeSelector(ctx, r, os); err != nil {
		log.Error(err, "nodeselector conficts")
		return upgradev1.OS{}, err
	}
	return os, nil
}

// Get nodes which do not have master label，have nodeselector label or do not have  other os cr nodeselector
func getNodeNum(ctx context.Context, r common.ReadStatusWriter, nodeSelector string) (int, error) {
	labelList := []labelRequirementCreator{masterLabel{op: selection.DoesNotExist}, nodeSelectorLabel{value: nodeSelector, op: selection.Equals}}
	requirements, err := createRequirement(labelList)
	if err != nil {
		return 0, err
	}
	nodesItems, err := getNodes(ctx, r, 0, requirements...)
	if err != nil {
		log.Error(err, "get slave nodes fail")
		return 0, err
	}
	nodeNum := len(nodesItems)
	return nodeNum, nil
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

// get now in upgrading and match with nodeselector
func calNodeLimit(ctx context.Context, r common.ReadStatusWriter,
	label opsLabel, maxUnavailable int, nodeSelector string) (int, error) {
	label.op = selection.Exists
	labelList := []labelRequirementCreator{
		masterLabel{op: selection.DoesNotExist},
		label,
		nodeSelectorLabel{value: nodeSelector, op: selection.Equals}}
	requirements, err := createRequirement(labelList)
	if err != nil {
		return 0, err
	}
	nodes, err := getNodes(ctx, r, 0, requirements...)
	if err != nil {
		return 0, err
	}
	return maxUnavailable - len(nodes), nil
}

func assignOperation(ctx context.Context, r common.ReadStatusWriter, os upgradev1.OS, limit int,
	opsInstance operation) (bool, error) {
	opsLabel := opsInstance.getOpsLabel()
	opsLabel.op = selection.DoesNotExist
	labelList := []labelRequirementCreator{
		masterLabel{op: selection.DoesNotExist},
		opsLabel,
		nodeSelectorLabel{value: os.Spec.NodeSelector, op: selection.Equals}}
	requirements, err := createRequirement(labelList)
	nodes, err := getNodes(ctx, r, limit+1, requirements...) // one more to see if all nodes updated
	if err != nil {
		return false, err
	}
	// Upgrade OS for selected nodes
	count, err := opsInstance.updateNodes(ctx, r, &os, nodes, limit)
	if err != nil {
		return false, err
	}

	return count >= limit, nil
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

// Check whether the nodeselector conflicts with other nodeselector in OS CRs. If the nodeselector is empty, return the list of other nodeselectors.
func checkNodeSelector(ctx context.Context, r common.ReadStatusWriter, os upgradev1.OS) error {
	var osList upgradev1.OSList
	if err := r.List(ctx, &osList, &client.ListOptions{}); err != nil {
		log.Error(err, "unable to list nodes with requirements")
		return err
	}
	var sameNodeSelectorList []types.NamespacedName
	for _, osItem := range osList.Items {
		// Exclude current os, controller-runtime not supports multiple indexs as listoptions in current version,
		// so cannot list os without current os use List function
		if osItem.Name == os.Name && osItem.Namespace == os.Namespace {
			continue
		}
		if os.Spec.NodeSelector == osItem.Spec.NodeSelector {
			sameNodeSelectorList = append(sameNodeSelectorList, types.NamespacedName{
				Namespace: osItem.Namespace,
				Name:      osItem.Name,
			})
		}
	}
	// If a node label corresponds to multiple OS CRs, upgrade or configuration information may conflict.
	// As a result, an error is reported and returned when there are one-to-many relationships.
	if len(sameNodeSelectorList) > 0 {
		errorMessage := sameNodeSelectorList[0].String()
		for i := 1; i < len(sameNodeSelectorList); i++ {
			errorMessage = errorMessage + " , " + sameNodeSelectorList[i].String()
		}
		log.Error(nil, "OS CR "+os.Name+" in namespace "+os.Namespace+" has same nodeselector with "+errorMessage)
		return fmt.Errorf("OS CR %s in namespace %s has same nodeselector with %s", os.Name, os.Namespace, errorMessage)
	}
	return nil
}

func setTimeInterval(timeInterval int) ctrl.Result {
	return ctrl.Result{Requeue: true, RequeueAfter: time.Duration(timeInterval) * time.Second}
}

func createRequirement(labelsList []labelRequirementCreator) ([]labels.Requirement, error) {
	var requirements []labels.Requirement
	for _, label := range labelsList {
		requirement, err := label.createLabelRequirement()
		if err != nil {
			log.Error(err, "unable to create requirement "+values.LabelNodeSelector)
			return []labels.Requirement{}, err
		}
		requirements = append(requirements, requirement...)
	}
	return requirements, nil
}
