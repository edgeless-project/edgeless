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

    fn description(&self) -> String {
        r"Perform SET and GET operations on a Redis server -- https://redis.io/

        A SET operation is performed with a cast() on the key specified in the 'key' configuration parameter of the resource.
        A GET operation is performed with a call(), with the key specified in the message body."
            .to_string()
    }

    fn outputs(&self) -> Vec<String> {
        vec![]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            (String::from("url"), String::from("URL of the Redis server to use")),
            (String::from("key"), String::from("Key for SET operations (optional)")),
            (
                String::from("add-workflow-id"),
                String::from("If present, add the workflow identifier to the key"),
            ),
        ])
    }

    fn version(&self) -> String {
        String::from("1.2")
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

/// The redis resource can be used to access a Redis KVS.
/// Each resource instance has its own connection at the Redis URL specified
/// in the resource configuration.
/// The same resource can be used to GET or SET keys.
///
/// The GET operation is done on an arbitrary key that is specified as the
/// message of the call() operation.
///
/// The SET operation is done via a cast() on the key. There are two options:
///
/// 1. The key can be specified in the resource configuration.
/// 2. The key can be specified in the message data as "key:value"
///   (i.e., the message contains a colon). In this case, the part before the colon
///   is used as the key.
/// 
/// If the resource configuration contains the "add-workflow-id" parameter,
/// the workflow ID is prepended to the key used in both GET and SET operations.
impl RedisResource {
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        redis_url: &str,
        redis_key: Option<&String>,
        workflow_id: Option<String>,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;

        log::info!(
            "RedisResource created, url {}, key {:?}, workflow_id {:?}",
            redis_url,
            redis_key,
            workflow_id
        );

        let workflow_id_header = if let Some(workflow_id) = workflow_id {
            format!("{}:", workflow_id)
        } else {
            String::default()
        };
        let redis_key = redis_key.cloned().map(|k| format!("{}{}", workflow_id_header, k));

        let mut connection = redis::Client::open(redis_url)?.get_connection()?;

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    created,
                    metadata,
                } = dataplane_handle.receive_next().await;
                let started = crate::resources::observe_transfer(created, &mut telemetry_handle);

                let (get_operation, message_data) = match message {
                    Message::Call(data) => (true, data),
                    Message::Cast(data) => (false, data),
                    _ => {
                        continue;
                    }
                };

                if get_operation {
                    // GET
                    let redis_key = format!("{}{}", workflow_id_header, message_data);
                    match connection.get::<&str, std::string::String>(&redis_key) {
                        Ok(res) => {
                            dataplane_handle
                                .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply(res), &metadata)
                                .await
                        }
                        Err(err) => {
                            log::error!("Could not get key '{}' from redis resource: {}", redis_key, err);
                            dataplane_handle
                                .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Err, &metadata)
                                .await
                        }
                    };
                } else {
                    // SET on dynamic key if the message contains colon
                    if let Some(colon_pos) = message_data.find(':') {
                        let (k_str, v_str) = message_data.split_at(colon_pos);
                        let v_str = &v_str[1..]; // skip the colon
                        let redis_key = format!("{}{}", workflow_id_header, k_str);
                        if let Err(err) = connection.set::<&str, &str, std::string::String>(&redis_key, v_str) {
                            log::error!(
                                "Could not set to dynamic key '{}' to value '{}' via redis resource: {}",
                                redis_key,
                                v_str,
                                err
                            );
                        }
                    } else {
                        // SET on the fixed key specified in the resource configuration
                        if let Some(redis_key) = &redis_key {
                            if let Err(err) = connection.set::<&str, &str, std::string::String>(redis_key, &message_data) {
                                log::error!(
                                    "Could not set to fixed key '{}' to value '{}' via redis resource: {}",
                                    redis_key,
                                    &message_data,
                                    err
                                );
                            }
                        } else {
                            log::warn!("Invalid SET operation requested on a redis resource without a 'key' specified in the configuration");
                        }
                    }
                }

                crate::resources::observe_execution(started, &mut telemetry_handle, get_operation);
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
        if let Some(url) = instance_specification.configuration.get("url") {
            let mut lck = self.inner.lock().await;
            let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;
            let telemetry_handle = lck.telemetry_handle.fork(std::collections::BTreeMap::from([(
                "FUNCTION_ID".to_string(),
                new_id.function_id.to_string(),
            )]));

            match RedisResource::new(
                dataplane_handle,
                telemetry_handle,
                url,
                instance_specification.configuration.get("key"),
                instance_specification
                    .configuration
                    .contains_key("add-workflow-id")
                    .then_some(instance_specification.workflow_id),
            )
            .await
            {
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
                detail: Some("Missing Redis URL".to_string()),
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
