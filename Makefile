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
export GO111MODULE=on
export GO_BUILD = $(GO) build -mod=vendor
else
export GO_BUILD=$(GO) build
endif

VERSION_FILE := ./VERSION
VERSION := $(shell cat $(VERSION_FILE))
PACKAGE:=openeuler.org/KubeOS/pkg/version

EXTRALDFLAGS := -linkmode=external -extldflags=-ftrapv \
	-extldflags=-Wl,-z,relro,-z,now

LD_FLAGS := -ldflags '-buildid=IdByKubeOS \
	-X ${PACKAGE}.Version=${VERSION} \
	$(EXTRALDFLAGS)   '

GO_BUILD_CGO = CGO_ENABLED=1 \
	CGO_CFLAGS="-fstack-protector-strong -fPIE -fPIC -D_FORTIFY_SOURCE=2 -O2" \
	CGO_LDFLAGS_ALLOW='-Wl,-z,relro,-z,now' \
	CGO_LDFLAGS="-Wl,-z,relro,-z,now -Wl,-z,noexecstack" \
	${GO_BUILD} -buildmode=pie -trimpath -tags "seccomp selinux static_build cgo netgo osusergo"

all: proxy operator agent hostshell

# Build binary
proxy:
	${GO_BUILD_CGO} ${LD_FLAGS} -o bin/os-proxy  cmd/proxy/main.go
	strip bin/os-proxy

operator:
	${GO_BUILD_CGO} ${LD_FLAGS} -o bin/os-operator cmd/operator/main.go
	strip bin/os-operator

agent:
	${GO_BUILD_CGO} ${LD_FLAGS} -o bin/os-agent cmd/agent/main.go
	strip bin/os-agent

hostshell:
	${GO_BUILD_CGO} ${LD_FLAGS} -o bin/hostshell cmd/admin-container/main.go
	strip bin/hostshell

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

## Location to install dependencies to
LOCALBIN = $(shell pwd)/bin
$(LOCALBIN):
	mkdir -p $(LOCALBIN)

## Tool Binaries
CONTROLLER_GEN = $(LOCALBIN)/controller-gen
ENVTEST = $(LOCALBIN)/setup-envtest

## Tool Versionsjk
CONTROLLER_TOOLS_VERSION = v0.5.0
ENVTEST_K8S_VERSION = 1.20.2 ## ENVTEST_K8S_VERSION refers to the version of kubebuilder assets to be downloaded by envtest binary.

controller-gen: $(CONTROLLER_GEN) ## Download controller-gen locally if necessary. If wrong version is installed, it will be overwritten.
$(CONTROLLER_GEN): $(LOCALBIN)
	test -s $(LOCALBIN)/controller-gen && $(LOCALBIN)/controller-gen --version | grep -q $(CONTROLLER_TOOLS_VERSION) || \
	GOBIN=$(LOCALBIN) go install sigs.k8s.io/controller-tools/cmd/controller-gen@$(CONTROLLER_TOOLS_VERSION)

# Download kustomize locally if necessary
KUSTOMIZE = $(LOCALBIN)/kustomize
kustomize:
	$(call go-get-tool,$(KUSTOMIZE),sigs.k8s.io/kustomize/kustomize/v3@v3.8.7)

.PHONY: test
test: manifests fmt vet envtest ## Run tests.
	KUBEBUILDER_ASSETS="$(shell $(ENVTEST) use $(ENVTEST_K8S_VERSION) --bin-dir $(LOCALBIN) -p path)" go test ./... -race -count=1 -timeout=300s -cover -gcflags=all=-l

.PHONY: envtest
envtest: $(ENVTEST) ## Download envtest-setup locally if necessary.
$(ENVTEST): $(LOCALBIN)
	test -s $(LOCALBIN)/setup-envtest || GOBIN=$(LOCALBIN) go install sigs.k8s.io/controller-runtime/tools/setup-envtest@latest

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
