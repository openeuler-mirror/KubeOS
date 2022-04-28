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
	"testing"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"

	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/kubernetes"
	"k8s.io/client-go/rest"
	"k8s.io/kubectl/pkg/drain"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/manager"

	"openeuler.org/KubeOS/api/v1alpha1"
	"openeuler.org/KubeOS/pkg/agentclient"
	ptypes "openeuler.org/KubeOS/pkg/common"
)

var fakeClient client.Client
var config *rest.Config = new(rest.Config)

type fakeReconciler struct{}

// Get is implement of client.Reader.Get
func (fakeReconciler) Get(ctx context.Context, key client.ObjectKey, obj client.Object) error {
	if key.Name == "bb" {
		return fmt.Errorf("error Get")
	}
	return nil
}

// List is implement of client.Reader.List
func (fakeReconciler) List(_ context.Context, list client.ObjectList, _ ...client.ListOption) error {
	return nil
}

// Update is implement of client.Writer.Update
func (fakeReconciler) Update(ctx context.Context, obj client.Object, opts ...client.UpdateOption) error {
	return nil
}

// Patch is implement of client.Writer.Patch
func (fakeReconciler) Patch(ctx context.Context, obj client.Object, patch client.Patch,
	opts ...client.PatchOption) error {
	return nil
}

// test NewOSReconciler
//func TestNewOSReconciler(t *testing.T) {
//	var mManager manager.Manager
//	mManager, _ = ctrl.NewManager(config, manager.Options{Scheme: runtime.NewScheme()})
//	type args struct {
//		mgr manager.Manager
//	}
//	tests := []struct {
//		name string
//		args args
//	}{
//		{name: "normal", args: args{mgr: mManager}},
//	}
//	for _, tt := range tests {
//		t.Run(tt.name, func(t *testing.T) {
//			if got := NewOSReconciler(tt.args.mgr); !reflect.DeepEqual(got, nil) {
//			}
//		})
//	}
//}

// test getOSAndNodeStatus
func Test_getOSAndNodeStatus(t *testing.T) {
	type args struct {
		ctx      context.Context
		r        ptypes.ReadStatusWriter
		name     types.NamespacedName
		hostName string
	}
	tests := []struct {
		name     string
		args     args
		wantOS   v1alpha1.OS
		wantNode corev1.Node
	}{
		{name: "normal", args: args{ctx: context.Background(), r: fakeReconciler{},
			name: client.ObjectKey{Name: "aa"}, hostName: "aa"}, wantOS: v1alpha1.OS{TypeMeta: metav1.TypeMeta{},
			ObjectMeta: metav1.ObjectMeta{}, Spec: v1alpha1.OSSpec{},
		}, wantNode: corev1.Node{TypeMeta: metav1.TypeMeta{}, ObjectMeta: metav1.ObjectMeta{},
			Spec: corev1.NodeSpec{}, Status: corev1.NodeStatus{},
		}},
		{name: "error", args: args{ctx: context.Background(), r: fakeReconciler{},
			name: client.ObjectKey{Name: "bb"}, hostName: "cc"}, wantOS: v1alpha1.OS{TypeMeta: metav1.TypeMeta{},
			ObjectMeta: metav1.ObjectMeta{}, Spec: v1alpha1.OSSpec{},
		}, wantNode: corev1.Node{TypeMeta: metav1.TypeMeta{}, ObjectMeta: metav1.ObjectMeta{},
			Spec: corev1.NodeSpec{}, Status: corev1.NodeStatus{},
		}},
		{name: "error", args: args{ctx: context.Background(), r: fakeReconciler{},
			name: client.ObjectKey{Name: "cc"}, hostName: "bb"}, wantOS: v1alpha1.OS{TypeMeta: metav1.TypeMeta{},
			ObjectMeta: metav1.ObjectMeta{}, Spec: v1alpha1.OSSpec{},
		}, wantNode: corev1.Node{TypeMeta: metav1.TypeMeta{}, ObjectMeta: metav1.ObjectMeta{},
			Spec: corev1.NodeSpec{}, Status: corev1.NodeStatus{},
		}},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			gotOS, gotNode := getOSAndNodeStatus(tt.args.ctx, tt.args.r, tt.args.name, tt.args.hostName)
			if !reflect.DeepEqual(gotOS, tt.wantOS) {
				t.Errorf("getOSAndNodeStatus() gotOS = %v, want %v", gotOS, tt.wantOS)
			}
			if !reflect.DeepEqual(gotNode, tt.wantNode) {
				t.Errorf("getOSAndNodeStatus() gotNode = %v, want %v", gotNode, tt.wantNode)
			}
		})
	}
}

// test SetupWithManager
func TestOSReconciler_SetupWithManager(t *testing.T) {
	type fields struct {
		Scheme        *runtime.Scheme
		Connection    *agentclient.Client
		Client        client.Client
		kubeclientset kubernetes.Interface
		hostName      string
	}
	fakeClient, _ = client.New(config, client.Options{Scheme: runtime.NewScheme()})
	var mManager manager.Manager
	mManager, _ = ctrl.NewManager(config, manager.Options{Scheme: runtime.NewScheme()})
	client, _ := agentclient.New("aa")
	type args struct {
		mgr ctrl.Manager
	}
	tests := []struct {
		name    string
		fields  fields
		args    args
		wantErr bool
	}{
		{name: "normal", fields: fields{Scheme: runtime.NewScheme(), Connection: client, Client: fakeClient,
			kubeclientset: kubernetes.Interface(nil), hostName: "aa"}, args: args{mgr: mManager}, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := &OSReconciler{
				Client:        tt.fields.Client,
				Scheme:        tt.fields.Scheme,
				Connection:    tt.fields.Connection,
				kubeclientset: tt.fields.kubeclientset,
				hostName:      tt.fields.hostName,
			}
			if err := r.SetupWithManager(tt.args.mgr); (err != nil) != tt.wantErr {
				t.Errorf("SetupWithManager() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

// test evictNode
func Test_evictNode(t *testing.T) {
	type args struct {
		drainer *drain.Helper
		node    *corev1.Node
	}
	tests := []struct {
		name    string
		args    args
		wantErr bool
	}{
		{name: "normal", args: args{drainer: &drain.Helper{}, node: &corev1.Node{
			TypeMeta:   metav1.TypeMeta{},
			ObjectMeta: metav1.ObjectMeta{},
			Spec:       corev1.NodeSpec{Unschedulable: true},
			Status:     corev1.NodeStatus{},
		}}, wantErr: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if err := evictNode(tt.args.drainer, tt.args.node); (err != nil) != tt.wantErr {
				t.Errorf("evictNode() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}
