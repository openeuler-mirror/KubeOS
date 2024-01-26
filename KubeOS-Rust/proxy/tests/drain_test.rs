mod common;

use common::*;
use drain::drain_os;
use k8s_openapi::api::core::v1::{Node, Pod};
use kube::Api;

#[tokio::test]
#[ignore = "integration test"]
async fn test_drain() {
    let client = setup().await.unwrap();
    // drain node
    let nodes: Api<Node> = Api::all(client.clone());
    let node_name = "kubeos-test-worker";
    println!("cordon node");
    nodes.cordon(node_name).await.unwrap();
    println!("drain node");
    drain_os(&client, node_name, true).await.unwrap();

    // assert unschedulable
    println!("check node unschedulable");
    let node = nodes.get(node_name).await.unwrap();
    if let Some(spec) = node.spec {
        assert_eq!(spec.unschedulable, Some(true));
    } else {
        panic!("node spec is none");
    }
    // list all pods on kubeos-test-worker node and all pods should belong to daemonset
    println!("list all pods on kubeos-test-worker node");
    let pods: Api<Pod> = Api::all(client.clone());
    let pod_list = pods.list(&Default::default()).await.unwrap();
    // check the pod is from daemonset
    for p in pod_list {
        if p.spec.unwrap().node_name.unwrap() == node_name {
            assert_eq!(p.metadata.owner_references.unwrap()[0].kind, "DaemonSet");
        }
    }
    nodes.uncordon(node_name).await.unwrap();

    clean_env()
}
