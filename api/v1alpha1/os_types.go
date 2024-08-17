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

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// OSSpec defines the desired state of OS
type OSSpec struct {
	OSVersion      string `json:"osversion"`
	ImageURL       string `json:"imageurl"`
	MaxUnavailable int    `json:"maxunavailable"`
	CheckSum       string `json:"checksum"`
	FlagSafe       bool   `json:"flagSafe"`
	MTLS           bool   `json:"mtls"`
	// +kubebuilder:validation:Enum=docker;disk;containerd
	ImageType      string `json:"imagetype"`
	ContainerImage string `json:"containerimage"`
	// +kubebuilder:validation:Enum=upgrade;config;rollback
	OpsType       string `json:"opstype"`
	EvictPodForce bool   `json:"evictpodforce"`
	// +kubebuilder:validation:Optional
	CaCert string `json:"cacert"`
	// +kubebuilder:validation:Optional
	ClientCert string `json:"clientcert"`
	// +kubebuilder:validation:Optional
	ClientKey string `json:"clientkey"`
	// +kubebuilder:validation:Optional
	SysConfigs SysConfigs `json:"sysconfigs"`
	// +kubebuilder:validation:Optional
	UpgradeConfigs SysConfigs `json:"upgradeconfigs"`
	// +kubebuilder:validation:Optional
	// +kubebuilder:default:=no-label
	NodeSelector string `json:"nodeselector"`
	// +kubebuilder:validation:Optional
	TimeWindow TimeWindow `json:"timewindow"`
	// +kubebuilder:validation:Optional
	TimeInterval int `json:"timeinterval"`
	// +kubebuilder:validation:Optional
	// +kubebuilder:validation:Enum=serial;parallel
	// +kubebuilder:default:=parallel
	ExecutionMode string `json:"executionmode"`
}

// +kubebuilder:subresource:status
// +kubebuilder:object:root=true

// OS is a specification for OS in the cluster
type OS struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec OSSpec `json:"spec,omitempty"`
}

// +kubebuilder:object:root=true

// OSList is a list of OS resources
type OSList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []OS `json:"items"`
}

// SysConfigs defines all configurations expected by the user
type SysConfigs struct {
	// +kubebuilder:validation:Optional
	Version string `json:"version"`
	// +kubebuilder:validation:Optional
	Configs []SysConfig `json:"configs"`
}

// SysConfig defines a type of configurations expected by the user
type SysConfig struct {
	// +kubebuilder:validation:Optional
	Model string `json:"model"`
	// +kubebuilder:validation:Optional
	ConfigPath string `json:"configpath"`
	// +kubebuilder:validation:Optional
	Contents []Content `json:"contents"`
}

// Content defines the key and value of configuration
type Content struct {
	// +kubebuilder:validation:Optional
	Key string `json:"key"`
	// +kubebuilder:validation:Optional
	Value string `json:"value"`
	// +kubebuilder:validation:Optional
	Operation string `json:"operation"`
}

type TimeWindow struct {
	StartTime string `json:"starttime"`
	EndTime   string `json:"endtime"`
}

// +kubebuilder:subresource:status
// +kubebuilder:object:root=true

// OSInstance defines some infomation of a node
type OSInstance struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`
	// +kubebuilder:validation:Optional
	Status OSInstanceStatus `json:"status,omitempty"`
	// +kubebuilder:validation:Optional
	Spec OSInstanceSpec `json:"spec,omitempty"`
}

// OSInstanceStatus defines status of a node
type OSInstanceStatus struct {
	// +kubebuilder:validation:Optional
	SysConfigs SysConfigs `json:"sysconfigs"`
	// +kubebuilder:validation:Optional
	UpgradeConfigs SysConfigs `json:"upgradeconfigs"`
}

// OSInstanceSpec defines desired state of OS
type OSInstanceSpec struct {
	// +kubebuilder:validation:Optional
	NodeStatus string `json:"nodestatus"`
	// +kubebuilder:validation:Optional
	SysConfigs SysConfigs `json:"sysconfigs"`
	// +kubebuilder:validation:Optional
	UpgradeConfigs SysConfigs `json:"upgradeconfigs"`
}

// +kubebuilder:object:root=true

// OSInstanceList is a list of OSInstance
type OSInstanceList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []OSInstance `json:"items"`
}

func init() {
	SchemeBuilder.Register(&OS{}, &OSList{})
	SchemeBuilder.Register(&OSInstance{}, &OSInstanceList{})
}
