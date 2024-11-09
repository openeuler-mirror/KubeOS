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

package controllers

import (
	"context"
	"encoding/json"
	"fmt"
	"reflect"
	"testing"
	"time"

	"github.com/agiledragon/gomonkey/v2"
	"github.com/google/uuid"
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	v1 "k8s.io/api/core/v1"
	apiextensions "k8s.io/apiextensions-apiserver/pkg/apis/apiextensions/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/types"
	upgradev1 "openeuler.org/KubeOS/api/v1alpha1"
	"openeuler.org/KubeOS/pkg/common"
	"openeuler.org/KubeOS/pkg/values"
)

var _ = Describe("OsController", func() {
	const (
		OSName   = "test-os"
		timeout  = time.Second * 20
		interval = time.Millisecond * 500
	)
	var testNamespace string
	var node1Name string

	BeforeEach(func() {
		var generatedTestNamespace = "test-namespace-" + uuid.New().String()
		// Add any setup steps that needs to be executed before each test
		desiredTestNamespace := &v1.Namespace{
			TypeMeta: metav1.TypeMeta{
				APIVersion: "v1",
				Kind:       "Namespace",
			},
			ObjectMeta: metav1.ObjectMeta{
				Name: generatedTestNamespace,
			},
		}

		err := k8sClient.Create(context.Background(), desiredTestNamespace)
		Expect(err).ToNot(HaveOccurred())

		existingNamespace := &v1.Namespace{}
		Eventually(func() bool {
			err := k8sClient.Get(context.Background(),
				types.NamespacedName{Name: generatedTestNamespace}, existingNamespace)
			return err == nil
		}, timeout, interval).Should(BeTrue())

		testNamespace = existingNamespace.Name
	})

	AfterEach(func() {
		// delete all nodes
		nodeList := &v1.NodeList{}
		err := k8sClient.List(context.Background(), nodeList)
		Expect(err).ToNot(HaveOccurred())
		for _, node := range nodeList.Items {
			k8sClient.Delete(context.Background(), &node)
		}
		nodeList = &v1.NodeList{}
		Eventually(func() bool {
			err = k8sClient.List(context.Background(), nodeList)
			if err != nil || len(nodeList.Items) != 0 {
				return false
			}
			return true
		}, timeout, interval).Should(BeTrue())

		// delete all OS CRs
		osList := &upgradev1.OSList{}
		err = k8sClient.List(context.Background(), osList)
		Expect(err).ToNot(HaveOccurred())
		for _, os := range osList.Items {
			k8sClient.Delete(context.Background(), &os)
		}
		osList = &upgradev1.OSList{}
		Eventually(func() bool {
			err = k8sClient.List(context.Background(), osList)
			if err != nil || len(osList.Items) != 0 {
				return false
			}
			return true
		}, timeout, interval).Should(BeTrue())
	})

	Context("When we change the OSVersion to previous version and Opstype is rollback", func() {
		It("Should label the osinstance's nodestatus to upgrading", func() {
			ctx := context.Background()

			// create Node1
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v2",
					},
				},
			}
			err := k8sClient.Create(ctx, node1)
			Expect(err).ToNot(HaveOccurred())
			existingNode := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance1
			OSIns := &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node1Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs: upgradev1.SysConfigs{
						Version: "v1",
						Configs: []upgradev1.SysConfig{},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			// Check that the corresponding OSIns CR has been created
			osInsCRLookupKey := types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node1Name))

			// create Node2
			node2Name := "test-node-" + uuid.New().String()
			node2 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v2",
					},
				},
			}
			err = k8sClient.Create(ctx, node2)
			Expect(err).ToNot(HaveOccurred())
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node2Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance2
			OSIns = &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node2Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs: upgradev1.SysConfigs{
						Version: "v1",
						Configs: []upgradev1.SysConfig{},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}, Version: "v1"},
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			// Check that the corresponding OSIns CR has been created
			osInsCRLookupKey2 := types.NamespacedName{Name: node2Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node2Name))

			// create OS CR
			OS := &upgradev1.OS{
				TypeMeta: metav1.TypeMeta{
					APIVersion: "upgrade.openeuler.org/v1alpha1",
					Kind:       "OS",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      OSName,
					Namespace: testNamespace,
				},
				Spec: upgradev1.OSSpec{
					OpsType:        "rollback",
					MaxUnavailable: 3,
					OSVersion:      "KubeOS v1",
					FlagSafe:       true,
					MTLS:           false,
					EvictPodForce:  true,
					SysConfigs: upgradev1.SysConfigs{
						Version: "v1",
						Configs: []upgradev1.SysConfig{},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
				},
			}
			Expect(k8sClient.Create(ctx, OS)).Should(Succeed())

			// Check that the corresponding OS CR has been created
			osCRLookupKey := types.NamespacedName{Name: OSName, Namespace: testNamespace}
			createdOS := &upgradev1.OS{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osCRLookupKey, createdOS)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOS.Spec.OSVersion).Should(Equal("KubeOS v1"))

			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			osInsCRLookupKey = types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Spec.NodeStatus).Should(Equal(values.NodeStatusUpgrade.String()))

			createdOSIns2 := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, createdOSIns2)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns2.Spec.NodeStatus).Should(Equal(values.NodeStatusUpgrade.String()))
		})
	})

	Context("When we want to configure node", func() {
		It("Should update OSInstance spec and update NodeStatus to config", func() {
			ctx := context.Background()
			// create Node1
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v1",
					},
				},
			}
			err := k8sClient.Create(ctx, node1)
			Expect(err).ToNot(HaveOccurred())
			existingNode := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance1
			OSIns := &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node1Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs: upgradev1.SysConfigs{
						Version: "v1",
						Configs: []upgradev1.SysConfig{},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
					NodeStatus:     values.NodeStatusIdle.String(),
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			osInsCRLookupKey1 := types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey1, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node1Name))

			// create Node2
			node2Name := "test-node-" + uuid.New().String()
			node2 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v1",
					},
				},
			}
			err = k8sClient.Create(ctx, node2)
			Expect(err).ToNot(HaveOccurred())
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node2Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance2
			OSIns = &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node2Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs: upgradev1.SysConfigs{
						Version: "v1",
						Configs: []upgradev1.SysConfig{},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}, Version: "v1"},
					NodeStatus:     values.NodeStatusIdle.String(),
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			// Check that the corresponding OSIns CR has been created
			osInsCRLookupKey2 := types.NamespacedName{Name: node2Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node2Name))

			var value1, value2 apiextensions.JSON
			value1.Raw, err = json.Marshal("a")
			Expect(err).ToNot(HaveOccurred())
			value2.Raw, err = json.Marshal("b")
			Expect(err).ToNot(HaveOccurred())
			OS := &upgradev1.OS{
				TypeMeta: metav1.TypeMeta{
					APIVersion: "upgrade.openeuler.org/v1alpha1",
					Kind:       "OS",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      OSName,
					Namespace: testNamespace,
				},
				Spec: upgradev1.OSSpec{
					OpsType:        "config",
					MaxUnavailable: 3,
					OSVersion:      "KubeOS v1",
					FlagSafe:       true,
					MTLS:           false,
					EvictPodForce:  true,
					SysConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: value1},
									{Key: "key2", Value: value2},
								},
							},
						},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
				},
			}
			Expect(k8sClient.Create(ctx, OS)).Should(Succeed())

			osCRLookupKey := types.NamespacedName{Name: OSName, Namespace: testNamespace}
			createdOS := &upgradev1.OS{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osCRLookupKey, createdOS)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOS.Spec.OSVersion).Should(Equal("KubeOS v1"))

			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			configedOSIns1 := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey1, configedOSIns1)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(configedOSIns1.Spec.NodeStatus).Should(Equal(values.NodeStatusConfig.String()))
			Expect(configedOSIns1.Spec.SysConfigs.Version).Should(Equal("v2"))
			existingNode1 := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node2Name, Namespace: testNamespace}, existingNode1)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok := existingNode1.Labels[values.LabelConfiguring]
			Expect(ok).Should(Equal(true))

			configedOSIns2 := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, configedOSIns2)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(configedOSIns2.Spec.NodeStatus).Should(Equal(values.NodeStatusConfig.String()))
			Expect(configedOSIns2.Spec.SysConfigs.Version).Should(Equal("v2"))
			existingNode2 := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node2Name, Namespace: testNamespace}, existingNode2)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok = existingNode2.Labels[values.LabelConfiguring]
			Expect(ok).Should(Equal(true))
		})
	})

	Context("When we deploy OS, but there is a node without osinstance", func() {
		It("Should not label upgrading and skip that node", func() {
			ctx := context.Background()
			// create Node
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v1",
					},
				},
			}
			err := k8sClient.Create(ctx, node1)
			Expect(err).ToNot(HaveOccurred())
			existingNode := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			OS := &upgradev1.OS{
				TypeMeta: metav1.TypeMeta{
					APIVersion: "upgrade.openeuler.org/v1alpha1",
					Kind:       "OS",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      OSName,
					Namespace: testNamespace,
				},
				Spec: upgradev1.OSSpec{
					OpsType:        "upgrade",
					MaxUnavailable: 3,
					OSVersion:      "KubeOS v2",
					FlagSafe:       true,
					MTLS:           false,
					EvictPodForce:  true,
					SysConfigs: upgradev1.SysConfigs{
						Configs: []upgradev1.SysConfig{},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
				},
			}
			Expect(k8sClient.Create(ctx, OS)).Should(Succeed())

			osCRLookupKey := types.NamespacedName{Name: OSName, Namespace: testNamespace}
			createdOS := &upgradev1.OS{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osCRLookupKey, createdOS)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOS.Spec.OSVersion).Should(Equal("KubeOS v2"))

			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok := existingNode.Labels[values.LabelUpgrading]
			Expect(ok).Should(Equal(false))

			createdOS.Spec.OpsType = "test"
			Expect(k8sClient.Update(ctx, createdOS)).Should(Succeed())
		})
	})

	Context("When we want to upgrade and do sysconfigs which contain grub.cmd.current and .next", func() {
		It("Should exchange .current and .next", func() {
			ctx := context.Background()

			// create Node1
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v1",
					},
				},
			}
			err := k8sClient.Create(ctx, node1)
			Expect(err).ToNot(HaveOccurred())
			existingNode := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance1
			OSIns := &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node1Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs: upgradev1.SysConfigs{
						Version: "v1",
						Configs: []upgradev1.SysConfig{},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			// Check that the corresponding OSIns CR has been created
			osInsCRLookupKey1 := types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey1, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node1Name))

			// create Node2
			node2Name := "test-node-" + uuid.New().String()
			node2 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v1",
					},
				},
			}
			err = k8sClient.Create(ctx, node2)
			Expect(err).ToNot(HaveOccurred())
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node2Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance2
			OSIns = &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node2Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs: upgradev1.SysConfigs{
						Version: "v1",
						Configs: []upgradev1.SysConfig{},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}, Version: "v1"},
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			// Check that the corresponding OSIns CR has been created
			osInsCRLookupKey2 := types.NamespacedName{Name: node2Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node2Name))

			// create OS CR
			var value1, value2 apiextensions.JSON
			value1.Raw, err = json.Marshal("1")
			Expect(err).ToNot(HaveOccurred())
			value2.Raw, err = json.Marshal("2")
			Expect(err).ToNot(HaveOccurred())
			OS := &upgradev1.OS{
				TypeMeta: metav1.TypeMeta{
					APIVersion: "upgrade.openeuler.org/v1alpha1",
					Kind:       "OS",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      OSName,
					Namespace: testNamespace,
				},
				Spec: upgradev1.OSSpec{
					OpsType:        "upgrade",
					MaxUnavailable: 3,
					OSVersion:      "KubeOS v2",
					FlagSafe:       true,
					MTLS:           false,
					EvictPodForce:  true,
					SysConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{Model: "grub.cmdline.current", Contents: []upgradev1.Content{{Key: "a", Value: value1}}},
							{Model: "grub.cmdline.next", Contents: []upgradev1.Content{{Key: "b", Value: value2}}},
						},
					},
					UpgradeConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{Model: "grub.cmdline.current", Contents: []upgradev1.Content{{Key: "a", Value: value1}}},
							{Model: "grub.cmdline.next", Contents: []upgradev1.Content{{Key: "b", Value: value2}}},
						},
					},
				},
			}
			Expect(k8sClient.Create(ctx, OS)).Should(Succeed())

			// Check that the corresponding OS CR has been created
			osCRLookupKey := types.NamespacedName{Name: OSName, Namespace: testNamespace}
			createdOS := &upgradev1.OS{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osCRLookupKey, createdOS)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOS.Spec.OSVersion).Should(Equal("KubeOS v2"))

			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			// check node1 osinstance
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey1, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Spec.SysConfigs.Configs[0]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.next", Contents: []upgradev1.Content{{Key: "a", Value: value1}}}))
			Expect(createdOSIns.Spec.SysConfigs.Configs[1]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.current", Contents: []upgradev1.Content{{Key: "b", Value: value2}}}))
			Expect(createdOSIns.Spec.UpgradeConfigs.Configs[0]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.current", Contents: []upgradev1.Content{{Key: "a", Value: value1}}}))
			Expect(createdOSIns.Spec.NodeStatus).Should(Equal(values.NodeStatusUpgrade.String()))

			// check node2 osinstance
			createdOSIns2 := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, createdOSIns2)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns2.Spec.NodeStatus).Should(Equal(values.NodeStatusUpgrade.String()))
			Expect(createdOSIns2.Spec.SysConfigs.Configs[0]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.next", Contents: []upgradev1.Content{{Key: "a", Value: value1}}}))
			Expect(createdOSIns2.Spec.SysConfigs.Configs[1]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.current", Contents: []upgradev1.Content{{Key: "b", Value: value2}}}))
			Expect(createdOSIns2.Spec.UpgradeConfigs.Configs[0]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.current", Contents: []upgradev1.Content{{Key: "a", Value: value1}}}))

			// check os cr spec
			osCRLookupKey = types.NamespacedName{Name: OSName, Namespace: testNamespace}
			createdOS = &upgradev1.OS{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osCRLookupKey, createdOS)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOS.Spec.SysConfigs.Configs[0]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.current", Contents: []upgradev1.Content{{Key: "a", Value: value1}}}))
			Expect(createdOS.Spec.SysConfigs.Configs[1]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.next", Contents: []upgradev1.Content{{Key: "b", Value: value2}}}))
		})
	})

	Context("When we want to upgrade node with nodes having NodeSelector label", func() {
		It("Should only update node with NodeSelector label", func() {
			ctx := context.Background()
			// create Node1
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os":               "linux",
						"upgrade.openeuler.org/node-selector": "openeuler",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v1",
					},
				},
			}
			err := k8sClient.Create(ctx, node1)
			Expect(err).ToNot(HaveOccurred())
			existingNode := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance1
			OSIns := &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node1Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs: upgradev1.SysConfigs{
						Version: "v1",
						Configs: []upgradev1.SysConfig{},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
					NodeStatus:     values.NodeStatusIdle.String(),
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			osInsCRLookupKey1 := types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey1, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node1Name))

			// create Node2
			node2Name := "test-node-" + uuid.New().String()
			node2 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v1",
					},
				},
			}
			err = k8sClient.Create(ctx, node2)
			Expect(err).ToNot(HaveOccurred())
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node2Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance2
			OSIns = &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node2Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs:     upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}, Version: "v1"},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}, Version: "v1"},
					NodeStatus:     values.NodeStatusIdle.String(),
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			// Check that the corresponding OSIns CR has been created
			osInsCRLookupKey2 := types.NamespacedName{Name: node2Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node2Name))

			var value1, value2 apiextensions.JSON
			value1.Raw, err = json.Marshal("a")
			Expect(err).ToNot(HaveOccurred())
			value2.Raw, err = json.Marshal("b")
			Expect(err).ToNot(HaveOccurred())
			OS := &upgradev1.OS{
				TypeMeta: metav1.TypeMeta{
					APIVersion: "upgrade.openeuler.org/v1alpha1",
					Kind:       "OS",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      OSName,
					Namespace: testNamespace,
				},
				Spec: upgradev1.OSSpec{
					OpsType:        "upgrade",
					MaxUnavailable: 3,
					OSVersion:      "KubeOS v2",
					FlagSafe:       true,
					MTLS:           false,
					EvictPodForce:  true,
					NodeSelector:   "openeuler",
					SysConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: value1},
									{Key: "key2", Value: value2},
								},
							},
						},
					},
					UpgradeConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{Model: "kernel.sysctl.persist",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: value1},
									{Key: "key2", Value: value2},
								},
							}},
					},
				},
			}
			Expect(k8sClient.Create(ctx, OS)).Should(Succeed())

			osCRLookupKey := types.NamespacedName{Name: OSName, Namespace: testNamespace}
			createdOS := &upgradev1.OS{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osCRLookupKey, createdOS)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOS.Spec.OSVersion).Should(Equal("KubeOS v2"))

			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			existingNode1 := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode1)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok := existingNode1.Labels[values.LabelUpgrading]
			Expect(ok).Should(Equal(true))

			upgradeOSIns1 := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey1, upgradeOSIns1)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(upgradeOSIns1.Spec.NodeStatus).Should(Equal(values.NodeStatusUpgrade.String()))
			Expect(upgradeOSIns1.Spec.UpgradeConfigs.Version).Should(Equal("v2"))
			Expect(upgradeOSIns1.Spec.SysConfigs.Version).Should(Equal("v2"))

			existingNode2 := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node2Name, Namespace: testNamespace}, existingNode2)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok = existingNode2.Labels[values.LabelUpgrading]
			Expect(ok).Should(Equal(false))

			upgradeOSIns2 := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, upgradeOSIns2)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(upgradeOSIns2.Spec.NodeStatus).Should(Equal(values.NodeStatusIdle.String()))
			Expect(upgradeOSIns2.Spec.UpgradeConfigs.Version).Should(Equal("v1"))
			Expect(upgradeOSIns2.Spec.SysConfigs.Version).Should(Equal("v1"))
		})
	})

	Context("When we want to config node with nodes having NodeSelector label", func() {
		It("Should only config node with NodeSelector label", func() {
			ctx := context.Background()
			// create Node1
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os":               "linux",
						"upgrade.openeuler.org/node-selector": "openeuler",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v1",
					},
				},
			}
			err := k8sClient.Create(ctx, node1)
			Expect(err).ToNot(HaveOccurred())
			existingNode := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance1
			OSIns := &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node1Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs:     upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}, Version: "v1"},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}, Version: "v1"},
					NodeStatus:     values.NodeStatusIdle.String(),
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			osInsCRLookupKey1 := types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey1, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node1Name))

			// create Node2
			node2Name := "test-node-" + uuid.New().String()
			node2 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
					},
				},
				TypeMeta: metav1.TypeMeta{
					APIVersion: "v1",
					Kind:       "Node",
				},
				Status: v1.NodeStatus{
					NodeInfo: v1.NodeSystemInfo{
						OSImage: "KubeOS v1",
					},
				},
			}
			err = k8sClient.Create(ctx, node2)
			Expect(err).ToNot(HaveOccurred())
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node2Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())

			// create OSInstance2
			OSIns = &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node2Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						values.LabelOSinstance: node2Name,
					},
				},
				Spec: upgradev1.OSInstanceSpec{
					SysConfigs:     upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}, Version: "v1"},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}, Version: "v1"},
					NodeStatus:     values.NodeStatusIdle.String(),
				},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			// Check that the corresponding OSIns CR has been created
			osInsCRLookupKey2 := types.NamespacedName{Name: node2Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node2Name))

			var value1, value2 apiextensions.JSON
			value1.Raw, err = json.Marshal("a")
			Expect(err).ToNot(HaveOccurred())
			value2.Raw, err = json.Marshal("b")
			Expect(err).ToNot(HaveOccurred())
			OS := &upgradev1.OS{
				TypeMeta: metav1.TypeMeta{
					APIVersion: "upgrade.openeuler.org/v1alpha1",
					Kind:       "OS",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      OSName,
					Namespace: testNamespace,
				},
				Spec: upgradev1.OSSpec{
					OpsType:        "config",
					MaxUnavailable: 3,
					OSVersion:      "KubeOS v1",
					FlagSafe:       true,
					MTLS:           false,
					EvictPodForce:  true,
					NodeSelector:   "openeuler",
					SysConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: value1},
									{Key: "key2", Value: value2},
								},
							},
						},
					},
				},
			}
			Expect(k8sClient.Create(ctx, OS)).Should(Succeed())

			osCRLookupKey := types.NamespacedName{Name: OSName, Namespace: testNamespace}
			createdOS := &upgradev1.OS{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osCRLookupKey, createdOS)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOS.Spec.SysConfigs.Version).Should(Equal("v2"))

			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			existingNode1 := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode1)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok := existingNode1.Labels[values.LabelConfiguring]
			Expect(ok).Should(Equal(true))

			upgradeOSIns1 := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey1, upgradeOSIns1)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(upgradeOSIns1.Spec.NodeStatus).Should(Equal(values.NodeStatusConfig.String()))
			Expect(upgradeOSIns1.Spec.SysConfigs.Version).Should(Equal("v2"))

			existingNode2 := &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node2Name, Namespace: testNamespace}, existingNode2)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok = existingNode2.Labels[values.LabelConfiguring]
			Expect(ok).Should(Equal(false))

			upgradeOSIns2 := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey2, upgradeOSIns2)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(upgradeOSIns2.Spec.NodeStatus).Should(Equal(values.NodeStatusIdle.String()))
			Expect(upgradeOSIns2.Spec.SysConfigs.Version).Should(Equal("v1"))
		})
	})
})

func Test_deepCopySpecConfigs(t *testing.T) {
	type args struct {
		os         *upgradev1.OS
		osinstance *upgradev1.OSInstance
		configType string
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{
			name: "error",
			args: args{
				os:         &upgradev1.OS{},
				osinstance: &upgradev1.OSInstance{},
				configType: "test"},
			wantErr: true,
		},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := deepCopySpecConfigs(tt.args.os, tt.args.osinstance, tt.args.configType); (err != nil) != tt.wantErr {
				t.Errorf("deepCopySpecConfigs() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func Test_getNodes(t *testing.T) {
	type args struct {
		ctx   context.Context
		r     common.ReadStatusWriter
		limit int
		reqs  []labels.Requirement
	}
	tests := []struct {
		name    string
		args    args
		want    []v1.Node
		wantErr bool
	}{
		{
			name: "list error",
			args: args{
				ctx:   context.Background(),
				r:     &OSReconciler{},
				limit: 1,
			},
			want:    nil,
			wantErr: true,
		},
	}
	patchList := gomonkey.ApplyMethodSeq(&OSReconciler{}, "List", []gomonkey.OutputCell{
		{Values: gomonkey.Params{fmt.Errorf("list error")}},
	})
	defer patchList.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := getNodes(tt.args.ctx, tt.args.r, tt.args.limit, tt.args.reqs...)
			if (err != nil) != tt.wantErr {
				t.Errorf("getNodes() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("getNodes() = %v, want %v", got, tt.want)
			}
		})
	}
}

func Test_getOSCr(t *testing.T) {
	type args struct {
		ctx  context.Context
		r    common.ReadStatusWriter
		name types.NamespacedName
	}
	tests := []struct {
		name    string
		args    args
		want    upgradev1.OS
		wantErr bool
	}{
		{
			name: "get os cr error",
			args: args{
				ctx:  context.Background(),
				r:    &OSReconciler{},
				name: types.NamespacedName{Namespace: "test_ns", Name: "test"},
			},
			want:    upgradev1.OS{},
			wantErr: true,
		},
		{
			name: "check nodeselector error",
			args: args{
				ctx:  context.Background(),
				r:    &OSReconciler{},
				name: types.NamespacedName{Namespace: "test_ns", Name: "test"},
			},
			want:    upgradev1.OS{},
			wantErr: true,
		},
	}
	patchGet := gomonkey.ApplyMethodReturn(&OSReconciler{}, "Get", []gomonkey.OutputCell{
		{Values: gomonkey.Params{fmt.Errorf("get os error")}},
		{Values: gomonkey.Params{nil}},
	})
	defer patchGet.Reset()
	patchList := gomonkey.ApplyFunc(checkNodeSelector, func(_ context.Context, _ common.ReadStatusWriter, _ upgradev1.OS) error {
		return fmt.Errorf("check nodeselector error")
	})
	defer patchList.Reset()
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := getOSCr(tt.args.ctx, tt.args.r, tt.args.name)
			if (err != nil) != tt.wantErr {
				t.Errorf("getOSCr() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("getOSCr() = %v, want %v", got, tt.want)
			}
		})
	}
}
