use std::process::{Command, Stdio};

use anyhow::Result;
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::ResourceExt,
    client::Client,
    config::{Config, KubeConfigOptions, Kubeconfig},
    Api,
};
use manager::utils::{CommandExecutor, RealCommandExecutor};

pub const CLUSTER: &str = "kubeos-test";

pub fn run_command(cmd: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(cmd).args(args).stdout(Stdio::inherit()).stderr(Stdio::inherit()).output()?;
    if !output.status.success() {
        println!("failed to run command: {} {}\n", cmd, args.join(" "));
    }
    Ok(())
}

pub async fn setup() -> Result<Client> {
    // set PATH variable
    let path = std::env::var("PATH").unwrap();
    let new_path = format!("{}:{}", path, "../../bin");
    std::env::set_var("PATH", new_path);

    // create cluster
    let executor = RealCommandExecutor {};
    println!("Creating cluster");
    run_command("bash", &["./tests/setup/setup_test_env.sh"]).expect("failed to create cluster");

    // connect to the cluster
    let kind_config = executor.run_command_with_output("kind", &["get", "kubeconfig", "-n", CLUSTER]).unwrap();
    let kubeconfig = Kubeconfig::from_yaml(kind_config.as_str()).expect("failed to parse kubeconfig");
    let options = KubeConfigOptions::default();
    let config = Config::from_custom_kubeconfig(kubeconfig, &&options).await.expect("failed to create config");
    let client = Client::try_from(config).expect("failed to create client");
    // list all nodes
    let nodes: Api<Node> = Api::all(client.clone());
    let node_list = nodes.list(&Default::default()).await.expect("failed to list nodes");
    for n in node_list {
        println!("Found Node: {}", n.name());
    }
    // check node status
    let node = nodes.get("kubeos-test-worker").await.unwrap();
    let status = node.status.unwrap();
    let conditions = status.conditions.unwrap();
    for c in conditions {
        if c.type_ == "Ready" {
            assert_eq!(c.status, "True");
        }
    }
    println!("Cluster ready");
    Ok(client)
}

pub fn clean_env() {
    let executor = RealCommandExecutor {};
    println!("Cleaning cluster");
    executor.run_command("kind", &["delete", "clusters", CLUSTER]).expect("failed to clean cluster");
}
