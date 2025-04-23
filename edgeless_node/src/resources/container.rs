// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::{SinkExt, StreamExt};

// XXX
// pub struct ContainerResourceSpec {}

// impl super::resource_provider_specs::ResourceProviderSpecs for ContainerResourceSpec {
//     fn class_type(&self) -> String {
//         String::from("container")
//     }

//     fn outputs(&self) -> Vec<String> {
//         vec![String::from("new_request")]
//     }

//     fn configurations(&self) -> std::collections::HashMap<String, String> {
//         std::collections::HashMap::from([(String::from("model"), String::from("Model to be used for chatting"))])
//     }

//     fn version(&self) -> String {
//         String::from("1.1")
//     }
// }

#[derive(Clone)]
pub struct ContainerResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<ContainerResourceProviderInner>>,
}

#[derive(Debug)]
struct CallCommand {
    msg: String,
    resource_id: edgeless_api::function_instance::ComponentId,
    reply_sender: tokio::sync::oneshot::Sender<anyhow::Result<(edgeless_api::function_instance::InstanceId, String)>>,
}

enum ContainerCommand {
    Call(CallCommand),
    // resource_id, target
    Patch(edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::InstanceId),
}

pub struct ContainerResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    instances: std::collections::HashMap<edgeless_api::function_instance::ComponentId, ContainerResource>,
    sender: futures::channel::mpsc::UnboundedSender<ContainerCommand>,
    _handle: tokio::task::JoinHandle<()>,
}

pub struct ContainerResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for ContainerResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl ContainerResource {
    /// Create a new container resource.
    ///
    /// - `dataplane_provider`: handle to the EDGELESS data plane
    /// - `telemetry_hangle`: handle to the node's telemetry sub-system
    /// - `instance_id`: identifier of this resource instance
    /// - `sender`: channel to send commands to the resource task
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        instance_id: edgeless_api::function_instance::InstanceId,
        sender: futures::channel::mpsc::UnboundedSender<ContainerCommand>,
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
                    .send(ContainerCommand::Call(CallCommand {
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
                            log::warn!("Error from container resource provider: {}", err)
                        }
                    },
                    Err(err) => {
                        log::warn!("Communication error with container resource provider: {}", err)
                    }
                }

                crate::resources::observe_execution(started, &mut telemetry_handle, false);
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl ContainerResourceProvider {
    /// Create a container resource provider:
    ///
    /// - `dataplane_provider`: handle to the EDGELESS data plane
    /// - `telemetry_hangle`: handle to the node's telemetry sub-system
    /// - `resource_provider_id`: identifier of this resource provider,
    ///    also containing the identifier of the node hosting it
    /// - `image_name`: name of the container image, which must be locally
    ///    available
    /// - `init_payload`: string to call the init() handle of the container
    ///    function
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
        image_name: String,
        init_payload: String,
    ) -> Self {
        // Create a channel for:
        // - single receiver: the loop in the task below
        // - multiple senders: the resource instances that will be created
        //   at run-time
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let mut receiver: futures::channel::mpsc::UnboundedReceiver<ContainerCommand> = receiver;

        // Start the container, if not already started.

        let _handle = tokio::spawn(async move {
            let mut targets = std::collections::HashMap::new();
            while let Some(command) = receiver.next().await {
                match command {
                    ContainerCommand::Call(cmd) => {
                        if let Some(target) = targets.get(&cmd.resource_id) {
                            // let result = ollama
                            //     .send_chat_messages_with_history(
                            //         ollama_rs::generation::chat::request::ChatMessageRequest::new(
                            //             cmd.model_name.clone(),
                            //             vec![ollama_rs::generation::chat::ChatMessage::user(cmd.prompt)],
                            //         ),
                            //         cmd.history_id.clone(),
                            //     )
                            //     .await;
                            // let response = match result {
                            //     Ok(res) => Ok((*target, res.message.unwrap().content)),
                            //     Err(err) => anyhow::Result::Err(anyhow::anyhow!(
                            //         "Ollama error with model {}, history_id {}: {}",
                            //         cmd.model_name,
                            //         cmd.history_id,
                            //         err
                            //     )),
                            // };
                            // let _ = cmd.reply_sender.send(response);
                        }
                    }
                    ContainerCommand::Patch(resource_id, target) => {
                        targets.insert(resource_id, target);
                    }
                };
            }
        });
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(ContainerResourceProviderInner {
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
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for ContainerResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let mut lck = self.inner.lock().await;
        let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
        let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;
        let telemetry_handle = lck.telemetry_handle.fork(std::collections::BTreeMap::from([(
            "FUNCTION_ID".to_string(),
            new_id.function_id.to_string(),
        )]));

        match ContainerResource::new(dataplane_handle, telemetry_handle, new_id, lck.sender.clone()).await {
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
        let _ = lck.sender.send(ContainerCommand::Patch(update.function_id, target)).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[tokio::test]
    async fn test_container_resource() {
        // XXX
    }
}
