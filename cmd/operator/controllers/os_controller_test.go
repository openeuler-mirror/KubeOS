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
	"testing"
	"time"

	"github.com/agiledragon/gomonkey/v2"
	"github.com/google/uuid"
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	v1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/errors"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/kubernetes/scheme"
	"k8s.io/client-go/util/workqueue"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/event"

	upgradev1 "openeuler.org/KubeOS/api/v1alpha1"
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

	AfterEach(func() {
		desiredTestNamespace := &v1.Namespace{
			TypeMeta: metav1.TypeMeta{
				APIVersion: "v1",
				Kind:       "Namespace",
			},
			ObjectMeta: metav1.ObjectMeta{
				Name: testNamespace,
			},
		}
		// Add any teardown steps that needs to be executed after each test
		err := k8sClient.Delete(context.Background(), desiredTestNamespace,
			client.PropagationPolicy(metav1.DeletePropagationForeground))

		Expect(err).ToNot(HaveOccurred())

		existingNamespace := &v1.Namespace{}
		Eventually(func() bool {
			err := k8sClient.Get(context.Background(), types.NamespacedName{Name: testNamespace},
				existingNamespace)
			if err != nil && errors.IsNotFound(err) {
				return false
			}
			return true
		}, timeout, interval).Should(BeTrue())
	})

	Context("When we change the OSVersion to previous version and Opstype is rollback", func() {
		It("Should label the osinstance's nodestatus to upgrading", func() {
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

			// create OSInstance
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
		})
	})

	Context("When we want to configure node", func() {
		It("Should update OSInstance spec and update NodeStatus to config", func() {
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

			OSIns := &upgradev1.OSInstance{
				TypeMeta: metav1.TypeMeta{
					Kind:       "OSInstance",
					APIVersion: "upgrade.openeuler.org/v1alpha1",
				},
				ObjectMeta: metav1.ObjectMeta{
					Name:      node1Name,
					Namespace: testNamespace,
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

			osInsCRLookupKey := types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.ObjectMeta.Name).Should(Equal(node1Name))

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

			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			configedOSIns := &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, configedOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(configedOSIns.Spec.NodeStatus).Should(Equal(values.NodeStatusConfig.String()))
			Expect(configedOSIns.Spec.SysConfigs.Version).Should(Equal("v2"))
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
		})
	})

	Context("When we want to upgrade and do sysconfigs which contain grub.cmd.current and .next", func() {
		It("Should exchange .current and .next", func() {
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

			// create OSInstance
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
					OpsType:        "upgrade",
					MaxUnavailable: 3,
					OSVersion:      "KubeOS v2",
					FlagSafe:       true,
					MTLS:           false,
					EvictPodForce:  true,
					SysConfigs: upgradev1.SysConfigs{
						Version: "v2",
						Configs: []upgradev1.SysConfig{
							{Model: "grub.cmdline.current", Contents: []upgradev1.Content{{Key: "a", Value: "1"}}},
							{Model: "grub.cmdline.next", Contents: []upgradev1.Content{{Key: "b", Value: "2"}}},
						},
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
			Expect(createdOS.Spec.OSVersion).Should(Equal("KubeOS v2"))

			time.Sleep(1 * time.Second) // sleep a while to make sure Reconcile finished
			osInsCRLookupKey = types.NamespacedName{Name: node1Name, Namespace: testNamespace}
			createdOSIns = &upgradev1.OSInstance{}
			Eventually(func() bool {
				err := k8sClient.Get(ctx, osInsCRLookupKey, createdOSIns)
				return err == nil
			}, timeout, interval).Should(BeTrue())
			Expect(createdOSIns.Spec.SysConfigs.Configs[0]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.next", Contents: []upgradev1.Content{{Key: "a", Value: "1"}}}))
			Expect(createdOSIns.Spec.SysConfigs.Configs[1]).Should(Equal(upgradev1.SysConfig{Model: "grub.cmdline.current", Contents: []upgradev1.Content{{Key: "b", Value: "2"}}}))
		})
	})
})

func TestOSReconciler_DeleteOSInstance(t *testing.T) {
	type fields struct {
		Scheme *runtime.Scheme
		Client client.Client
	}
	kClient, _ := client.New(cfg, client.Options{Scheme: scheme.Scheme})
	type args struct {
		e event.DeleteEvent
		q workqueue.RateLimitingInterface
	}
	tests := []struct {
		name   string
		fields fields
		args   args
	}{
		{
			name: "delete osinstance",
			fields: fields{
				Scheme: nil,
				Client: kClient,
			},
			args: args{
				e: event.DeleteEvent{
					Object: &upgradev1.OSInstance{
						ObjectMeta: metav1.ObjectMeta{
							Name:      "test-node1",
							Namespace: "test",
						},
					},
				},
				q: nil,
			},
		},
	}
	var patchList *gomonkey.Patches
	var patchDelete *gomonkey.Patches
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := &OSReconciler{
				Scheme: tt.fields.Scheme,
				Client: tt.fields.Client,
			}
			patchList = gomonkey.ApplyMethodFunc(r.Client, "List", func(ctx context.Context, list client.ObjectList, opts ...client.ListOption) error {
				list.(*upgradev1.OSInstanceList).Items = []upgradev1.OSInstance{
					{
						ObjectMeta: metav1.ObjectMeta{
							Name:      "test-node1",
							Namespace: "test",
						},
					},
				}
				return nil
			})
			patchDelete = gomonkey.ApplyMethodReturn(r.Client, "Delete", nil)
			r.DeleteOSInstance(tt.args.e, tt.args.q)
		})
	}
	defer patchDelete.Reset()
	defer patchList.Reset()
}
