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
	// +kubebuilder:default=true
	MTLS        bool   `json:"mtls"`
	ImageType   string `json:"imagetype"`
	DockerImage string `json:"dockerimage"`
	OpsType     string `json:"opstype"`
	CaCert      string `json:"cacert"`
	ClientCert  string `json:"clientcert"`
	ClientKey   string `json:"clientkey"`
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

func init() {
	SchemeBuilder.Register(&OS{}, &OSList{})
}
