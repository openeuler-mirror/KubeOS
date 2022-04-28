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
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/rest"
	controllerruntime "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/manager"

	"openeuler.org/KubeOS/api/v1alpha1"
	"openeuler.org/KubeOS/pkg/common"
)

var config *rest.Config
var fakeClient client.Client

type fakeReconciler struct {
	needGetErr    bool
	needListErr   bool
	needUpdateErr bool
}

// Get is implement of client.Reader.Get
func (f fakeReconciler) Get(ctx context.Context, key client.ObjectKey, obj client.Object) error {
	if f.needGetErr {
		return fmt.Errorf("Not Get")
	}
	return nil
}

// List is implement of client.Reader.List
func (f fakeReconciler) List(_ context.Context, list client.ObjectList, _ ...client.ListOption) error {
	if f.needListErr {
		return fmt.Errorf("Not List")
	}
	nodeList := corev1.NodeList{
		Items: []corev1.Node{{ObjectMeta: metav1.ObjectMeta{Name: "aa", Labels: map[string]string{
			"saiyan.openeuler.org/images": "aaa",
		}}}},
	}
	reflect.Indirect(reflect.ValueOf(list)).Set(reflect.ValueOf(nodeList))
	return nil
}

// Update is implement of client.Writer.Update
func (f fakeReconciler) Update(ctx context.Context, obj client.Object, opts ...client.UpdateOption) error {
	if f.needUpdateErr {
		return fmt.Errorf("Not update")
	}
	return nil
}

// Patch is implement of client.Writer.Patch
func (fakeReconciler) Patch(ctx context.Context, obj client.Object, patch client.Patch,
	opts ...client.PatchOption) error {
	return nil
}

// test getAndUpdateOS
func Test_getAndUpdateOS(t *testing.T) {
	type args struct {
		ctx  context.Context
		r    common.ReadStatusWriter
		name types.NamespacedName
	}
	tests := []struct {
		name        string
		args        args
		wantOs      v1alpha1.OS
		wantNodeNum int
		wantErr     bool
	}{
		{name: "normal", args: args{ctx: context.Background(), r: fakeReconciler{},
			name: client.ObjectKey{Name: "aa"}}, wantOs: v1alpha1.OS{TypeMeta: metav1.TypeMeta{},
			ObjectMeta: metav1.ObjectMeta{}, Spec: v1alpha1.OSSpec{}}, wantNodeNum: 1, wantErr: false},
		{name: "getError", args: args{ctx: context.Background(), r: fakeReconciler{needGetErr: true,
			needListErr: false, needUpdateErr: false}, name: client.ObjectKey{Name: "aa"}},
			wantOs: v1alpha1.OS{TypeMeta: metav1.TypeMeta{}, ObjectMeta: metav1.ObjectMeta{},
				Spec: v1alpha1.OSSpec{}}, wantNodeNum: 0, wantErr: true},
		{name: "listError", args: args{ctx: context.Background(), r: fakeReconciler{needGetErr: false,
			needListErr: true, needUpdateErr: false}, name: client.ObjectKey{Name: "aa"}},
			wantOs: v1alpha1.OS{TypeMeta: metav1.TypeMeta{}, ObjectMeta: metav1.ObjectMeta{},
				Spec: v1alpha1.OSSpec{}}, wantNodeNum: 0, wantErr: true},
		{name: "updateError", args: args{ctx: context.Background(), r: fakeReconciler{needGetErr: false,
			needListErr: false, needUpdateErr: true}, name: client.ObjectKey{Name: "aa"}},
			wantOs: v1alpha1.OS{TypeMeta: metav1.TypeMeta{}, ObjectMeta: metav1.ObjectMeta{},
				Spec: v1alpha1.OSSpec{}}, wantNodeNum: 1, wantErr: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			gotOs, gotNodeNum, err := getAndUpdateOS(tt.args.ctx, tt.args.r, tt.args.name)
			if (err != nil) != tt.wantErr {
				t.Errorf("getAndUpdateOS() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(gotOs, tt.wantOs) {
				t.Errorf("getAndUpdateOS() gotOs = %v, want %v", gotOs, tt.wantOs)
			}
			if gotNodeNum != tt.wantNodeNum {
				t.Errorf("getAndUpdateOS() gotNodeNum = %v, want %v", gotNodeNum, tt.wantNodeNum)
			}
		})
	}
}

// test assignUpgrade
func Test_assignUpgrade(t *testing.T) {
	type args struct {
		ctx       context.Context
		r         common.ReadStatusWriter
		osVersion string
		limit     int
	}
	tests := []struct {
		name    string
		args    args
		want    bool
		wantErr bool
	}{
		{name: "normal", args: args{ctx: context.Background(), r: fakeReconciler{}, osVersion: "openEuler21.03",
			limit: 3}, want: false, wantErr: false},
		{name: "overLimit", args: args{ctx: context.Background(), r: fakeReconciler{}, osVersion: "openEuler21.03",
			limit: 0}, want: true, wantErr: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := assignUpgrade(tt.args.ctx, tt.args.r, tt.args.osVersion, tt.args.limit)
			if (err != nil) != tt.wantErr {
				t.Errorf("assignUpgrade() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("assignUpgrade() got = %v, want %v", got, tt.want)
			}
		})
	}
}

// test getNodes
func Test_getNodes(t *testing.T) {
	type args struct {
		limit int
		reqs  []labels.Requirement
	}
	tests := []struct {
		name    string
		args    args
		want    []corev1.Node
		wantErr bool
	}{
		{name: "normal", args: args{}, want: []corev1.Node{{ObjectMeta: metav1.ObjectMeta{Name: "aa",
			Labels: map[string]string{"saiyan.openeuler.org/images": "aaa"}}}}, wantErr: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := getNodes(context.Background(), fakeReconciler{}, tt.args.limit, tt.args.reqs...)
			if (err != nil) != tt.wantErr {
				t.Errorf("getNodes() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("getNodes() got = %v, want %v", got, tt.want)
			}
		})
	}
}

// test checkUpgrading
func Test_checkUpgrading(t *testing.T) {
	type args struct {
		ctx            context.Context
		r              common.ReadStatusWriter
		maxUnavailable int
	}
	tests := []struct {
		name    string
		args    args
		want    int
		wantErr bool
	}{
		{name: "normal", args: args{ctx: context.Background(), r: fakeReconciler{}, maxUnavailable: 3},
			want: 2, wantErr: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := checkUpgrading(tt.args.ctx, tt.args.r, tt.args.maxUnavailable)
			if (err != nil) != tt.wantErr {
				t.Errorf("checkUpgrading() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if got != tt.want {
				t.Errorf("checkUpgrading() got = %v, want %v", got, tt.want)
			}
		})
	}
}

// test SetupWithManager
func TestOSReconciler_SetupWithManager(t *testing.T) {
	type fields struct {
		Scheme *runtime.Scheme
		Client client.Client
	}
	fakeClient, _ = client.New(config, client.Options{Scheme: runtime.NewScheme()})
	var mManager manager.Manager
	mManager, _ = controllerruntime.NewManager(config, manager.Options{Scheme: runtime.NewScheme()})
	type args struct {
		mgr controllerruntime.Manager
	}
	tests := []struct {
		name    string
		fields  fields
		args    args
		wantErr bool
	}{
		{name: "normal", fields: fields{Scheme: runtime.NewScheme(), Client: fakeClient}, args: args{mManager},
			wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := &OSReconciler{
				Client: tt.fields.Client,
				Scheme: tt.fields.Scheme,
			}
			if err := r.SetupWithManager(tt.args.mgr); (err != nil) != tt.wantErr {
				t.Errorf("SetupWithManager() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

// test Reconcile
func TestReconcile(t *testing.T) {
	type args struct {
		ctx context.Context
		r   common.ReadStatusWriter
		req controllerruntime.Request
	}
	tests := []struct {
		name    string
		args    args
		want    controllerruntime.Result
		wantErr bool
	}{
		{name: "normal", args: args{ctx: context.Background(), r: fakeReconciler{}, req: controllerruntime.Request{}},
			want: controllerruntime.Result{Requeue: true, RequeueAfter: 15000000000}, wantErr: false},
		{name: "getErr", args: args{ctx: context.Background(), r: fakeReconciler{needGetErr: true, needListErr: false,
			needUpdateErr: false}, req: controllerruntime.Request{}},
			want: controllerruntime.Result{Requeue: true, RequeueAfter: 0}, wantErr: true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := Reconcile(tt.args.ctx, tt.args.r, tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("Reconcile() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("Reconcile() got = %v, want %v", got, tt.want)
			}
		})
	}
}

// test Min
func TestMin(t *testing.T) {
	little, big := 3, 4
	type args struct {
		a int
		b int
	}
	tests := []struct {
		name string
		args args
		want int
	}{
		{name: "normal", args: args{a: little, b: big}, want: little},
		{name: "equal", args: args{a: little, b: little}, want: little},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := min(tt.args.a, tt.args.b); got != tt.want {
				t.Errorf("Min() = %v, want %v", got, tt.want)
			}
		})
	}
}

// test Reconcile
func TestOSReconciler_Reconcile(t *testing.T) {
	type fields struct {
		Scheme *runtime.Scheme
		Client client.Client
	}
	type args struct {
		ctx context.Context
		req controllerruntime.Request
	}
	tests := []struct {
		name    string
		fields  fields
		args    args
		want    controllerruntime.Result
		wantErr bool
	}{
		{name: "normal", fields: fields{Scheme: runtime.NewScheme(), Client: fakeClient},
			args: args{ctx: context.Background(), req: controllerruntime.Request{}}, want: controllerruntime.Result{},
			wantErr: false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			r := &OSReconciler{
				Client: tt.fields.Client,
				Scheme: tt.fields.Scheme,
			}
			got, err := r.Reconcile(tt.args.ctx, tt.args.req)
			if (err != nil) != tt.wantErr {
				t.Errorf("Reconcile() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !reflect.DeepEqual(got, tt.want) {
				t.Errorf("Reconcile() got = %v, want %v", got, tt.want)
			}
		})
	}
}
