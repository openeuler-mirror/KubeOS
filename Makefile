## Copyright (c) Huawei Technologies Co., Ltd. 2021. All rights reserved.
 # KubeOS is licensed under the Mulan PSL v2.
 # You can use this software according to the terms and conditions of the Mulan PSL v2.
 # You may obtain a copy of Mulan PSL v2 at:
 #     http://license.coscl.org.cn/MulanPSL2
 # THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 # IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 # PURPOSE.
## See the Mulan PSL v2 for more details.

# Image URL to use all building/pushing image targets
IMG_PROXY ?= proxy:latest
IMG_OPERATOR ?= operator:latest
# Produce CRDs that work back to Kubernetes 1.11 (no version conversion)
CRD_OPTIONS ?= "crd:trivialVersions=true,preserveUnknownFields=false"

# Get the currently used golang install path (in GOPATH/bin, unless GOBIN is set)
ifeq (,$(shell go env GOBIN))
GOBIN=$(shell go env GOPATH)/bin
else
GOBIN=$(shell go env GOBIN)
endif

GO := go
ifeq ($(shell go help mod >/dev/null 2>&1 && echo true), true)
export GO_BUILD=GO111MODULE=on; $(GO) build -mod=vendor
else
export GO_BUILD=$(GO) build
endif

VERSION_FILE := ./VERSION
VERSION := $(shell cat $(VERSION_FILE))
PACKAGE:=openeuler.org/saiyan/pkg/version
BUILDFLAGS = -buildmode=pie -trimpath
LDFLAGS = -w -s -buildid=IdByKubeOS -linkmode=external -extldflags=-static -extldflags=-zrelro -extldflags=-Wl,-z,now -X ${PACKAGE}.Version=${VERSION}
ENV = CGO_CFLAGS="-fstack-protector-all" CGO_CPPFLAGS="-D_FORTIFY_SOURCE=2 -O2"

all: proxy operator agent

# Build binary
proxy:
	${ENV} ${GO_BUILD} -ldflags '$(LDFLAGS)' $(BUILDFLAGS) -o bin/proxy cmd/proxy/main.go
	strip bin/proxy

operator:
	${ENV} ${GO_BUILD} -ldflags '$(LDFLAGS)' $(BUILDFLAGS) -o bin/operator cmd/operator/main.go
	strip bin/operator

agent:
	${ENV} ${GO_BUILD} -tags "osusergo netgo static_build" -ldflags '$(LDFLAGS)' $(BUILDFLAGS) -o bin/os-agent cmd/agent/main.go
	strip bin/os-agent

test:
	$(GO) test $(shell go list ./... ) -race -cover -count=1 -timeout=300s
	
# Install CRDs into a cluster
install: manifests
	kubectl apply -f confg/crd

# Uninstall CRDs from a cluster
uninstall: manifests
	kubectl delete -f config/crd

# Deploy controller in the configured Kubernetes cluster in ~/.kube/config
deploy: manifests
	kubectl apply -f config/rbac
	kubectl apply -f config/manager

# UnDeploy controller from the configured Kubernetes cluster in ~/.kube/config
undeploy:
	kubectl delete -f config/rbac
	kubectl delete -f config/manager

# Generate manifests e.g. CRD, RBAC etc.
manifests: controller-gen
	$(CONTROLLER_GEN) $(CRD_OPTIONS) rbac:roleName=upgrade-manager-role paths="./..." output:crd:artifacts:config=config/crd

# Run go fmt against code
fmt:
	go fmt ./...

# Run go vet against code
vet:
	go vet ./...

# Generate code
generate: controller-gen
	$(CONTROLLER_GEN) object:headerFile="hack/boilerplate.go.txt" paths="./..."

# Build the docker image
docker-build: operator proxy
	docker build --target operator -t ${IMG_OPERATOR} .
	docker build --target proxy -t ${IMG_PROXY} .

# Push the docker image
docker-push:
	docker push ${IMG_OPERATOR}
	docker push ${IMG_PROXY}

# Download controller-gen locally if necessary
CONTROLLER_GEN = $(shell pwd)/bin/controller-gen
controller-gen:
	$(call go-get-tool,$(CONTROLLER_GEN),sigs.k8s.io/controller-tools/cmd/controller-gen@v0.5.0)

# Download kustomize locally if necessary
KUSTOMIZE = $(shell pwd)/bin/kustomize
kustomize:
	$(call go-get-tool,$(KUSTOMIZE),sigs.k8s.io/kustomize/kustomize/v3@v3.8.7)

# go-get-tool will 'go get' any package $2 and install it to $1.
PROJECT_DIR := $(shell dirname $(abspath $(lastword $(MAKEFILE_LIST))))
define go-get-tool
@[ -f $(1) ] || { \
set -e ;\
TMP_DIR=$$(mktemp -d) ;\
cd $$TMP_DIR ;\
go mod init tmp ;\
echo "Downloading $(2)" ;\
GOBIN=$(PROJECT_DIR)/bin go get $(2) ;\
rm -rf $$TMP_DIR ;\
}
endef
