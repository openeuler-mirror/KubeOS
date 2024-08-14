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

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/types"
	"openeuler.org/KubeOS/pkg/common"
	"openeuler.org/KubeOS/pkg/values"
	"sigs.k8s.io/controller-runtime/pkg/client"

	upgradev1 "openeuler.org/KubeOS/api/v1alpha1"
)

type operation interface {
	updateNodes(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
		nodes []corev1.Node, limit int) (int, error)
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
	nodes []corev1.Node, limit int) (int, error) {
	var count int
	for _, node := range nodes {
		if count >= limit {
			break
		}
		osVersionNode := node.Status.NodeInfo.OSImage
		if os.Spec.OSVersion != osVersionNode {
			log.Info("Upgrading node " + node.Name)
			var osInstance upgradev1.OSInstance
			if err := r.Get(ctx, types.NamespacedName{Namespace: os.GetObjectMeta().GetNamespace(), Name: node.Name}, &osInstance); err != nil {
				if err = client.IgnoreNotFound(err); err != nil {
					log.Error(err, "failed to get osInstance "+node.Name, "skip this node")
					return count, err
				}
				continue
			}
			if err := u.updateNodeAndOSins(ctx, r, os, &node, &osInstance); err != nil {
				log.Error(err, "failed to update node and osinstance ,skip this node ")
				continue
			}
			count++
		}
	}
	return count, nil
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
	nodes []corev1.Node, limit int) (int, error) {
	var count int
	for _, node := range nodes {
		if count >= limit {
			break
		}
		var osInstance upgradev1.OSInstance
		if err := r.Get(ctx, types.NamespacedName{Namespace: os.GetObjectMeta().GetNamespace(), Name: node.Name}, &osInstance); err != nil {
			if err = client.IgnoreNotFound(err); err != nil {
				log.Error(err, "failed to get osInstance "+node.Name)
				return count, err
			}
			continue
		}
		if os.Spec.SysConfigs.Version != osInstance.Spec.SysConfigs.Version {
			log.Info("Configuring node " + node.Name)
			if err := c.updateNodeAndOSins(ctx, r, os, &node, &osInstance); err != nil {
				log.Error(err, "failed to update node and osinstance ,skip this node ")
				continue
			}
			count++
		}
	}
	return count, nil
}

func (c configOps) updateNodeAndOSins(ctx context.Context, r common.ReadStatusWriter, os *upgradev1.OS,
	node *corev1.Node, osInstance *upgradev1.OSInstance) error {
	if err := deepCopySpecConfigs(os, osInstance, values.SysConfigName); err != nil {
		return err
	}
	osInstance.Spec.NodeStatus = values.NodeStatusConfig.String()
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
