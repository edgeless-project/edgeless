use edgeless_dataplane::core::Message;
extern crate redis;
use redis::Commands;

pub struct RedisResourceProvider {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
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
    async fn new(dataplane_handle: edgeless_dataplane::handle::DataplaneHandle, redis_url: &str, redis_key: &str) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let redis_key = redis_key.to_string();

        let mut connection = redis::Client::open(redis_url)?.get_connection()?;

        log::info!("RedisResource created, URL: {}", redis_url);

        let handle = tokio::spawn(async move {
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

                if let Err(e) = connection.set::<&str, &str, std::string::String>(&redis_key, &message_data) {
                    log::error!("Could not set key '{}' to value '{}': {}", redis_key, &message_data, e);
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

impl RedisResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            resource_provider_id,
            dataplane_provider,
            instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, RedisResource>::new(),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI for RedisResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::resource_configuration::SpawnResourceResponse> {
        if let (Some(url), Some(key)) = (
            instance_specification.configuration.get("url"),
            instance_specification.configuration.get("key"),
        ) {
            let new_id = edgeless_api::function_instance::InstanceId::new(self.resource_provider_id.node_id);
            let dataplane_handle = self.dataplane_provider.get_handle_for(new_id.clone()).await;

            match RedisResource::new(dataplane_handle, url, key).await {
                Ok(resource) => {
                    self.instances.insert(new_id.clone(), resource);
                    return Ok(edgeless_api::resource_configuration::SpawnResourceResponse::InstanceId(new_id));
                }
                Err(err) => {
                    return Ok(edgeless_api::resource_configuration::SpawnResourceResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: "Invalid resource configuration".to_string(),
                            detail: Some(err.to_string()),
                        },
                    ));
                }
            }
        }

        Ok(edgeless_api::resource_configuration::SpawnResourceResponse::ResponseError(
            edgeless_api::common::ResponseError {
                summary: "Invalid resource configuration".to_string(),
                detail: Some("One of the fields 'url' or 'key' is missing".to_string()),
            },
        ))
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.instances.remove(&resource_id);
        Ok(())
    }
}
