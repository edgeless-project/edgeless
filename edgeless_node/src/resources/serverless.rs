// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::{SinkExt, StreamExt};

pub struct ServerlessResourceProviderSpec {
    class_type: String,
    version: String,
}

impl ServerlessResourceProviderSpec {
    pub fn new(class_type: &str, version: &str) -> Self {
        Self {
            class_type: class_type.to_string(),
            version: version.to_string(),
        }
    }
}

impl super::resource_provider_specs::ResourceProviderSpecs for ServerlessResourceProviderSpec {
    fn class_type(&self) -> String {
        self.class_type.clone()
    }

    fn outputs(&self) -> Vec<String> {
        vec!["out".to_string()]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::new()
    }

    fn version(&self) -> String {
        self.version.clone()
    }
}

#[derive(Clone)]
pub struct ServerlessResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<ServerlessResourceProviderInner>>,
}

#[derive(Debug)]
struct CallCommand {
    msg: String,
    resource_id: edgeless_api::function_instance::ComponentId,
    reply_sender: tokio::sync::oneshot::Sender<anyhow::Result<(edgeless_api::function_instance::InstanceId, String)>>,
}

enum ServerlessCommand {
    Call(CallCommand),
    // resource_id, target
    Patch(edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::InstanceId),
}

pub struct ServerlessResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    instances: std::collections::HashMap<edgeless_api::function_instance::ComponentId, ServerlessResource>,
    sender: futures::channel::mpsc::UnboundedSender<ServerlessCommand>,
    _handle: tokio::task::JoinHandle<()>,
}

pub struct ServerlessResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for ServerlessResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl ServerlessResource {
    /// Create a new serverless resource.
    ///
    /// - `dataplane_provider`: handle to the EDGELESS data plane
    /// - `telemetry_hangle`: handle to the node's telemetry sub-system
    /// - `instance_id`: identifier of this resource instance
    /// - `sender`: channel to send commands to the resource task
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        instance_id: edgeless_api::function_instance::InstanceId,
        sender: futures::channel::mpsc::UnboundedSender<ServerlessCommand>,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;
        let mut sender = sender;

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id: _,
                    channel_id: _,
                    message,
                    created,
                } = dataplane_handle.receive_next().await;
                let started = crate::resources::observe_transfer(created, &mut telemetry_handle);

                // Ignore any non-cast messages.
                let msg = match message {
                    edgeless_dataplane::core::Message::Cast(data) => data,
                    _ => {
                        continue;
                    }
                };

                let (reply_sender, reply_receiver) =
                    tokio::sync::oneshot::channel::<anyhow::Result<(edgeless_api::function_instance::InstanceId, String)>>();
                let _ = sender
                    .send(ServerlessCommand::Call(CallCommand {
                        msg,
                        resource_id: instance_id.function_id,
                        reply_sender,
                    }))
                    .await;

                match reply_receiver.await {
                    Ok(response) => match response {
                        Ok((target, response)) => {
                            let _ = dataplane_handle.send(target, response).await;
                        }
                        Err(err) => {
                            log::warn!("Error from serverless resource provider: {}", err)
                        }
                    },
                    Err(err) => {
                        log::warn!("Communication error with serverless resource provider: {}", err)
                    }
                }

                crate::resources::observe_execution(started, &mut telemetry_handle, false);
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl ServerlessResourceProvider {
    /// Create a serverless resource provider:
    ///
    /// - `dataplane_provider`: handle to the EDGELESS data plane
    /// - `telemetry_hangle`: handle to the node's telemetry sub-system
    /// - `resource_provider_id`: identifier of this resource provider,
    ///    also containing the identifier of the node hosting it
    /// - `function_url`: the serverless function entry point as an HTTP URL
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
        function_url: String,
    ) -> Self {
        // Create a channel for:
        // - single receiver: the loop in the task below
        // - multiple senders: the resource instances that will be created
        //   at run-time
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let mut receiver: futures::channel::mpsc::UnboundedReceiver<ServerlessCommand> = receiver;
        let client = reqwest::Client::new();

        let _handle = tokio::spawn(async move {
            let mut targets = std::collections::HashMap::new();
            while let Some(command) = receiver.next().await {
                match command {
                    ServerlessCommand::Call(cmd) => {
                        if let Some(target) = targets.get(&cmd.resource_id) {
                            let client = client.request(reqwest::Method::POST, function_url.clone()).body(cmd.msg);
                            let response = match client.send().await {
                                Ok(ret) => {
                                    if ret.status() == reqwest::StatusCode::OK {
                                        match ret.text().await {
                                            Ok(body) => Ok((*target, body)),
                                            Err(err) => anyhow::Result::Err(anyhow::anyhow!(
                                                "error when calling serverless function at {} for resource {}: {}",
                                                function_url,
                                                cmd.resource_id,
                                                err,
                                            )),
                                        }
                                    } else {
                                        anyhow::Result::Err(anyhow::anyhow!(
                                            "error when calling serverless function at {} for resource {}: status {} returned",
                                            function_url,
                                            cmd.resource_id,
                                            ret.status()
                                        ))
                                    }
                                }
                                Err(err) => anyhow::Result::Err(anyhow::anyhow!(
                                    "error when calling serverless function at {} for resource {}: {}",
                                    function_url,
                                    cmd.resource_id,
                                    err,
                                )),
                            };

                            let _ = cmd.reply_sender.send(response);
                        }
                    }
                    ServerlessCommand::Patch(resource_id, target) => {
                        targets.insert(resource_id, target);
                    }
                };
            }
        });
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(ServerlessResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                telemetry_handle,
                instances: std::collections::HashMap::new(),
                sender,
                _handle,
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for ServerlessResourceProvider {
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

        match ServerlessResource::new(dataplane_handle, telemetry_handle, new_id, lck.sender.clone()).await {
            Ok(resource) => {
                lck.instances.insert(new_id.function_id, resource);
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

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.inner.lock().await.instances.remove(&resource_id.function_id);
        Ok(())
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // Find the target component to which we have to send events
        // generated on the "out" output channel.
        let target = match update.output_mapping.get("out") {
            Some(val) => *val,
            None => {
                anyhow::bail!("Missing mapping of channel: out");
            }
        };

        // Check that the resource to be patched is active.
        let mut lck = self.inner.lock().await;
        if !lck.instances.contains_key(&update.function_id) {
            anyhow::bail!("Patching a non-existing resource: {}", update.function_id);
        }

        // Add/update the mapping of the resource provider to the target.
        let _ = lck.sender.send(ServerlessCommand::Patch(update.function_id, target)).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_call_external_openfaas_function() -> anyhow::Result<()> {
        let client = reqwest::Client::new()
            .request(reqwest::Method::POST, "http://localhost:5000/")
            .body("3.14");

        let ret = client.send().await?;

        let s = ret.status();
        println!("status = {}", s);
        if s == reqwest::StatusCode::OK {
            println!("body = {}", ret.text().await?);
        }
        Ok(())
    }
}
