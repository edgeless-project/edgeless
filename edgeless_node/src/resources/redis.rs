// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_dataplane::core::Message;
extern crate redis;
use redis::Commands;

pub struct RedisResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for RedisResourceSpec {
    fn class_type(&self) -> String {
        String::from("redis")
    }

    fn outputs(&self) -> Vec<String> {
        vec![]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            (String::from("url"), String::from("URL of the Redis server to use")),
            (String::from("key"), String::from("Key to set")),
        ])
    }

    fn version(&self) -> String {
        String::from("1.1")
    }
}

#[derive(Clone)]
pub struct RedisResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<RedisResourceProviderInner>>,
}

pub struct RedisResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, RedisResource>,
}

pub struct RedisResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for RedisResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl RedisResource {
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        redis_url: &str,
        redis_key: &str,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;
        let redis_key = redis_key.to_string();

        let mut connection = redis::Client::open(redis_url)?.get_connection()?;

        log::info!("RedisResource created, URL: {}", redis_url);

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    created,
                } = dataplane_handle.receive_next().await;
                let started = crate::resources::observe_transfer(created, &mut telemetry_handle);

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

                if let Err(e) = connection.set::<&str, &str, std::string::String>(&redis_key, &message_data) {
                    log::error!("Could not set key '{}' to value '{}': {}", redis_key, &message_data, e);
                }

                if need_reply {
                    dataplane_handle
                        .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply("".to_string()))
                        .await;
                }

                crate::resources::observe_execution(started, &mut telemetry_handle);
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl RedisResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(RedisResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                telemetry_handle,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, RedisResource>::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for RedisResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        if let (Some(url), Some(key)) = (
            instance_specification.configuration.get("url"),
            instance_specification.configuration.get("key"),
        ) {
            let mut lck = self.inner.lock().await;
            let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;
            let telemetry_handle = lck.telemetry_handle.fork(std::collections::BTreeMap::from([(
                "FUNCTION_ID".to_string(),
                new_id.function_id.to_string(),
            )]));

            match RedisResource::new(dataplane_handle, telemetry_handle, url, key).await {
                Ok(resource) => {
                    lck.instances.insert(new_id, resource);
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
                detail: Some("One of the fields 'url' or 'key' is missing".to_string()),
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
