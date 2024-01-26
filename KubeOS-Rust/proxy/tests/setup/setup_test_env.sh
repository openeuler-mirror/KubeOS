#!/bin/bash
# this bash script executes in proxy directory

set -Eeuxo pipefail

# Define variables
KIND_VERSION="v0.19.0"
KUBECTL_VERSION="v1.24.15"
KIND_CLUSTER_NAME="kubeos-test"
DOCKER_IMAGES=("busybox:stable" "nginx:alpine" "kindest/node:v1.24.15@sha256:7db4f8bea3e14b82d12e044e25e34bd53754b7f2b0e9d56df21774e6f66a70ab")
NODE_IMAGE="kindest/node:v1.24.15@sha256:7db4f8bea3e14b82d12e044e25e34bd53754b7f2b0e9d56df21774e6f66a70ab"
RESOURCE="./tests/setup/resources.yaml"
KIND_CONFIG="./tests/setup/kind-config.yaml"
BIN_PATH="../../bin/"
ARCH=$(uname -m)

# Install kind and kubectl
install_bins() {
    # if bin dir not exist then create
    if [ ! -d "${BIN_PATH}" ]; then
        mkdir -p "${BIN_PATH}"
    fi
    if [ ! -f "${BIN_PATH}"kind ]; then
        echo "Installing Kind..."
        # For AMD64 / x86_64
        if [ "$ARCH" = x86_64 ]; then
            # add proxy if you are behind proxy
            curl -Lo "${BIN_PATH}"kind https://kind.sigs.k8s.io/dl/"${KIND_VERSION}"/kind-linux-amd64
        fi
        # For ARM64
        if [ "$ARCH" = aarch64 ]; then
            curl -Lo "${BIN_PATH}"kind https://kind.sigs.k8s.io/dl/"${KIND_VERSION}"/kind-linux-arm64
        fi
        chmod +x "${BIN_PATH}"kind
    fi
    if [ ! -f "${BIN_PATH}"kubectl ]; then
        echo "Installing kubectl..."
        if [ "$ARCH" = x86_64 ]; then
            curl -Lo "${BIN_PATH}"kubectl "https://dl.k8s.io/release/${KUBECTL_VERSION}/bin/linux/amd64/kubectl"
        fi
        if [ "$ARCH" = aarch64 ]; then
            curl -Lo "${BIN_PATH}"kubectl "https://dl.k8s.io/release/${KUBECTL_VERSION}/bin/linux/arm64/kubectl"
        fi
        chmod +x "${BIN_PATH}"kubectl
    fi
    export PATH=$PATH:"${BIN_PATH}"
}

# Create Kind Cluster
create_cluster() {
    echo "Creating Kind cluster..."
    for image in "${DOCKER_IMAGES[@]}"; do
        docker pull "$image"
    done
    kind create cluster --name "${KIND_CLUSTER_NAME}" --config "${KIND_CONFIG}" --image "${NODE_IMAGE}"
}

# Load Docker image into Kind cluster
load_docker_image() {
    echo "Loading Docker image into Kind cluster..."
    DOCKER_IMAGE=$(printf "%s " "${DOCKER_IMAGES[@]:0:2}")
    kind load docker-image ${DOCKER_IMAGE} --name "${KIND_CLUSTER_NAME}"
}

# Apply Kubernetes resource files
apply_k8s_resources() {
    echo "Applying Kubernetes resources from ${RESOURCE}..."
    kubectl apply -f "${RESOURCE}"
    echo "Waiting for nodes getting ready..."
    sleep 40s
}

main() {
    export no_proxy=localhost,127.0.0.1
    install_bins
    create_cluster
    load_docker_image
    apply_k8s_resources
}

main
