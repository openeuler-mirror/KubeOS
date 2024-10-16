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

// Package controllers contains the Reconcile of operator
package controllers

import (
	"context"
	"fmt"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/types"
	"sigs.k8s.io/controller-runtime/pkg/client"

	upgradev1 "openeuler.org/KubeOS/api/v1alpha1"
	"openeuler.org/KubeOS/pkg/common"
	"openeuler.org/KubeOS/pkg/values"
)

type operation interface {
	updateNodes(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
		nodes []corev1.Node, limit int) (int, []error)
	updateNodeAndOSins(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
		node *corev1.Node, osInstance *upgradev1.OSInstance) error
	getOpsLabel() opsLabel
}

type upgradeOps struct {
	label opsLabel
}

func (u upgradeOps) getOpsLabel() opsLabel {
	return u.label
}

func (u upgradeOps) updateNodes(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
	nodes []corev1.Node, limit int) (int, []error) {
	var count = 0
	var errList []error
	for _, node := range nodes {
		if count >= limit {
			break
		}
		osVersionNode := node.Status.NodeInfo.OSImage
		if os.Spec.OSVersion != osVersionNode {
			log.Info("Upgrading node " + node.Name)
			var osInstance upgradev1.OSInstance
			if err := r.Get(ctx, types.NamespacedName{Namespace: values.OsiNamespace, Name: node.Name}, &osInstance); err != nil {
				if err = client.IgnoreNotFound(err); err != nil {
					log.Error(err, "osInstance not found "+node.Name, ", maybe the os-proxy initialization is not complete. "+
						"Restart the reconcile and wait until it is complete.")
					return count, []error{err}
				}
				log.Error(err, "failed to get osInstance "+node.Name+"skip this node")
				errList = append(errList, err)
				continue
			}
			if err := u.updateNodeAndOSins(ctx, r, os, &node, &osInstance); err != nil {
				log.Error(err, "failed to update node and osinstance ,skip this node ")
				errList = append(errList, err)
				continue
			}
			count++
		}
	}
	if count == 0 && os.Spec.ExecutionMode == ExecutionModeSerial {
		if errList = deleteSerialLabel(ctx, r, nodes); len(errList) != 0 {
			log.Error(nil, "failed to delete nodes serial label")
		}
	}
	if len(errList) > 0 {
		return count, errList
	}
	return count, nil

}

func deleteSerialLabel(ctx context.Context, r common.ReadStatusWriter, nodes []corev1.Node) []error {
	var errList []error
	for _, node := range nodes {
		if _, ok := node.Labels[values.LabelSerial]; ok {
			delete(node.Labels, values.LabelSerial)
			if err := r.Update(ctx, &node); err != nil {
				log.Error(err, "unable to delete serial label ", "node", node.Name+", skip this node")
				errList = append(errList, err)
			}
			log.Info("delete node " + node.Name + " serial label " + values.LabelSerial + " successfully")
		}
	}
	if len(errList) > 0 {
		return errList
	}
	return nil
}

func (u upgradeOps) updateNodeAndOSins(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
	node *corev1.Node, osInstance *upgradev1.OSInstance) error {
	if osInstance.Spec.UpgradeConfigs.Version != os.Spec.UpgradeConfigs.Version {
		if err := deepCopySpecConfigs(os, osInstance, values.UpgradeConfigName); err != nil {
			return err
		}
	}
	if osInstance.Spec.SysConfigs.Version != os.Spec.SysConfigs.Version {
		if err := deepCopySpecConfigs(os, osInstance, values.SysConfigName); err != nil {
			return err
		}
		// exchange "grub.cmdline.current" and "grub.cmdline.next"
		for i, config := range osInstance.Spec.SysConfigs.Configs {
			if config.Model == "grub.cmdline.current" {
				osInstance.Spec.SysConfigs.Configs[i].Model = "grub.cmdline.next"
			}
			if config.Model == "grub.cmdline.next" {
				osInstance.Spec.SysConfigs.Configs[i].Model = "grub.cmdline.current"
			}
		}
	}
	osInstance.Spec.NodeStatus = values.NodeStatusUpgrade.String()
	osInstance.Spec.NamespacedName = upgradev1.NamespacedName{Name: os.Name, Namespace: os.Namespace}
	log.V(1).Info("Wait to update osinstance name:" + osInstance.Name + " node name is " + node.Name)
	if err := r.Update(ctx, osInstance); err != nil {
		log.Error(err, "unable to update", "osInstance", osInstance.Name)
		return err
	}
	log.Info("Update osinstance spec successfully")

	node.Labels[values.LabelUpgrading] = ""
	if err := r.Update(ctx, node); err != nil {
		log.Error(err, "unable to label", "node", node.Name)
		return err
	}
	log.Info("Add node upgrading label " + values.LabelUpgrading + " successfully")
	return nil
}

type configOps struct {
	label opsLabel
}

func (c configOps) getOpsLabel() opsLabel {
	return c.label
}

func (c configOps) updateNodes(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
	nodes []corev1.Node, limit int) (int, []error) {
	var count = 0
	var errList []error
	for _, node := range nodes {
		if count >= limit {
			break
		}
		var osInstance upgradev1.OSInstance
		if err := r.Get(ctx, types.NamespacedName{Namespace: os.GetObjectMeta().GetNamespace(), Name: node.Name}, &osInstance); err != nil {
			if err = client.IgnoreNotFound(err); err != nil {
				log.Error(err, "osInstance not found "+node.Name, ", maybe the os-proxy initialization is not complete. "+
					"Restart the reconcile and wait until it is complete.")
				return count, []error{err}
			}
			log.Error(err, "failed to get osInstance "+node.Name+", skip this node")
			errList = append(errList, err)
			continue
		}
		if os.Spec.SysConfigs.Version != osInstance.Spec.SysConfigs.Version {
			log.Info("Configuring node " + node.Name)
			if err := c.updateNodeAndOSins(ctx, r, os, &node, &osInstance); err != nil {
				log.Error(err, "failed to update node and osinstance ,skip this node ")
				errList = append(errList, err)
				continue
			}
			count++
		}
	}
	if count == 0 && os.Spec.ExecutionMode == ExecutionModeSerial {
		if errList = deleteSerialLabel(ctx, r, nodes); len(errList) != 0 {
			log.Error(nil, "failed to delete nodes serial label")
		}
	}
	if len(errList) > 0 {
		return count, errList
	}
	return count, errList

}

func (c configOps) updateNodeAndOSins(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
	node *corev1.Node, osInstance *upgradev1.OSInstance) error {
	if err := deepCopySpecConfigs(os, osInstance, values.SysConfigName); err != nil {
		return err
	}
	osInstance.Spec.NodeStatus = values.NodeStatusConfig.String()
	osInstance.Spec.NamespacedName = upgradev1.NamespacedName{Name: os.Name, Namespace: os.Namespace}
	log.V(1).Info("Wait to update osinstance name:" + osInstance.Name + " node name is " + node.Name)
	if err := r.Update(ctx, osInstance); err != nil {
		log.Error(err, "unable to update", "osInstance", osInstance.Name)
		return err
	}
	log.Info("Update osinstance spec successfully")

	node.Labels[values.LabelConfiguring] = ""
	if err := r.Update(ctx, node); err != nil {
		log.Error(err, "unable to label", "node", node.Name)
		return err
	}
	log.Info("Add node configuring label " + values.LabelConfiguring + " successfully")
	return nil
}

type serialOps struct {
	label opsLabel
}

func (s serialOps) getOpsLabel() opsLabel {
	return s.label
}

func (s serialOps) updateNodes(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
	nodes []corev1.Node, limit int) (int, []error) {
	var count int
	var errList []error
	for _, node := range nodes {
		if count >= limit {
			break
		}
		var osInstance upgradev1.OSInstance
		if err := r.Get(ctx, types.NamespacedName{Namespace: os.GetObjectMeta().GetNamespace(), Name: node.Name}, &osInstance); err != nil {
			if err = client.IgnoreNotFound(err); err != nil {
				log.Error(err, "osInstance not found "+node.Name, ", maybe the os-proxy initialization is not complete. "+
					"Restart the reconcile and wait until it is complete.")
				return count, []error{err}
			}
			log.Error(err, "failed to get osInstance "+node.Name+", skip this node")
			errList = append(errList, err)
			continue
		}
		switch s.getOpsLabel().label {
		case values.LabelUpgrading:
			if os.Spec.OSVersion != node.Status.NodeInfo.OSImage {
				log.Info("Add Serial Label to node " + node.Name)
				if err := s.updateNodeAndOSins(ctx, r, os, &node, &osInstance); err != nil {
					log.Error(err, "failed to update node and osinstance ,skip this node ")
					errList = append(errList, err)
					continue
				}
				count++
			}
		case values.LabelConfiguring:
			if os.Spec.SysConfigs.Version != osInstance.Spec.SysConfigs.Version {
				log.Info("Add Serial Label to node " + node.Name)
				if err := s.updateNodeAndOSins(ctx, r, os, &node, &osInstance); err != nil {
					log.Error(err, "failed to update node and osinstance ,skip this node ")
					errList = append(errList, err)
					continue
				}
				count++
			}
		default:
			log.Error(nil, "ops "+s.getOpsLabel().label+" cannot be recognized")
			return count, []error{fmt.Errorf("ops " + s.getOpsLabel().label + " cannot be recognized")}
		}
	}
	if len(errList) == 0 {
		return count, nil
	}
	return count, errList
}
func (s serialOps) updateNodeAndOSins(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
	node *corev1.Node, osInstance *upgradev1.OSInstance) error {
	node.Labels[values.LabelSerial] = ""
	if err := r.Update(ctx, node); err != nil {
		log.Error(err, "unable to label", "node", node.Name)
		return err
	}
	log.Info("Add node serial label " + values.LabelSerial + " successfully")
	return nil
}
