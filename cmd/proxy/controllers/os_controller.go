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
	"fmt"
	"os"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/util/retry"
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
//+kubebuilder:rbac:groups=upgrade.openeuler.org,resources=osinstances,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=upgrade.openeuler.org,resources=osinstances/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=upgrade.openeuler.org,resources=osinstances/finalizers,verbs=update
//+kubebuilder:rbac:groups="",resources=nodes,verbs=get;list;update;watch;patch
//+kubebuilder:rbac:groups="",resources=pods,verbs=get;list
//+kubebuilder:rbac:groups="",resources=pods/eviction,verbs=create
//+kubebuilder:rbac:groups="apps",resources=daemonsets,verbs=get;delete

// Reconcile is part of the main kubernetes reconciliation loop which aims to
// move the current state of the cluster closer to the desired state.
func (r *OSReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	ctx = context.Background()
	osInstance, err := checkOsiExist(ctx, r, req.Namespace, r.hostName)
	if err != nil {
		return values.RequeueNow, err
	}
	osCr, node := getOSAndNodeStatus(ctx, r, req.NamespacedName, r.hostName)
	sameOSVersion := checkVersion(osCr.Spec.OSVersion, node.Status.NodeInfo.OSImage)
	if sameOSVersion {
		configOps, err := checkConfigVersion(osCr, osInstance, values.SysConfigName)
		if err != nil {
			return values.RequeueNow, err
		}
		if configOps == values.Reassign {
			if err = r.refreshNode(ctx, &node, osInstance, osCr.Spec.SysConfigs.Version, values.SysConfigName); err != nil {
				return values.RequeueNow, err
			}
			return values.RequeueNow, nil
		}
		if configOps == values.UpdateConfig {
			osInstance.Spec.SysConfigs = osCr.Spec.SysConfigs
			if err = r.Update(ctx, osInstance); err != nil {
				return values.RequeueNow, err
			}
			return values.RequeueNow, nil
		}
		if err := r.setConfig(ctx, osInstance, values.SysConfigName); err != nil {
			return values.RequeueNow, err
		}
		if err = r.refreshNode(ctx, &node, osInstance, osCr.Spec.SysConfigs.Version,
			values.SysConfigName); err != nil {
			return values.RequeueNow, err
		}
	} else {
		if osCr.Spec.OpsType == values.NodeStatusConfig.String() {
			log.Error(nil, "Expect OS Version is not same with Node OS Version, please upgrade first")
			return values.RequeueNow, err
		}
		configOps, err := checkConfigVersion(osCr, osInstance, values.UpgradeConfigName)
		if err != nil {
			return values.RequeueNow, err
		}
		if configOps == values.Reassign {
			if err = r.refreshNode(ctx, &node, osInstance, osCr.Spec.UpgradeConfigs.Version,
				values.UpgradeConfigName); err != nil {
				return values.RequeueNow, err
			}
			return values.RequeueNow, nil
		}
		if err := r.setConfig(ctx, osInstance, values.UpgradeConfigName); err != nil {
			return values.RequeueNow, err
		}
		if err = r.upgradeNode(ctx, &osCr, &node); err != nil {
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

func (r *OSReconciler) upgradeNode(ctx context.Context, osCr *upgradev1.OS, node *corev1.Node) error {
	osVersionSpec := osCr.Spec.OSVersion
	if _, ok := node.Labels[values.LabelUpgrading]; ok {
		drainer := &drain.Helper{
			Ctx:                ctx,
			Client:             r.kubeclientset,
			GracePeriodSeconds: -1,
			Out:                os.Stdout,
			ErrOut:             os.Stderr,
		}
		if osCr.Spec.EvictPodForce {
			drainer.DeleteEmptyDirData = true
			drainer.IgnoreAllDaemonSets = true
			drainer.Force = true
		}
		if err := evictNode(drainer, node); err != nil {
			return err
		}
		opsType := osCr.Spec.OpsType
		switch opsType {
		case "upgrade":
			version := osVersionSpec
			downloadInfo := &agentclient.DownloadInfo{
				ImageURL:       osCr.Spec.ImageURL,
				FlagSafe:       osCr.Spec.FlagSafe,
				CheckSum:       osCr.Spec.CheckSum,
				CaCert:         osCr.Spec.CaCert,
				ClientCert:     osCr.Spec.ClientCert,
				ClientKey:      osCr.Spec.ClientKey,
				MTLS:           osCr.Spec.MTLS,
				ImageType:      osCr.Spec.ImageType,
				ContainerImage: osCr.Spec.ContainerImage,
			}
			if err := r.Connection.UpdateSpec(version, downloadInfo); err != nil {
				return err
			}
		case "rollback":
			if err := r.Connection.RollbackSpec(); err != nil {
				return err
			}
		default:
			return fmt.Errorf("operation %s cannot be recognized", opsType)
		}
	}
	return nil
}

func (r *OSReconciler) refreshNode(ctx context.Context, node *corev1.Node, osInstance *upgradev1.OSInstance,
	osConfigVersion string, configType string) error {
	if _, ok := node.Labels[values.LabelUpgrading]; ok {
		delete(node.Labels, values.LabelUpgrading)
		if err := r.Update(ctx, node); err != nil {
			log.Error(err, "unable to delete label", "node", node.Name)
			return err
		}
	}
	if node.Spec.Unschedulable { // update done, uncordon the node
		drainer := &drain.Helper{
			Ctx:                ctx,
			Client:             r.kubeclientset,
			GracePeriodSeconds: -1,
			Out:                os.Stdout,
			ErrOut:             os.Stderr,
		}
		if err := drain.RunCordonOrUncordon(drainer, node, false); err != nil {
			return err
		}
		log.Info("Uncordon successfully", "node", node.Name)
	}
	if err := updateNodeStatus(ctx, r, osInstance, osConfigVersion, configType); err != nil {
		log.Error(err, "unable to change osInstance nodeStatus to idle")
		return err
	}
	return nil
}

func checkOsiExist(ctx context.Context, r common.ReadStatusWriter, nameSpace string,
	nodeName string) (*upgradev1.OSInstance, error) {
	var osInstance upgradev1.OSInstance
	if err := r.Get(ctx, types.NamespacedName{
		Namespace: nameSpace,
		Name:      nodeName,
	}, &osInstance); err != nil {
		if errors.IsNotFound(err) {
			log.Info("Create OSInstance")
			osInstance = upgradev1.OSInstance{
				ObjectMeta: metav1.ObjectMeta{
					Namespace: nameSpace,
					Name:      nodeName,
					Labels: map[string]string{
						values.LabelOSinstance: nodeName,
					},
				},
			}
			osInstance.Spec.NodeStatus = values.NodeStatusIdle.String()
			if err = r.Create(ctx, &osInstance); err != nil {
				log.Error(err, "Error create OSInstance ")
				return &upgradev1.OSInstance{}, err
			}
		} else {
			log.Error(err, "Error Get OSInstance ")
			return &upgradev1.OSInstance{}, err
		}
	}
	return &osInstance, nil
}

func updateNodeStatus(ctx context.Context, r common.ReadStatusWriter, osInstance *upgradev1.OSInstance,
	osConfigVersion string, configType string) error {
	if osInstance.Spec.NodeStatus == values.NodeStatusIdle.String() {
		return nil
	}
	// Change nodeStatus to idle, when
	// 1.complte config or no config(conVersionStatus == conVersionSpec and nodestatus is config or upgrade)
	// 2.os.spec.sysconfig/upgradeconfig.version is not equals to osInstance.spec.sysconfig/upgradeconfig.version ,
	// that means when configuring or upgrading config was changed again , so
	conVersionSpec := osInstance.Spec.SysConfigs.Version
	conVersionStatus := osInstance.Status.SysConfigs.Version
	if (conVersionStatus == conVersionSpec) ||
		(configType == values.SysConfigName && osConfigVersion != osInstance.Spec.SysConfigs.Version) ||
		(configType == values.UpgradeConfigName && osConfigVersion != osInstance.Spec.UpgradeConfigs.Version) {
		if err := retry.RetryOnConflict(retry.DefaultBackoff, func() (err error) {
			if err = r.Get(ctx, client.ObjectKey{Name: osInstance.Name, Namespace: osInstance.Namespace},
				osInstance); err != nil {
				return err
			}
			osInstance.Spec.NodeStatus = values.NodeStatusIdle.String()
			return r.Update(ctx, osInstance)
		}); err != nil {
			return err
		}
	}
	return nil
}

func updateConfigStatus(ctx context.Context, r common.ReadStatusWriter, osInstance *upgradev1.OSInstance,
	configType string) error {
	switch configType {
	case values.UpgradeConfigName:
		osInstance.Status.UpgradeConfigs = osInstance.Spec.UpgradeConfigs
	case values.SysConfigName:
		osInstance.Status.SysConfigs = osInstance.Spec.SysConfigs
	default:
		log.Error(nil, "Cannot recognize configType: "+configType)
	}
	if err := r.Status().Update(ctx, osInstance); err != nil {
		log.Error(err, "Update OSInstance Error")
		return err
	}
	return nil
}

func (r *OSReconciler) setConfig(ctx context.Context, osInstance *upgradev1.OSInstance, configType string) error {
	expectConfigVersion, curConfigVersion, configs := getConfigs(osInstance, configType)
	if expectConfigVersion != curConfigVersion {
		var sysConfigs []agentclient.SysConfig
		for _, config := range configs {
			configTmp := agentclient.SysConfig{
				Model:      config.Model,
				ConfigPath: config.ConfigPath,
			}
			contentsTmp := make(map[string]agentclient.KeyInfo)
			for _, content := range config.Contents {
				contentsTmp[content.Key] = agentclient.KeyInfo{
					Value:     content.Value,
					Operation: content.Operation,
				}
			}
			configTmp.Contents = contentsTmp
			sysConfigs = append(sysConfigs, configTmp)
		}
		configInfo := &agentclient.ConfigsInfo{Configs: sysConfigs}
		if err := r.Connection.ConfigureSpec(configInfo); err != nil {
			log.Error(err, "configure Error")
			return err
		}
		if err := updateConfigStatus(ctx, r, osInstance, configType); err != nil {
			return err
		}
		return nil
	}
	return nil
}

func getConfigs(osInstance *upgradev1.OSInstance, configType string) (string, string, []upgradev1.SysConfig) {
	var expectConfigVersion, curConfigVersion string
	var configs []upgradev1.SysConfig
	switch configType {
	case values.UpgradeConfigName:
		expectConfigVersion = osInstance.Spec.UpgradeConfigs.Version
		curConfigVersion = osInstance.Status.UpgradeConfigs.Version
		configs = osInstance.Spec.UpgradeConfigs.Configs
	case values.SysConfigName:
		expectConfigVersion = osInstance.Spec.SysConfigs.Version
		curConfigVersion = osInstance.Status.SysConfigs.Version
		configs = osInstance.Spec.SysConfigs.Configs
	default:
		log.Error(nil, "Cannot recognize configType: "+configType)
	}
	return expectConfigVersion, curConfigVersion, configs
}

func checkVersion(versionA string, versionB string) bool {
	return versionA == versionB
}

func checkConfigVersion(os upgradev1.OS, osInstance *upgradev1.OSInstance,
	configType string) (values.ConfigOperation, error) {
	nodeStatus := osInstance.Spec.NodeStatus
	if nodeStatus == values.NodeStatusIdle.String() {
		return values.DoNothing, nil
	}
	// check if os.spec.sysconfig/upgradeconfig.version is equal to
	// osInstance.spec.sysconfig/upgradeconfig.version,
	// if not configs may be changed during upgrading or configuring.
	// 1、For upgradeconfig , refresh the node to enable the operator to
	// reallocate upgrade tasks and obtain the latest config.
	// 2、For sysconfig :
	// When nodestatus=config, refresh the node to enable the operator to
	// reallocate configuration tasks and obtain
	// the latest config.
	// When nodestatus=upgrade, OS reboot is complete, the configuration task cannot be delivered again.
	// Therefore, the system obtains the latest version configuration and updates the osInstance.sysconfig file.
	var osConfigVersion, osiConfigVersion string
	switch configType {
	case values.UpgradeConfigName:
		osConfigVersion = os.Spec.UpgradeConfigs.Version
		osiConfigVersion = osInstance.Spec.UpgradeConfigs.Version
		if !checkVersion(osConfigVersion, osiConfigVersion) {
			log.Info("os.spec.upgradeconfig version is not equals to osInstance.spec.upgradeconfig.version,",
				"operation:", "reassgin upragde to get newest upgradeconfig")
			return values.Reassign, nil
		}
	case values.SysConfigName:
		osConfigVersion = os.Spec.SysConfigs.Version
		osiConfigVersion = osInstance.Spec.SysConfigs.Version
		if !checkVersion(osConfigVersion, osiConfigVersion) {
			if nodeStatus == values.NodeStatusConfig.String() {
				log.Info("os.spec.sysconfig version is not equals to osInstance.spec.sysconfig.version,",
					"operation:", "reassgin config to get newest sysconfig")
				return values.Reassign, nil
			}
			if nodeStatus == values.NodeStatusUpgrade.String() {
				log.Info("os.spec.sysconfig version is not equals to osInstance.spec.sysconfig.version,",
					"operation:", "update osInstance.spec.sysconfig and reconcile")

				return values.UpdateConfig, nil
			}
		}
	default:
		return "", fmt.Errorf("operation %s cannot be recognized", configType)

	}
	return values.DoNothing, nil
}
