// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use base64::Engine;
use edgeless_dataplane::core::Message;

pub struct HttpPosterResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for HttpPosterResourceSpec {
    fn class_type(&self) -> String {
        String::from("http-poster")
    }

    fn description(&self) -> String {
        r"Post the message received via cast() to a web server".to_string()
    }

    fn outputs(&self) -> Vec<String> {
        vec![]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            (String::from("url"), String::from("The URL of the target web server")),
            (
                String::from("decode_base64"),
                String::from("Assume the incoming message is base64-encoded"),
            ),
        ])
    }

    fn version(&self) -> String {
        String::from("1.0")
    }
}

#[derive(Clone)]
pub struct HttpPosterResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<HttpPosterResourceProviderInner>>,
}

struct HttpPosterResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, HttpPosterResource>,
}

pub struct HttpPosterResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for HttpPosterResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl HttpPosterResource {
    async fn new(
        url: String,
        decode_base64: bool,
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> Self {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id: _,
                    channel_id: _,
                    message,
                    created,
                    metadata: _,
                } = dataplane_handle.receive_next().await;
                let started = crate::resources::observe_transfer(created, &mut telemetry_handle);
                let message_data = match message {
                    Message::Cast(data) => data,
                    _ => {
                        continue;
                    }
                };

                let client = reqwest::Client::new();

                let mut client_r = client.request(reqwest::Method::POST, &url);

                if decode_base64 {
                    match base64::engine::general_purpose::STANDARD.decode(message_data) {
                        Ok(vec) => client_r = client_r.body(vec),
                        Err(err) => log::warn!("invalid base64-encoded data received: {err}"),
                    }
                } else {
                    client_r = client_r.body(message_data);
                }

                if let Err(err) = client_r.send().await {
                    log::warn!("error when posting to '{url}': {err}");
                }

                crate::resources::observe_execution(started, &mut telemetry_handle, true);
            }
        });

        Self { join_handle: handle }
    }
}

impl HttpPosterResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(HttpPosterResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                telemetry_handle,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, HttpPosterResource>::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for HttpPosterResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let url = instance_specification.configuration.get("url").unwrap_or(&String::default()).clone();
        if let Err(err) = reqwest::Url::parse(&url) {
            return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Error when creating a http-poster resource".to_string(),
                    detail: Some(format!("Invalid url field '{url}': {err}")),
                },
            ));
        }
        let decode_base64 = instance_specification
            .configuration
            .get("decode_base64")
            .unwrap_or(&String::from("false"))
            .eq_ignore_ascii_case("true");

        let mut lck = self.inner.lock().await;

        let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
        let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;

        let telemetry_handle = lck.telemetry_handle.fork(std::collections::BTreeMap::from([(
            "FUNCTION_ID".to_string(),
            new_id.function_id.to_string(),
        )]));
        lck.instances.insert(
            new_id,
            HttpPosterResource::new(url, decode_base64, dataplane_handle, telemetry_handle).await,
        );

        Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
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
