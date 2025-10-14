// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_dataplane::core::Message;

#[derive(Clone)]
pub struct PortalResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<PortalResourceProviderInner>>,
}

pub struct PortalResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, PortalResource>,
}

pub struct PortalResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for PortalResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

/// Portal resource, to teleport messages from one domain to another.
impl PortalResource {
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;

        log::info!("Portal created",);

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id: _,
                    channel_id: _,
                    message,
                    created,
                    metadata: _,
                } = dataplane_handle.receive_next().await;
                let started = edgeless_node::resources::observe_transfer(created, &mut telemetry_handle);

                let need_reply = match message {
                    Message::Call(_data) => true,
                    Message::Cast(_data) => false,
                    _ => {
                        continue;
                    }
                };

                edgeless_node::resources::observe_execution(started, &mut telemetry_handle, need_reply);
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl PortalResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(PortalResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                telemetry_handle,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, PortalResource>::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for PortalResourceProvider {
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

        match PortalResource::new(dataplane_handle, telemetry_handle).await {
            Ok(resource) => {
                lck.instances.insert(new_id, resource);
                Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
            }
            Err(err) => Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Invalid resource configuration".to_string(),
                    detail: Some(err.to_string()),
                },
            )),
        }
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        todo!("XXX")
    }
}
