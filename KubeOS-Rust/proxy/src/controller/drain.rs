/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2023. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use futures::{stream, StreamExt};
use k8s_openapi::api::core::v1::{Pod, PodSpec, PodStatus};
use kube::{
    api::{EvictParams, ListParams},
    core::ObjectList,
    Api, Client, ResourceExt,
};
use log::{debug, error, info};
use reqwest::StatusCode;
use tokio::time::{sleep, Duration, Instant};
use tokio_retry::{
    strategy::{jitter, ExponentialBackoff},
    RetryIf,
};

use self::error::{
    DrainError::{DeletePodsError, GetPodListsError, WaitDeletionError},
    EvictionError::{EvictionErrorNoRetry, EvictionErrorRetry},
};
use super::values::{
    EVERY_DELETION_CHECK, EVERY_EVICTION_RETRY, MAX_EVICT_POD_NUM, MAX_RETRIES_TIMES, RETRY_BASE_DELAY,
    RETRY_MAX_DELAY, TIMEOUT,
};

pub async fn drain_os(client: &Client, node_name: &str, force: bool) -> Result<(), error::DrainError> {
    let pods_list = get_pods_deleted(client, node_name, force).await?;

    stream::iter(pods_list)
        .for_each_concurrent(MAX_EVICT_POD_NUM, move |pod| {
            let k8s_client = client.clone();
            async move {
                if evict_pod(&k8s_client, &pod, force).await.is_ok() {
                    wait_for_deletion(&k8s_client, &pod).await.ok();
                }
            }
        })
        .await;

    Ok(())
}

async fn get_pods_deleted(
    client: &Client,
    node_name: &str,
    force: bool,
) -> Result<impl Iterator<Item = Pod>, error::DrainError> {
    let lp = ListParams { field_selector: Some(format!("spec.nodeName={}", node_name)), ..Default::default() };
    let pods_api: Api<Pod> = Api::all(client.clone());
    let pods: ObjectList<Pod> = match pods_api.list(&lp).await {
        Ok(pods @ ObjectList { .. }) => pods,
        Err(err) => {
            return Err(GetPodListsError { source: err, node_name: node_name.to_string() });
        },
    };
    let mut filterd_pods_list: Vec<Pod> = Vec::new();
    let mut filterd_err: Vec<String> = Vec::new();
    let pod_filter = CombinedFilter::new(force);
    for pod in pods.into_iter() {
        let filter_result = pod_filter.filter(&pod);
        if filter_result.status == PodDeleteStatus::Error {
            filterd_err.push(filter_result.desc);
            continue;
        }
        if filter_result.result {
            filterd_pods_list.push(pod);
        }
    }
    if filterd_err.len() > 0 {
        return Err(DeletePodsError { errors: filterd_err });
    }
    Ok(filterd_pods_list.into_iter())
}

async fn evict_pod(k8s_client: &kube::Client, pod: &Pod, force: bool) -> Result<(), error::EvictionError> {
    let pod_api: Api<Pod> = get_pod_api_with_namespace(k8s_client, pod);

    let error_handling_strategy =
        if force { ErrorHandleStrategy::RetryStrategy } else { ErrorHandleStrategy::TolerateStrategy };

    RetryIf::spawn(
        error_handling_strategy.retry_strategy(),
        || async {
            loop {
                let eviction_result = pod_api.evict(&pod.name_any(), &EvictParams::default()).await;

                match eviction_result {
                    Ok(_) => {
                        pod.name();
                        debug!("Successfully evicted Pod '{}'", pod.name_any());
                        break;
                    }
                    Err(kube::Error::Api(e)) => {
                        let status_code = StatusCode::from_u16(e.code);
                        match status_code {
                            Ok(StatusCode::FORBIDDEN) => {
                                return Err(EvictionErrorNoRetry {
                                    source: kube::Error::Api(e.clone()),
                                    pod_name: pod.name_any(),
                                });
                            }
                            Ok(StatusCode::NOT_FOUND) => {
                                return Err(EvictionErrorNoRetry {
                                    source: kube::Error::Api(e.clone()),
                                    pod_name: pod.name_any(),
                                });
                            }
                            Ok(StatusCode::INTERNAL_SERVER_ERROR) => {
                                error!(
                                    "Evict pod {} reported error: '{}' and will retry in {:.2}s. This error maybe is due to misconfigured PodDisruptionBudgets.",
                                    pod.name_any(),
                                    e,
                                    EVERY_EVICTION_RETRY.as_secs_f64()
                                );
                                sleep(EVERY_EVICTION_RETRY).await;
                                continue;
                            }
                            Ok(StatusCode::TOO_MANY_REQUESTS) => {
                                error!("Evict pod {} reported error: '{}' and will retry in {:.2}s. This error maybe is due to PodDisruptionBugets.",
                                    pod.name_any(),
                                    e,
                                    EVERY_EVICTION_RETRY.as_secs_f64()
                                );
                                sleep(EVERY_EVICTION_RETRY).await;
                                continue;
                            }
                            Ok(_) => {
                                error!(
                                    "Evict pod {} reported error: '{}'.",
                                    pod.name_any(),
                                    e
                                );
                                return Err(EvictionErrorRetry {
                                    source: kube::Error::Api(e.clone()),
                                    pod_name: pod.name_any(),
                                });
                            }
                            Err(_) => {
                                error!(
                                    "Evict pod {} reported error: '{}'.Received invalid response code from Kubernetes API",
                                    pod.name_any(),
                                    e
                                );
                                return Err(EvictionErrorRetry {
                                    source: kube::Error::Api(e.clone()),
                                    pod_name: pod.name_any(),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        error!("Evict pod {} reported error: '{}' and will retry", pod.name_any(),e);
                        return Err(EvictionErrorRetry {
                            source: e,
                            pod_name: pod.name_any(),
                        });
                    }
                }
            }
            Ok(())
        },
        error_handling_strategy
    ).await
}

async fn wait_for_deletion(k8s_client: &kube::Client, pod: &Pod) -> Result<(), error::DrainError> {
    let start_time = Instant::now();

    let pod_api: Api<Pod> = get_pod_api_with_namespace(k8s_client, pod);
    let response_error_not_found: u16 = 404;
    loop {
        match pod_api.get(&pod.name_any()).await {
            Ok(p) if p.uid() != pod.uid() => {
                let name = (&p).name_any();
                info!("Pod {} deleted.", name);
                break;
            },
            Ok(_) => {
                info!("Pod '{}' is not yet deleted. Waiting {}s.", pod.name_any(), EVERY_DELETION_CHECK.as_secs_f64());
            },
            Err(kube::Error::Api(e)) if e.code == response_error_not_found => {
                info!("Pod {} is deleted.", pod.name_any());
                break;
            },
            Err(e) => {
                error!(
                    "Get pod {} reported error: '{}', whether pod is deleted cannot be determined, waiting {}s.",
                    pod.name_any(),
                    e,
                    EVERY_DELETION_CHECK.as_secs_f64()
                );
            },
        }
        if start_time.elapsed() > TIMEOUT {
            return Err(WaitDeletionError { pod_name: pod.name_any(), max_wait: TIMEOUT });
        } else {
            sleep(EVERY_DELETION_CHECK).await;
        }
    }
    Ok(())
}

fn get_pod_api_with_namespace(client: &kube::Client, pod: &Pod) -> Api<Pod> {
    match pod.metadata.namespace.as_ref() {
        Some(namespace) => Api::namespaced(client.clone(), namespace),
        None => Api::default_namespaced(client.clone()),
    }
}

trait NameAny {
    fn name_any(self: &Self) -> String;
}

impl NameAny for &Pod {
    fn name_any(self: &Self) -> String {
        self.metadata.name.clone().or_else(|| self.metadata.generate_name.clone()).unwrap_or_default()
    }
}
trait PodFilter {
    fn filter(self: &Self, pod: &Pod) -> Box<FilterResult>;
}

struct FinishedOrFailedFilter {}
impl PodFilter for FinishedOrFailedFilter {
    fn filter(self: &Self, pod: &Pod) -> Box<FilterResult> {
        return match pod.status.as_ref() {
            Some(PodStatus { phase: Some(phase), .. }) if phase == "Failed" || phase == "Succeeded" => {
                FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay)
            },
            _ => FilterResult::create_filter_result(false, "", PodDeleteStatus::Okay),
        };
    }
}
struct DaemonFilter {
    finished_or_failed_filter: FinishedOrFailedFilter,
    force: bool,
}
impl PodFilter for DaemonFilter {
    fn filter(self: &Self, pod: &Pod) -> Box<FilterResult> {
        if let FilterResult { result: true, .. } = self.finished_or_failed_filter.filter(pod).as_ref() {
            return FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay);
        }

        return match pod.metadata.owner_references.as_ref() {
            Some(owner_references)
                if owner_references
                    .iter()
                    .any(|reference| reference.controller.unwrap_or(false) && reference.kind == "DaemonSet") =>
            {
                if self.force {
                    let description = format!("Ignore Pod '{}': Pod is member of a DaemonSet", pod.name_any());
                    Box::new(FilterResult { result: false, desc: description, status: PodDeleteStatus::Warning })
                } else {
                    let description = format!("Cannot drain Pod '{}': Pod is member of a DaemonSet", pod.name_any());
                    Box::new(FilterResult { result: false, desc: description, status: PodDeleteStatus::Error })
                }
            },
            _ => FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay),
        };
    }
}
impl DaemonFilter {
    fn new(force: bool) -> DaemonFilter {
        return DaemonFilter { finished_or_failed_filter: FinishedOrFailedFilter {}, force: force };
    }
}

struct MirrorFilter {}
impl PodFilter for MirrorFilter {
    fn filter(self: &Self, pod: &Pod) -> Box<FilterResult> {
        return match pod.metadata.annotations.as_ref() {
            Some(annotations) if annotations.contains_key("kubernetes.io/config.mirror") => {
                let description = format!("Ignore Pod '{}': Pod is a static Mirror Pod", pod.name_any());
                FilterResult::create_filter_result(false, &description.to_string(), PodDeleteStatus::Warning)
            },
            _ => FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay),
        };
    }
}

struct LocalStorageFilter {
    finished_or_failed_filter: FinishedOrFailedFilter,
    force: bool,
}
impl PodFilter for LocalStorageFilter {
    fn filter(self: &Self, pod: &Pod) -> Box<FilterResult> {
        if let FilterResult { result: true, .. } = self.finished_or_failed_filter.filter(pod).as_ref() {
            return FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay);
        }

        return match pod.spec.as_ref() {
            Some(PodSpec { volumes: Some(volumes), .. }) if volumes.iter().any(|volume| volume.empty_dir.is_some()) => {
                if self.force {
                    let description = format!("Force draining Pod '{}': Pod has local storage", pod.name_any());
                    Box::new(FilterResult { result: true, desc: description, status: PodDeleteStatus::Warning })
                } else {
                    let description = format!("Cannot drain Pod '{}': Pod has local Storage", pod.name_any());
                    Box::new(FilterResult { result: false, desc: description, status: PodDeleteStatus::Error })
                }
            },
            _ => FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay),
        };
    }
}
impl LocalStorageFilter {
    fn new(force: bool) -> LocalStorageFilter {
        return LocalStorageFilter { finished_or_failed_filter: FinishedOrFailedFilter {}, force: force };
    }
}
struct UnreplicatedFilter {
    finished_or_failed_filter: FinishedOrFailedFilter,
    force: bool,
}
impl PodFilter for UnreplicatedFilter {
    fn filter(self: &Self, pod: &Pod) -> Box<FilterResult> {
        if let FilterResult { result: true, .. } = self.finished_or_failed_filter.filter(pod).as_ref() {
            return FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay);
        }

        let is_replicated = pod.metadata.owner_references.is_some();

        if is_replicated {
            return FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay);
        }

        return if !is_replicated && self.force {
            let description = format!("Force drain Pod '{}': Pod is unreplicated", pod.name_any());
            Box::new(FilterResult { result: true, desc: description, status: PodDeleteStatus::Warning })
        } else {
            let description = format!("Cannot drain Pod '{}': Pod is unreplicated", pod.name_any());
            Box::new(FilterResult { result: false, desc: description, status: PodDeleteStatus::Error })
        };
    }
}
impl UnreplicatedFilter {
    fn new(force: bool) -> UnreplicatedFilter {
        return UnreplicatedFilter { finished_or_failed_filter: FinishedOrFailedFilter {}, force: force };
    }
}

struct DeletedFilter {
    delete_wait_timeout: Duration,
}
impl PodFilter for DeletedFilter {
    fn filter(self: &Self, pod: &Pod) -> Box<FilterResult> {
        let now = Instant::now().elapsed();
        return match pod.metadata.deletion_timestamp.as_ref() {
            Some(time)
                if time.0.timestamp() != 0
                    && now - Duration::from_secs(time.0.timestamp() as u64) >= self.delete_wait_timeout =>
            {
                FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay)
            },
            _ => FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay),
        };
    }
}

struct CombinedFilter {
    deleted_filter: DeletedFilter,
    daemon_filter: DaemonFilter,
    mirror_filter: MirrorFilter,
    local_storage_filter: LocalStorageFilter,
    unreplicated_filter: UnreplicatedFilter,
}
impl PodFilter for CombinedFilter {
    fn filter(self: &Self, pod: &Pod) -> Box<FilterResult> {
        let mut filter_res = self.deleted_filter.filter(pod);
        if !filter_res.result {
            info!("{}", filter_res.desc);
            return Box::new(FilterResult {
                result: filter_res.result,
                desc: filter_res.desc.clone(),
                status: filter_res.status.clone(),
            });
        }
        filter_res = self.daemon_filter.filter(pod);
        if !filter_res.result {
            info!("{}", filter_res.desc);
            return Box::new(FilterResult {
                result: filter_res.result,
                desc: filter_res.desc.clone(),
                status: filter_res.status.clone(),
            });
        }
        filter_res = self.mirror_filter.filter(pod);
        if !filter_res.result {
            info!("{}", filter_res.desc);
            return Box::new(FilterResult {
                result: filter_res.result,
                desc: filter_res.desc.clone(),
                status: filter_res.status.clone(),
            });
        }
        filter_res = self.local_storage_filter.filter(pod);
        if !filter_res.result {
            info!("{}", filter_res.desc);
            return Box::new(FilterResult {
                result: filter_res.result,
                desc: filter_res.desc.clone(),
                status: filter_res.status.clone(),
            });
        }
        filter_res = self.unreplicated_filter.filter(pod);
        if !filter_res.result {
            info!("{}", filter_res.desc);
            return Box::new(FilterResult {
                result: filter_res.result,
                desc: filter_res.desc.clone(),
                status: filter_res.status.clone(),
            });
        }

        return FilterResult::create_filter_result(true, "", PodDeleteStatus::Okay);
    }
}
impl CombinedFilter {
    fn new(force: bool) -> CombinedFilter {
        return CombinedFilter {
            deleted_filter: DeletedFilter { delete_wait_timeout: TIMEOUT },
            daemon_filter: DaemonFilter::new(force),
            mirror_filter: MirrorFilter {},
            local_storage_filter: LocalStorageFilter::new(force),
            unreplicated_filter: UnreplicatedFilter::new(force),
        };
    }
}

#[derive(PartialEq, Clone, Copy)]
enum PodDeleteStatus {
    Okay,
    Warning,
    Error,
}
struct FilterResult {
    result: bool,
    desc: String,
    status: PodDeleteStatus,
}
impl FilterResult {
    fn create_filter_result(result: bool, desc: &str, status: PodDeleteStatus) -> Box<FilterResult> {
        Box::new(FilterResult { result: result, desc: desc.to_string(), status: status })
    }
}

enum ErrorHandleStrategy {
    RetryStrategy,
    TolerateStrategy,
}

impl ErrorHandleStrategy {
    fn retry_strategy(&self) -> impl Iterator<Item = Duration> {
        let backoff =
            ExponentialBackoff::from_millis(RETRY_BASE_DELAY.as_millis() as u64).max_delay(RETRY_MAX_DELAY).map(jitter);

        return match self {
            Self::TolerateStrategy => {
                return backoff.take(0);
            },

            Self::RetryStrategy => backoff.take(MAX_RETRIES_TIMES),
        };
    }
}

impl tokio_retry::Condition<error::EvictionError> for ErrorHandleStrategy {
    fn should_retry(&mut self, error: &error::EvictionError) -> bool {
        match self {
            Self::TolerateStrategy => false,
            Self::RetryStrategy => {
                if let error::EvictionError::EvictionErrorRetry { .. } = error {
                    true
                } else {
                    false
                }
            },
        }
    }
}

pub mod error {
    use thiserror::Error;
    use tokio::time::Duration;

    #[derive(Debug, Error)]
    pub enum DrainError {
        #[error("Get node {} pods list error reported: {}", node_name, source)]
        GetPodListsError { source: kube::Error, node_name: String },

        #[error("Pod '{}' was not deleted in the time allocated ({:.2}s).",pod_name,max_wait.as_secs_f64())]
        WaitDeletionError { pod_name: String, max_wait: Duration },
        #[error("")]
        DeletePodsError { errors: Vec<String> },
    }

    #[derive(Debug, Error)]
    pub enum EvictionError {
        #[error("Evict Pod {} error: '{}'", pod_name, source)]
        EvictionErrorRetry { source: kube::Error, pod_name: String },

        #[error("Evict Pod {} error: '{}'", pod_name, source)]
        EvictionErrorNoRetry { source: kube::Error, pod_name: String },
    }
}
