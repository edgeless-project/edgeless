// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct KafkaEgressResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for KafkaEgressResourceSpec {
    fn class_type(&self) -> String {
        String::from("kafka-egress")
    }

    fn description(&self) -> String {
        r"Send a message to an external Apache Kafka server -- see https://kafka.apache.org/"
            .to_string()
    }

    fn outputs(&self) -> Vec<String> {
        vec![]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            (
                String::from("brokers"),
                String::from("Comma-separated list of initial brokers to access the cluster"),
            ),
            (
                String::from("topic"),
                String::from("Topic to which messages are posted"),
            ),
        ])
    }

    fn version(&self) -> String {
        String::from("1.1")
    }
}

#[derive(Clone)]
pub struct KafkaEgressResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<KafkaEgressResourceProviderInner>>,
}

pub struct KafkaEgressResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    instances:
        std::collections::HashMap<edgeless_api::function_instance::InstanceId, KafkaEgressResource>,
}

pub struct KafkaEgressResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for KafkaEgressResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl KafkaEgressResource {
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        kafka_brokers: &str,
        kafka_topic: &str,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;
        let kafka_brokers = kafka_brokers.to_string();
        let kafka_topic = kafka_topic.to_string();

        let producer: rdkafka::producer::BaseProducer = rdkafka::config::ClientConfig::new()
            .set("bootstrap.servers", &kafka_brokers)
            .create()?;

        log::info!("KafkaEgressResource created, brokers: {}", kafka_brokers);

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
                    edgeless_dataplane::core::Message::Call(data) => {
                        need_reply = true;
                        data
                    }
                    edgeless_dataplane::core::Message::Cast(data) => data,
                    _ => {
                        continue;
                    }
                };

                if let Err(e) = producer.send(
                    rdkafka::producer::BaseRecord::to(&kafka_topic)
                        .payload(&message_data)
                        .key(""),
                ) {
                    log::error!("Failed to send message to topic '{}': {:?}", kafka_topic, e);
                }

                if need_reply {
                    dataplane_handle
                        .reply(
                            source_id,
                            channel_id,
                            edgeless_dataplane::core::CallRet::Reply("".to_string()),
                        )
                        .await;
                }

                crate::resources::observe_execution(started, &mut telemetry_handle);
            }
        });

        Ok(Self {
            join_handle: handle,
        })
    }
}

impl KafkaEgressResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(KafkaEgressResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                telemetry_handle,
                instances: std::collections::HashMap::<
                    edgeless_api::function_instance::InstanceId,
                    KafkaEgressResource,
                >::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl
    edgeless_api::resource_configuration::ResourceConfigurationAPI<
        edgeless_api::function_instance::InstanceId,
    > for KafkaEgressResourceProvider
{
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<
        edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>,
    > {
        if let (Some(brokers), Some(topic)) = (
            instance_specification.configuration.get("brokers"),
            instance_specification.configuration.get("topic"),
        ) {
            let mut lck = self.inner.lock().await;
            let new_id =
                edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;
            let telemetry_handle = lck
                .telemetry_handle
                .fork(std::collections::BTreeMap::from([(
                    "FUNCTION_ID".to_string(),
                    new_id.function_id.to_string(),
                )]));

            match KafkaEgressResource::new(dataplane_handle, telemetry_handle, brokers, topic).await
            {
                Ok(resource) => {
                    lck.instances.insert(new_id, resource);
                    return Ok(edgeless_api::common::StartComponentResponse::InstanceId(
                        new_id,
                    ));
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
                detail: Some("One of the fields 'brokers' or 'topic' is missing".to_string()),
            },
        ))
    }

    async fn stop(
        &mut self,
        resource_id: edgeless_api::function_instance::InstanceId,
    ) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        Ok(())
    }
}
