// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_dataplane::core::Message;

pub struct HttpEgressResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for HttpEgressResourceSpec {
    fn class_type(&self) -> String {
        String::from("http-egress")
    }

    fn description(&self) -> String {
        r"Execute HTTP commands on external web servers".to_string()
    }

    fn outputs(&self) -> Vec<String> {
        vec![]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }

    fn version(&self) -> String {
        String::from("1.1")
    }
}

#[derive(Clone)]
pub struct EgressResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<EgressResourceProviderInner>>,
}

struct EgressResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    egress_instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, EgressResource>,
}

pub struct EgressResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for EgressResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl EgressResource {
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> Self {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    created,
                } = dataplane_handle.receive_next().await;
                let started = crate::resources::observe_transfer(created, &mut telemetry_handle);
                let message_data = match message {
                    Message::Call(data) => data,
                    _ => {
                        continue;
                    }
                };

                let req = match edgeless_http::request_from_string(&message_data) {
                    Ok(val) => val,
                    Err(_) => {
                        dataplane_handle
                            .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Err)
                            .await;
                        continue;
                    }
                };
                let mut cloned_dataplane = dataplane_handle.clone();
                tokio::spawn(async move {
                    match Self::perform_request(req).await {
                        Ok(resp) => {
                            let serialized_resp = edgeless_http::response_to_string(&resp);
                            cloned_dataplane
                                .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply(serialized_resp))
                                .await;
                        }
                        Err(_) => {
                            cloned_dataplane
                                .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Err)
                                .await;
                        }
                    }
                });
                crate::resources::observe_execution(started, &mut telemetry_handle, true);
            }
        });

        Self { join_handle: handle }
    }

    async fn perform_request(req: edgeless_http::EdgelessHTTPRequest) -> anyhow::Result<edgeless_http::EdgelessHTTPResponse> {
        let method = reqwest::Method::from_bytes(edgeless_http::edgeless_method_to_string(req.method).as_bytes())?;

        let protocol_string = match req.protocol {
            edgeless_http::EdgelessHTTPProtocol::HTTPS => "HTTPS",
            _ => "HTTP",
        };

        let url = format!("{}://{}{}", protocol_string, req.host, req.path);

        let client = reqwest::Client::new();

        let mut client_r = client.request(method, url);

        if let Some(b) = req.body {
            client_r = client_r.body(b);
        }

        for (header_key, header_val) in req.headers {
            client_r = client_r.header(header_key, header_val);
        }

        let ret = client_r.send().await?;

        let headers = ret
            .headers()
            .iter()
            .filter_map(|(k, v)| match v.to_str() {
                Ok(value) => Some((k.to_string(), value.to_string())),
                _ => {
                    log::warn!("Could not parse received header value");
                    None
                }
            })
            .collect();

        Ok(edgeless_http::EdgelessHTTPResponse {
            status: ret.status().as_u16(),
            headers,
            body: match ret.bytes().await {
                Ok(btes) => Some(btes.to_vec()),
                _ => None,
            },
        })
    }
}

impl EgressResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(EgressResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                telemetry_handle,
                egress_instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, EgressResource>::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for EgressResourceProvider {
    async fn start(
        &mut self,
        _instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let mut lck = self.inner.lock().await;

        let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
        let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;

        let telemetry_handle = lck.telemetry_handle.fork(std::collections::BTreeMap::from([(
            "FUNCTION_ID".to_string(),
            new_id.function_id.to_string(),
        )]));
        lck.egress_instances
            .insert(new_id, EgressResource::new(dataplane_handle, telemetry_handle).await);

        Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.inner.lock().await.egress_instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // the resource has no channels: nothing to be patched
        Ok(())
    }
}
