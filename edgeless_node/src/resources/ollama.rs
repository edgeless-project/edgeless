// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::{SinkExt, StreamExt};

pub struct OllamaResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for OllamaResourceSpec {
    fn class_type(&self) -> String {
        String::from("ollama")
    }

    fn description(&self) -> String {
        r"Interact via an LLM ChatBot deployed on an external Ollama server -- see https://ollama.com/".to_string()
    }

    fn outputs(&self) -> Vec<String> {
        vec![String::from("out")]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([(String::from("model"), String::from("Model to be used for chatting"))])
    }

    fn version(&self) -> String {
        String::from("1.1")
    }
}

#[derive(Clone)]
pub struct OllamaResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<OllamaResourceProviderInner>>,
}

struct ChatCommand {
    model_name: String,
    history_id: String,
    prompt: String,
    resource_id: edgeless_api::function_instance::ComponentId,
    reply_sender: tokio::sync::oneshot::Sender<anyhow::Result<(edgeless_api::function_instance::InstanceId, String)>>,
}

enum OllamaCommand {
    Chat(ChatCommand),
    // resource_id, target
    Patch(edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::InstanceId),
}

impl std::fmt::Display for OllamaCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OllamaCommand::Chat(cmd) => write!(
                f,
                "model {}, history_id {}, prompt length {}, resource_id {})",
                cmd.model_name,
                cmd.history_id,
                cmd.prompt.len(),
                cmd.resource_id
            ),
            OllamaCommand::Patch(resource_id, target) => write!(f, "resource_id {}, target {}", resource_id, target),
        }
    }
}

pub struct OllamaResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    instances: std::collections::HashMap<edgeless_api::function_instance::ComponentId, OllamaResource>,
    sender: futures::channel::mpsc::UnboundedSender<OllamaCommand>,
    _handle: tokio::task::JoinHandle<()>,
}

pub struct OllamaResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for OllamaResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl OllamaResource {
    /// Create a new Ollama resource.
    ///
    /// - `dataplane_provider`: handle to the EDGELESS data plane
    /// - `telemetry_hangle`: handle to the node's telemetry sub-system
    /// - `model_name`: name of the AI model to use
    /// - `instance_id`: identifier of this resource instance
    /// - `sender`: channel to send commands to the resource task
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        model_name: String,
        instance_id: edgeless_api::function_instance::InstanceId,
        sender: futures::channel::mpsc::UnboundedSender<OllamaCommand>,
    ) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let mut telemetry_handle = telemetry_handle;
        let mut sender = sender;

        let history_id = instance_id.function_id.to_string();
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
                let prompt = match message {
                    edgeless_dataplane::core::Message::Cast(data) => data,
                    _ => {
                        continue;
                    }
                };

                let (reply_sender, reply_receiver) =
                    tokio::sync::oneshot::channel::<anyhow::Result<(edgeless_api::function_instance::InstanceId, String)>>();
                let _ = sender
                    .send(OllamaCommand::Chat(ChatCommand {
                        model_name: model_name.clone(),
                        history_id: history_id.clone(),
                        prompt,
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
                            log::warn!("Error from ollama: {}", err)
                        }
                    },
                    Err(err) => {
                        log::warn!("Communication error with ollama resource provider: {}", err)
                    }
                }

                crate::resources::observe_execution(started, &mut telemetry_handle, false);
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl OllamaResourceProvider {
    /// Create an Ollama resource provider:
    ///
    /// - `dataplane_provider`: handle to the EDGELESS data plane
    /// - `telemetry_hangle`: handle to the node's telemetry sub-system
    /// - `resource_provider_id`: identifier of this resource provider,
    ///    also containing the identifier of the node hosting it
    /// - `ollama_host`: address of the ollama server
    /// - `ollama_port`: port number of the ollama server
    /// - `ollama_messages_number_limit`: maximum number of messages per
    ///    chat conversation
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
        ollama_host: &str,
        ollama_port: u16,
        ollama_messages_number_limit: u16,
    ) -> Self {
        // Create a channel for:
        // - single receiver: the loop in the task below
        // - multiple senders: the resource instances that will be created
        //   at run-time
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let mut receiver: futures::channel::mpsc::UnboundedReceiver<OllamaCommand> = receiver;

        // Create a new instance of the ollama connector.
        let mut ollama = ollama_rs::Ollama::new_with_history(format!("http://{}", ollama_host), ollama_port, ollama_messages_number_limit);

        let _handle = tokio::spawn(async move {
            let mut targets = std::collections::HashMap::new();
            while let Some(command) = receiver.next().await {
                match command {
                    OllamaCommand::Chat(cmd) => {
                        if let Some(target) = targets.get(&cmd.resource_id) {
                            let result = ollama
                                .send_chat_messages_with_history(
                                    ollama_rs::generation::chat::request::ChatMessageRequest::new(
                                        cmd.model_name.clone(),
                                        vec![ollama_rs::generation::chat::ChatMessage::user(cmd.prompt)],
                                    ),
                                    cmd.history_id.clone(),
                                )
                                .await;
                            let response = match result {
                                Ok(res) => Ok((*target, res.message.unwrap().content)),
                                Err(err) => anyhow::Result::Err(anyhow::anyhow!(
                                    "Ollama error with model {}, history_id {}: {}",
                                    cmd.model_name,
                                    cmd.history_id,
                                    err
                                )),
                            };
                            let _ = cmd.reply_sender.send(response);
                        }
                    }
                    OllamaCommand::Patch(resource_id, target) => {
                        targets.insert(resource_id, target);
                    }
                };
            }
        });
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(OllamaResourceProviderInner {
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
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for OllamaResourceProvider {
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

        // Read configuration
        let model = match instance_specification.configuration.get("model") {
            Some(model) => model,
            None => {
                return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                    edgeless_api::common::ResponseError {
                        summary: "Invalid resource configuration".to_string(),
                        detail: Some("Missing model name".to_string()),
                    },
                ))
            }
        };

        match OllamaResource::new(dataplane_handle, telemetry_handle, model.to_string(), new_id, lck.sender.clone()).await {
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
        let _ = lck.sender.send(OllamaCommand::Patch(update.function_id, target)).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[ignore]
    #[tokio::test]
    async fn test_ollama_tutorial() {
        let address = std::env::var("OLLAMA_ADDRESS").unwrap_or("localhost".to_string());
        let port = std::env::var("OLLAMA_PORT")
            .unwrap_or("11434".to_string())
            .parse::<u16>()
            .unwrap_or(11434);
        let mut ollama = ollama_rs::Ollama::new_with_history(format!("http://{}", address), port, 30);

        let options = ollama_rs::generation::options::GenerationOptions::default()
            .temperature(0.2)
            .repeat_penalty(1.5)
            .top_k(25)
            .top_p(0.25);

        let prompts = vec!["Who are you?", "What did I just ask?"];

        if let Ok(models) = ollama.list_local_models().await {
            for model in models {
                // Without chat history.
                for prompt in &prompts {
                    let res = ollama
                        .generate(
                            ollama_rs::generation::completion::request::GenerationRequest::new(model.name.clone(), prompt.to_string())
                                .options(options.clone()),
                        )
                        .await;
                    if let Ok(res) = res {
                        println!("\nmodel:\t\t{:?}\nprompt:\t\t{}\nresponse:\t{:?}\n", model, prompt, res);
                    }
                }

                // With chat history.
                let history_id = uuid::Uuid::new_v4().to_string();
                for prompt in &prompts {
                    let res = ollama
                        .send_chat_messages_with_history(
                            ollama_rs::generation::chat::request::ChatMessageRequest::new(
                                model.name.clone(),
                                vec![ollama_rs::generation::chat::ChatMessage::user(prompt.to_string())],
                            ),
                            history_id.clone(),
                        )
                        .await;
                    if let Ok(res) = res {
                        println!(
                            "model:\t\t{:?}\nhistory_id:\t{}\nprompt:\t\t{}\nresponse:\t{:?}\n",
                            model, history_id, prompt, res
                        );
                    }
                }

                // Get full history.
                let history = ollama.get_messages_history(history_id.clone());
                println!("History for {}", history_id);
                if let Some(history) = history {
                    for msg in history {
                        println!("\t {}", msg.content);
                    }
                }
            }
        } else {
            println!("The test cannot be run");
            return;
        }
    }
}
