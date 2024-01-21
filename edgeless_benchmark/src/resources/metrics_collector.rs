// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_dataplane::core::Message;
extern crate redis;
use redis::Commands;

// Smoothing factor of the EWMA of function/workflow latencies.
const ALPHA: f64 = 0.9_f64;

#[derive(Clone)]
pub struct MetricsCollectorResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<MetricsCollectorResourceProviderInner>>,
}

pub struct MetricsCollectorResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, MetricsCollectorResource>,
}

pub struct MetricsCollectorResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for MetricsCollectorResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl MetricsCollectorResource {
    async fn new(dataplane_handle: edgeless_dataplane::handle::DataplaneHandle, redis_url: &str) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;

        let mut connection = redis::Client::open(redis_url)?.get_connection()?;

        log::info!("MetricsCollectorResource created, URL: {}", redis_url);

        let handle = tokio::spawn(async move {
            let mut workflow_ts = std::collections::HashMap::new();
            let mut function_ts = std::collections::HashMap::new();
            let mut workflow_avg_lat = std::collections::HashMap::new();
            let mut function_avg_lat = std::collections::HashMap::new();
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                } = dataplane_handle.receive_next().await;

                let mut need_reply = false;
                let message_data = match message {
                    Message::Call(data) => {
                        need_reply = true;
                        data
                    }
                    Message::Cast(data) => data,
                    _ => {
                        continue;
                    }
                };

                let tokens: Vec<&str> = message_data.split(':').collect();
                if tokens.len() == 4 && tokens[0] == "workflow" {
                    let command = tokens[1];
                    let wf_name = tokens[2];
                    let transaction_id = tokens[3];
                    let key = format!("{}:{}", wf_name, transaction_id).to_string();
                    if command == "start" {
                        workflow_ts.insert(key, std::time::Instant::now());
                    } else if command == "end" {
                        if let Some(ts) = workflow_ts.remove(&key) {
                            let redis_key = format!("workflow:latencies:{}", wf_name).to_string();
                            let latency = ts.elapsed().as_millis() as i64;
                            if let Err(e) = connection.lpush::<&str, i64, usize>(&redis_key, latency) {
                                log::error!("Could not lpush value '{}' to key '{}': {}", latency, redis_key, e);
                            }
                            let avg_key: String = format!("{}", wf_name).to_string();
                            let avg_latency = match workflow_avg_lat.get(&avg_key) {
                                Some(prev_value) => latency as f64 * ALPHA + (1.0_f64 - ALPHA) * prev_value,
                                None => latency as f64,
                            };
                            let redis_key = format!("workflow:avg-latency:{}", wf_name).to_string();
                            workflow_avg_lat.insert(avg_key, avg_latency);
                            if let Err(e) = connection.set::<&str, f64, std::string::String>(&redis_key, avg_latency) {
                                log::error!("Could not set value '{}' for key '{}': {}", avg_latency, redis_key, e);
                            }
                        }
                    } else {
                        log::error!("invalid workflow command '{}' in: {}", tokens[1], message_data);
                    }
                } else if tokens.len() == 5 && tokens[0] == "function" {
                    let command = tokens[1];
                    let wf_name = tokens[2];
                    let fun_name = tokens[3];
                    let transaction_id = tokens[3];
                    let key: String = format!("{}:{}:{}", wf_name, fun_name, transaction_id).to_string();
                    if command == "start" {
                        function_ts.insert(key, std::time::Instant::now());
                    } else if command == "end" {
                        if let Some(ts) = function_ts.remove(&key) {
                            let redis_key = format!("function:latencies:{}:{}", wf_name, fun_name).to_string();
                            let latency = ts.elapsed().as_millis() as i64;
                            if let Err(e) = connection.lpush::<&str, i64, usize>(&redis_key, latency) {
                                log::error!("Could not lpush value '{}' to key '{}': {}", latency, redis_key, e);
                            }
                            let avg_key: String = format!("{}:{}", wf_name, fun_name).to_string();
                            let avg_latency = match function_avg_lat.get(&avg_key) {
                                Some(prev_value) => latency as f64 * ALPHA + (1.0_f64 - ALPHA) * prev_value,
                                None => latency as f64,
                            };
                            let redis_key = format!("function:avg-latency:{}", avg_key).to_string();
                            function_avg_lat.insert(avg_key, avg_latency);
                            if let Err(e) = connection.set::<&str, f64, std::string::String>(&redis_key, avg_latency) {
                                log::error!("Could not set value '{}' for key '{}': {}", avg_latency, redis_key, e);
                            }
                        }
                    } else {
                        log::error!("invalid workflow command '{}' in: {}", tokens[1], message_data);
                    }
                } else {
                    log::error!("invalid metric command received: {}", message_data);
                }

                if need_reply {
                    dataplane_handle
                        .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply("".to_string()))
                        .await;
                }
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl MetricsCollectorResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(MetricsCollectorResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, MetricsCollectorResource>::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>
    for MetricsCollectorResourceProvider
{
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        if let Some(url) = instance_specification.configuration.get("url") {
            let mut lck = self.inner.lock().await;
            let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id.clone()).await;

            match MetricsCollectorResource::new(dataplane_handle, url).await {
                Ok(resource) => {
                    lck.instances.insert(new_id.clone(), resource);
                    return Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id));
                }
                Err(err) => {
                    return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: "Invalid resource configuration".to_string(),
                            detail: Some(err.to_string()),
                        },
                    ));
                }
            }
        }

        Ok(edgeless_api::common::StartComponentResponse::ResponseError(
            edgeless_api::common::ResponseError {
                summary: "Invalid resource configuration".to_string(),
                detail: Some("Field 'url' is missing".to_string()),
            },
        ))
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // the resource has no channels: nothing to be patched
        Ok(())
    }
}
