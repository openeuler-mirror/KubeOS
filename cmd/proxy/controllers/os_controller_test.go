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
	"fmt"
	"reflect"
	"time"

	"github.com/agiledragon/gomonkey/v2"
	"github.com/google/uuid"
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	v1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/types"

	upgradev1 "openeuler.org/KubeOS/api/v1alpha1"
	"openeuler.org/KubeOS/pkg/agentclient"
	"openeuler.org/KubeOS/pkg/values"
)

var _ = Describe("OsController", func() {
	const (
		OSName = "test-os"

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

	Context("When we want to rollback", func() {
		It("Should be able to rollback to previous version", func() {
			ctx := context.Background()

			By("Creating a worker node")
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
						values.LabelUpgrading:   "",
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
			reconciler.hostName = node1Name

			By("Creating the corresponding OSInstance")
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
					NodeStatus:     values.NodeStatusUpgrade.String(),
					SysConfigs:     upgradev1.SysConfigs{},
					UpgradeConfigs: upgradev1.SysConfigs{},
				},
				Status: upgradev1.OSInstanceStatus{},
			}
			Expect(k8sClient.Create(ctx, OSIns)).Should(Succeed())

			// Check that the corresponding OSIns CR has been created
			osInsCRLookupKey := types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Spec.NodeStatus).Should(Equal(values.NodeStatusUpgrade.String()))

			// stub r.Connection.RollbackSpec()
			patchRollback := gomonkey.ApplyMethodReturn(reconciler.Connection, "RollbackSpec", nil)
			defer patchRollback.Reset()
			patchConfigure := gomonkey.ApplyMethodReturn(reconciler.Connection, "ConfigureSpec", nil)
			defer patchConfigure.Reset()

			By("Creating a OS custom resource")
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
					SysConfigs:     upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
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
			Expect(createdOS.Spec.OpsType).Should(Equal("rollback"))

			By("Changing the nodeinfo OSImage to previous version, pretending the rollback success")
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			existingNode.Status.NodeInfo.OSImage = "KubeOS v1"
			Expect(k8sClient.Status().Update(ctx, existingNode)).Should(Succeed())

			By("Changing the OS Spec config to trigger reconcile")
			createdOS = &upgradev1.OS{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osCRLookupKey, createdOS)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			createdOS.Spec.SysConfigs = upgradev1.SysConfigs{Version: "v1", Configs: []upgradev1.SysConfig{}}
			Expect(k8sClient.Update(ctx, createdOS)).Should(Succeed())

			time.Sleep(2 * time.Second) // sleep a while to make sure Reconcile finished
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			// NodeStatus changes to idle then operator can reassign configs to this node
			Expect(createdOSIns.Spec.NodeStatus).Should(Equal(values.NodeStatusIdle.String()))
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok := existingNode.Labels[values.LabelUpgrading]
			Expect(ok).Should(Equal(false))
		})
	})

	Context("When we have a sysconfig whose version is different from current OSInstance config version", func() {
		It("Should configure the node", func() {
			ctx := context.Background()

			By("Creating a worker node")
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
			reconciler.hostName = node1Name

			By("Creating the corresponding OSInstance")
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
					NodeStatus: values.NodeStatusConfig.String(),
					SysConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
								},
							},
						},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
				},
				Status: upgradev1.OSInstanceStatus{},
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

			// stub r.Connection.ConfigureSpec()
			patchConfigure := gomonkey.ApplyMethod(reflect.TypeOf(reconciler.Connection),
				"ConfigureSpec", func(_ *agentclient.Client, _ *agentclient.ConfigsInfo) error {
					return nil
				})
			defer patchConfigure.Reset()

			By("Creating a OS custom resource")
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
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
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

			By("Checking the OSInstance status config version")
			time.Sleep(2 * time.Second) // sleep a while to make sure Reconcile finished
			osInsCRLookupKey = types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Status.SysConfigs.Version).Should(Equal("v2"))
		})
	})

	Context("When we have a upgradeconfig whose version is different from current OSInstance config version", func() {
		It("Should configure the node", func() {
			ctx := context.Background()

			By("Creating a worker node")
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
						values.LabelUpgrading:   "",
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
			reconciler.hostName = node1Name

			By("Creating the corresponding OSInstance")
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
					NodeStatus: values.NodeStatusUpgrade.String(),
					SysConfigs: upgradev1.SysConfigs{},
					UpgradeConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
								},
							},
						},
					},
				},
				Status: upgradev1.OSInstanceStatus{},
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

			// stub r.Connection.ConfigureSpec()
			patchConfigure := gomonkey.ApplyMethod(reflect.TypeOf(reconciler.Connection),
				"ConfigureSpec", func(_ *agentclient.Client, _ *agentclient.ConfigsInfo) error {
					return nil
				})
			defer patchConfigure.Reset()
			patchUpdate := gomonkey.ApplyMethod(reflect.TypeOf(reconciler.Connection),
				"UpdateSpec", func(_ *agentclient.Client, _ string, _ *agentclient.DownloadInfo) error {
					return nil
				})
			defer patchUpdate.Reset()

			By("Creating a OS custom resource")
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
					SysConfigs:     upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
					UpgradeConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
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
			Expect(createdOS.Spec.OSVersion).Should(Equal("KubeOS v2"))

			By("Checking the OSInstance status config version")
			time.Sleep(2 * time.Second) // sleep a while to make sure Reconcile finished
			osInsCRLookupKey = types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Status.UpgradeConfigs.Version).Should(Equal("v2"))
		})
	})

	Context("When the controller finds that there is no OSInstance", func() {
		It("Should create the corresponding OSInstance", func() {
			ctx := context.Background()

			By("Creating a worker node")
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
			reconciler.hostName = node1Name

			By("Creating a OS custom resource")
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
						Version: "v1",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
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

			By("Checking the existence of new OSInstance")
			time.Sleep(2 * time.Second) // sleep a while to make sure Reconcile finished
			osInsCRLookupKey := types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Spec.NodeStatus).Should(Equal(values.NodeStatusIdle.String()))
			hostname, ok := createdOSIns.ObjectMeta.Labels[values.LabelOSinstance]
			Expect(ok).Should(BeTrue())
			Expect(hostname).Should(Equal(node1Name))
		})
	})

	Context("When we change the sysconfig version back to previous one when stuck in errors", func() {
		It("Should be able to rollback to previous config version to jump out of error state", func() {
			ctx := context.Background()

			By("Creating a worker node")
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
			reconciler.hostName = node1Name
			Expect(existingNode.Status.NodeInfo.OSImage).Should(Equal("KubeOS v1"))

			By("Creating the corresponding OSInstance")
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
					NodeStatus: values.NodeStatusConfig.String(),
					SysConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
								},
							},
						},
					},
					UpgradeConfigs: upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
				},
				Status: upgradev1.OSInstanceStatus{},
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
			Expect(createdOSIns.Spec.SysConfigs.Version).Should(Equal("v2"))
			createdOSIns.Status.SysConfigs.Version = "v1"
			Expect(k8sClient.Status().Update(ctx, createdOSIns)).Should(Succeed())
			Expect(createdOSIns.Status.SysConfigs.Version).Should(Equal("v1"))

			// stub r.Connection.ConfigureSpec()
			patchConfigure := gomonkey.ApplyMethod(reflect.TypeOf(reconciler.Connection),
				"ConfigureSpec", func(_ *agentclient.Client, _ *agentclient.ConfigsInfo) error {
					return fmt.Errorf("configure error")
				})
			defer patchConfigure.Reset()

			By("Creating a OS custom resource")
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
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
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

			By("Checking the OSInstance status config version failed to be updated")
			time.Sleep(2 * time.Second) // sleep a while to make sure Reconcile finished
			osInsCRLookupKey = types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Status.SysConfigs.Version).Should(Equal("v1"))
			Expect(createdOSIns.Spec.SysConfigs.Version).Should(Equal("v2"))

			By("Changing the OS Spec config version to previous one")
			OS.Spec.SysConfigs = upgradev1.SysConfigs{Version: "v1", Configs: []upgradev1.SysConfig{}}
			Expect(k8sClient.Update(ctx, OS)).Should(Succeed())
			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			// NodeStatus changes to idle then operator can reassign configs to this node
			Expect(createdOSIns.Spec.NodeStatus).Should(Equal(values.NodeStatusIdle.String()))
		})
	})

	Context("When we change the upgradeconfig version back to previous one when stuck in errors", func() {
		It("Should be able to rollback to previous config version to jump out of error state", func() {
			ctx := context.Background()

			By("Creating a worker node")
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
						values.LabelUpgrading:   "",
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
			reconciler.hostName = node1Name

			By("Creating the corresponding OSInstance")
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
					NodeStatus: values.NodeStatusUpgrade.String(),
					SysConfigs: upgradev1.SysConfigs{},
					UpgradeConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
								},
							},
						},
					},
				},
				Status: upgradev1.OSInstanceStatus{},
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
			createdOSIns.Status.UpgradeConfigs.Version = "v1"
			Expect(k8sClient.Status().Update(ctx, createdOSIns)).Should(Succeed())
			Expect(createdOSIns.Status.UpgradeConfigs.Version).Should(Equal("v1"))

			// stub r.Connection.ConfigureSpec()
			patchConfigure := gomonkey.ApplyMethod(reflect.TypeOf(reconciler.Connection),
				"ConfigureSpec", func(_ *agentclient.Client, _ *agentclient.ConfigsInfo) error {
					return fmt.Errorf("configure error")
				})
			defer patchConfigure.Reset()
			patchUpdate := gomonkey.ApplyMethod(reflect.TypeOf(reconciler.Connection),
				"UpdateSpec", func(_ *agentclient.Client, _ string, _ *agentclient.DownloadInfo) error {
					return nil
				})
			defer patchUpdate.Reset()

			By("Creating a OS custom resource")
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
					SysConfigs:     upgradev1.SysConfigs{Configs: []upgradev1.SysConfig{}},
					UpgradeConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
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
			Expect(createdOS.Spec.OSVersion).Should(Equal("KubeOS v2"))

			By("Checking the OSInstance status config version failed to be updated")
			time.Sleep(2 * time.Second) // sleep a while to make sure Reconcile finished
			osInsCRLookupKey = types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Status.UpgradeConfigs.Version).Should(Equal("v1"))
			Expect(createdOSIns.Spec.UpgradeConfigs.Version).Should(Equal("v2"))

			By("Changing the OS Spec config version to previous one")
			OS.Spec.UpgradeConfigs = upgradev1.SysConfigs{Version: "v1", Configs: []upgradev1.SysConfig{}}
			Expect(k8sClient.Update(ctx, OS)).Should(Succeed())
			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			// NodeStatus changes to idle then operator can reassign configs to this node
			Expect(createdOSIns.Spec.NodeStatus).Should(Equal(values.NodeStatusIdle.String()))
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok := existingNode.Labels[values.LabelUpgrading]
			Expect(ok).Should(Equal(false))
		})
	})

	Context("When we complete upgradeconfig, but sysconfig raises error", func() {
		It("Should be able to rollback to previous config version to jump out of error state", func() {
			ctx := context.Background()

			By("Creating a worker node")
			node1Name = "test-node-" + uuid.New().String()
			node1 := &v1.Node{
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
					Labels: map[string]string{
						"beta.kubernetes.io/os": "linux",
						values.LabelUpgrading:   "",
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
			reconciler.hostName = node1Name

			By("Creating the corresponding OSInstance")
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
					NodeStatus: values.NodeStatusUpgrade.String(),
					SysConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "c"},
									{Key: "key2", Value: "d"},
								},
							},
						},
					},
					UpgradeConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
								},
							},
						},
					},
				},
				Status: upgradev1.OSInstanceStatus{},
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
			createdOSIns.Status.UpgradeConfigs = upgradev1.SysConfigs{
				Version: "v2",
				Configs: []upgradev1.SysConfig{
					{
						Model: "kernel.sysctl",
						Contents: []upgradev1.Content{
							{Key: "key1", Value: "a"},
							{Key: "key2", Value: "b"},
						},
					},
				},
			}
			createdOSIns.Status.SysConfigs.Version = "v1"
			Expect(k8sClient.Status().Update(ctx, createdOSIns)).Should(Succeed())
			Expect(createdOSIns.Status.UpgradeConfigs.Version).Should(Equal("v2"))

			// stub r.Connection.ConfigureSpec()
			patchConfigure := gomonkey.ApplyMethod(reflect.TypeOf(reconciler.Connection),
				"ConfigureSpec", func(_ *agentclient.Client, _ *agentclient.ConfigsInfo) error {
					return fmt.Errorf("configure error")
				})
			defer patchConfigure.Reset()

			By("Creating a OS custom resource")
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
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "c"},
									{Key: "key2", Value: "d"},
								},
							},
						},
					},
					UpgradeConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{
								Model: "kernel.sysctl",
								Contents: []upgradev1.Content{
									{Key: "key1", Value: "a"},
									{Key: "key2", Value: "b"},
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
			Expect(createdOS.Spec.OSVersion).Should(Equal("KubeOS v2"))

			By("Checking the OSInstance status config version failed to be updated")
			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			osInsCRLookupKey = types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Status.SysConfigs.Version).Should(Equal("v1"))
			Expect(createdOSIns.Spec.SysConfigs.Version).Should(Equal("v2"))

			By("Changing the OS Spec config version to previous one")
			OS.Spec.SysConfigs = upgradev1.SysConfigs{Version: "v1", Configs: []upgradev1.SysConfig{}}
			Expect(k8sClient.Update(ctx, OS)).Should(Succeed())
			time.Sleep(2 * time.Second) // sleep a while to make sure Reconcile finished
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			// NodeStatus changes to idle then operator can reassign configs to this node
			Expect(createdOSIns.Spec.NodeStatus).Should(Equal(values.NodeStatusIdle.String()))
			Expect(createdOSIns.Spec.SysConfigs.Version).Should(Equal("v1"))
			existingNode = &v1.Node{}
			Eventually(func() bool {
				err := k8sClient.Get(context.Background(),
					types.NamespacedName{Name: node1Name, Namespace: testNamespace}, existingNode)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			_, ok := existingNode.Labels[values.LabelUpgrading]
			Expect(ok).Should(Equal(false))
		})
	})
})
