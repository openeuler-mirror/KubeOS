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
	"strconv"
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
	log.V(1).Info("start Reconcile of " + req.Name + " with namespace is " + req.Namespace)
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
		log.V(1).Info("not in time window, the start time is " + os.Spec.TimeWindow.StartTime +
			" , the end time " + os.Spec.TimeWindow.EndTime)
		return values.Requeue, nil
	}
	ops := os.Spec.OpsType
	var opsInstance operation
	switch ops {
	case "upgrade", "rollback":
		opsInstance = upgradeOps{
			label: opsLabel{
				label: values.LabelUpgrading,
				op:    selection.DoesNotExist,
			},
		}
	case "config":
		opsInstance = configOps{
			label: opsLabel{
				label: values.LabelConfiguring,
				op:    selection.DoesNotExist,
			},
		}
	default:
		log.Error(nil, "operation "+ops+" cannot be recognized")
		return values.Requeue, nil
	}
	commonNodesReq, err := newCommonsNodesRequirement(os.Spec.NodeSelector,
		selection.Equals).createNodeRequirement(ctx, r)
	if err != nil {
		return values.RequeueNow, err
	}
	allNodes, err := getNodes(ctx, r, 0, commonNodesReq...)
	if err != nil {
		return values.RequeueNow, err
	}
	log.V(1).Info("get all nodes num is " + strconv.Itoa(len(allNodes)))
	switch os.Spec.ExecutionMode {
	case ExecutionModeParallel:
		result, err := executeParallelOperation(ctx, r, os, opsInstance, len(allNodes))
		if err != nil {
			return values.RequeueNow, nil
		}
		return result, nil
	case ExecutionModeSerial:
		result, err := executeSerialOperation(ctx, r, os, opsInstance, len(allNodes))
		if err != nil {
			return values.RequeueNow, err
		}
		return result, nil
	default:
		log.Error(nil, "executionMode "+os.Spec.ExecutionMode+" cannot be recognized")
		return values.Requeue, nil
	}
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
	maxUnavailable int, requirements []labels.Requirement) (int, error) {
	nodes, err := getNodes(ctx, r, 0, requirements...)
	if err != nil {
		return 0, err
	}
	return maxUnavailable - len(nodes), nil
}
func assignOperation(ctx context.Context, r common.ReadStatusWriter, os upgradev1.OS, limit int,
	opsInstance operation, requirements []labels.Requirement) (int, error) {
	if limit == 0 {
		log.V(1).Info("limit is 0 , do not need to assign operation")
		return 0, nil
	}
	nodes, err := getNodes(ctx, r, limit+1, requirements...) // one more to see if all nodes updated
	if err != nil {
		return 0, err
	}
	log.V(1).Info("get wait to check nodes is " + strconv.Itoa(len(nodes)))
	count, errLists := opsInstance.updateNodes(ctx, r, &os, nodes, limit)
	if len(errLists) != 0 {
		return 0, fmt.Errorf("update nodes and osinstance error")
	}
	return count, nil
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
		osinstance.Spec.UpgradeConfigs = upgradev1.SysConfigs{}
		if err = json.Unmarshal(data, &osinstance.Spec.UpgradeConfigs); err != nil {
			return err
		}
	case values.SysConfigName:
		data, err := json.Marshal(os.Spec.SysConfigs)
		if err != nil {
			return err
		}
		osinstance.Spec.SysConfigs = upgradev1.SysConfigs{}
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

func executeParallelOperation(ctx context.Context, r common.ReadStatusWriter, os upgradev1.OS,
	opsInstance operation, nodeNum int) (ctrl.Result, error) {
	log.V(1).Info("start parallel operation")
	opsLabel := opsInstance.getOpsLabel()
	opsLabel.op = selection.Exists
	opsNodesReq, err := newopsNodesRequirement(os.Spec.NodeSelector,
		selection.Equals, opsLabel).createNodeRequirement(ctx, r)
	if err != nil {
		return values.RequeueNow, nil
	}
	limit, err := calNodeLimit(ctx, r, min(os.Spec.MaxUnavailable, nodeNum), opsNodesReq) // adjust maxUnavailable if need
	if err != nil {
		return values.RequeueNow, nil
	}
	log.V(1).Info("get limit is " + strconv.Itoa(limit))
	opsLabel.op = selection.DoesNotExist
	noOpsNodesReq, err := newopsNodesRequirement(os.Spec.NodeSelector,
		selection.Equals, opsLabel).createNodeRequirement(ctx, r)
	if err != nil {
		return values.RequeueNow, nil
	}
	if _, err := assignOperation(ctx, r, os, limit, opsInstance, noOpsNodesReq); err != nil {
		return values.RequeueNow, nil
	}
	return setTimeInterval(os.Spec.TimeInterval), nil
}

func executeSerialOperation(ctx context.Context, r common.ReadStatusWriter, os upgradev1.OS,
	opsInstance operation, nodeNum int) (ctrl.Result, error) {
	log.V(1).Info("start serial operation")
	opsLabel := opsInstance.getOpsLabel()
	opsLabel.op = selection.Exists
	opsNodesReq, err := newopsNodesRequirement(os.Spec.NodeSelector,
		selection.Equals, opsLabel).createNodeRequirement(ctx, r)
	if err != nil {
		return values.RequeueNow, nil
	}
	opsNodeNum, err := getNodes(ctx, r, 0, opsNodesReq...)
	if err != nil {
		return values.RequeueNow, nil
	}
	if len(opsNodeNum) > 0 {
		log.V(1).Info("a node is being upgraded or configured. Wait until the node upgrade or configuration is complete.")
		return values.Requeue, nil
	}

	serialNodesRequirement, err := newSerialNodesRequirement(os.Spec.NodeSelector,
		selection.Equals, selection.Exists).createNodeRequirement(ctx, r)
	if err != nil {
		return values.RequeueNow, nil
	}
	serialNodeLimit, err := calNodeLimit(ctx, r, min(os.Spec.MaxUnavailable, nodeNum), serialNodesRequirement)
	if err != nil {
		return values.RequeueNow, nil
	}
	log.V(1).Info("get the number of nodes which need to be added serial label num is " + strconv.Itoa(serialNodeLimit))
	noSerialNodesRequirement, err := newSerialNodesRequirement(os.Spec.NodeSelector,
		selection.Equals, selection.DoesNotExist).createNodeRequirement(ctx, r)
	if err != nil {
		return values.RequeueNow, nil
	}
	// add serial label to node
	serialOpsInstance := serialOps{
		label: opsInstance.getOpsLabel(),
	}
	log.V(1).Info("start add serial label to nodes")
	if _, err := assignOperation(ctx, r, os, serialNodeLimit, serialOpsInstance, noSerialNodesRequirement); err != nil {
		return values.RequeueNow, nil
	}

	log.V(1).Info("start check nodes needed to be upgrade/configure or not")
	serialLimit := 1 // 1 is the number of operation nodes when execution mode in serial
	count, err := assignOperation(ctx, r, os, serialLimit, opsInstance, serialNodesRequirement)
	if err != nil {
		return values.RequeueNow, nil
	}
	if count > 0 {
		return values.Requeue, nil
	}
	return setTimeInterval(os.Spec.TimeInterval), nil
}
