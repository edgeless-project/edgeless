// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use futures::{Future, SinkExt, StreamExt};

#[cfg(test)]
pub mod test;

enum AgentRequest {
    // Function lifecycle management API.
    SpawnFunction(
        edgeless_api::function_instance::SpawnFunctionRequest,
        futures::channel::oneshot::Sender<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>>,
    ),
    StopFunction(edgeless_api::function_instance::InstanceId),
    PatchFunction(edgeless_api::common::PatchRequest),

    // Resource  lifecycle management API.
    SpawnResource(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        futures::channel::oneshot::Sender<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>>,
    ),
    StopResource(
        edgeless_api::function_instance::InstanceId,
        futures::channel::oneshot::Sender<anyhow::Result<()>>,
    ),
    PatchResource(edgeless_api::common::PatchRequest, futures::channel::oneshot::Sender<anyhow::Result<()>>),

    // Node management API.
    UpdatePeers(edgeless_api::node_management::UpdatePeersRequest),
    Reset(),
}

pub struct Agent {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}

pub struct ResourceDesc {
    pub class_type: String,
    pub client: Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
}

impl Agent {
    pub fn new(
        runners: std::collections::HashMap<String, Box<dyn crate::base_runtime::RuntimeAPI + Send>>,
        resources: std::collections::HashMap<String, ResourceDesc>,
        node_id: uuid::Uuid,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        for class_type in runners.keys() {
            log::info!("new runner, class_type: {}", class_type);
        }

        let main_task = Box::pin(async move {
            Self::main_task(node_id, receiver, runners, resources, data_plane_provider).await;
        });

        (Agent { sender }, main_task)
    }

    async fn main_task(
        node_id: uuid::Uuid,
        receiver: futures::channel::mpsc::UnboundedReceiver<AgentRequest>,
        function_runtimes: std::collections::HashMap<String, Box<dyn crate::base_runtime::RuntimeAPI + Send>>,
        resources: std::collections::HashMap<String, ResourceDesc>,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    ) {
        let mut receiver = std::pin::pin!(receiver);
        let mut data_plane_provider = data_plane_provider;

        // key:   function class
        // value: function run-time API
        let mut function_runtimes = function_runtimes;

        // key: provider_id
        // value: class_type
        //        resource configuration API
        let mut resource_providers = resources;

        // Active function instances.
        // key:   physical function identifier
        // value: function class
        let mut function_instances = std::collections::HashMap::<edgeless_api::function_instance::ComponentId, String>::new();

        // Active resource instances.
        // key:   physical resource identifier
        // value: provider_id
        let mut resource_instances = std::collections::HashMap::<edgeless_api::function_instance::ComponentId, String>::new();

        log::info!("Starting EDGELESS node agent");
        while let Some(req) = receiver.next().await {
            match req {
                AgentRequest::SpawnFunction(spawn_req, responder) => {
                    log::debug!("Agent SpawnFunction {:?}", spawn_req.code.to_short_string());

                    // Get runner for function_class of spawn_req
                    let res = match function_runtimes.get_mut(&spawn_req.code.function_class_type) {
                        Some(runner) => {
                            // Assign a new physical identifier to the function instance being created.
                            let instance_id = edgeless_api::function_instance::InstanceId::new(node_id);

                            // Save function_class for further interaction.
                            function_instances.insert(instance_id.function_id, spawn_req.code.function_class_type.clone());

                            // Forward the start request to the matching runtime.
                            match runner.start(instance_id, spawn_req).await {
                                Ok(_) => edgeless_api::common::StartComponentResponse::InstanceId(instance_id),
                                Err(err) => edgeless_api::common::StartComponentResponse::ResponseError(edgeless_api::common::ResponseError {
                                    summary: "Could not start function".to_string(),
                                    detail: Some(format!("{}", err)),
                                }),
                            }
                        }
                        None => edgeless_api::common::StartComponentResponse::ResponseError(edgeless_api::common::ResponseError {
                            summary: "Error when creating a function instance".to_string(),
                            detail: Some(format!("Could not find runner for {}", spawn_req.code.function_class_type)),
                        }),
                    };
                    responder
                        .send(res)
                        .unwrap_or_else(|_| log::warn!("Agent SpawnFunction: responder send error"));
                }
                AgentRequest::StopFunction(stop_function_id) => {
                    log::debug!("Agent StopFunction {:?}", stop_function_id);

                    Self::stop_function(&mut function_runtimes, &mut function_instances, stop_function_id).await;
                }

                // PatchRequest contains function_id: ComponentId
                AgentRequest::PatchFunction(update) => {
                    log::debug!("Agent PatchFunction {:?}", update);

                    // Get function class by looking it up in the instanceId->functionClass map
                    // and then orward the patch request to the correct runner.
                    match function_instances.get(&update.function_id) {
                        Some(function_class) => match function_runtimes.get_mut(function_class) {
                            Some(runner) => {
                                if let Err(err) = runner.patch(update).await {
                                    log::error!("Unhandled Patch Error: {}", err);
                                }
                            }
                            None => {
                                log::error!("Could not find runner for function class '{}' when patching", function_class);
                            }
                        },
                        None => {
                            log::error!("Could not find function class for instanceId '{}' patching", update.function_id);
                        }
                    };
                }
                AgentRequest::UpdatePeers(request) => {
                    log::debug!("Agent UpdatePeers {:?}", request);
                    match request {
                        edgeless_api::node_management::UpdatePeersRequest::Add(node_id, invocation_url) => {
                            data_plane_provider
                                .add_peer(edgeless_dataplane::core::EdgelessDataplanePeerSettings { node_id, invocation_url })
                                .await
                        }
                        edgeless_api::node_management::UpdatePeersRequest::Del(node_id) => data_plane_provider.del_peer(node_id).await,
                        edgeless_api::node_management::UpdatePeersRequest::Clear => panic!("UpdatePeersRequest::Clear not implemented"),
                    };
                }
                AgentRequest::SpawnResource(instance_specification, responder) => {
                    log::debug!("Agent SpawnResource {:?}", instance_specification);

                    let res = if let Some((provider_id, resource_desc)) = resource_providers
                        .iter_mut()
                        .find(|(_provider_id, resource_desc)| resource_desc.class_type == instance_specification.class_type)
                    {
                        match resource_desc.client.start(instance_specification).await {
                            Ok(val) => match val {
                                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                                    log::info!(
                                        "Started resource class_type {}, provider_id {}, node_id {}, fid {}",
                                        resource_desc.class_type,
                                        provider_id,
                                        id.node_id,
                                        id.function_id
                                    );
                                    resource_instances.insert(id.function_id, provider_id.clone());
                                    edgeless_api::common::StartComponentResponse::InstanceId(id)
                                }
                                edgeless_api::common::StartComponentResponse::ResponseError(err) => {
                                    edgeless_api::common::StartComponentResponse::ResponseError(err)
                                }
                            },
                            Err(err) => edgeless_api::common::StartComponentResponse::ResponseError(edgeless_api::common::ResponseError {
                                summary: "Error when creating a resource".to_string(),
                                detail: Some(format!("{}", err)),
                            }),
                        }
                    } else {
                        edgeless_api::common::StartComponentResponse::ResponseError(edgeless_api::common::ResponseError {
                            summary: "Error when creating a resource".to_string(),
                            detail: Some(format!("Provider for class_type does not exist: {}", instance_specification.class_type)),
                        })
                    };
                    responder
                        .send(res)
                        .unwrap_or_else(|_| log::warn!("Agent SpawnResource: responder send error"));
                }
                AgentRequest::StopResource(resource_id, responder) => {
                    log::debug!("Agent StopResource {:?}", resource_id);

                    responder
                        .send(Self::stop_resource(&mut resource_providers, &mut resource_instances, resource_id).await)
                        .unwrap_or_else(|_| log::warn!("Agent StopResource: responder send error"));
                }
                AgentRequest::PatchResource(update, responder) => {
                    log::debug!("Agent PatchResource {:?}", update);

                    let res = if let Some(provider_id) = resource_instances.get(&update.function_id) {
                        if let Some(resource_desc) = resource_providers.get_mut(provider_id) {
                            resource_desc.client.patch(update).await
                        } else {
                            Err(anyhow::anyhow!(
                                "Cannot patch a resource, provider not found with provider_id: {}",
                                provider_id
                            ))
                        }
                    } else {
                        Err(anyhow::anyhow!("Cannot patch a resource, not found with fid: {}", update.function_id))
                    };
                    responder
                        .send(res)
                        .unwrap_or_else(|_| log::warn!("Agent PatchResource: responder send error"));
                }
                AgentRequest::Reset() => {
                    log::info!("Resetting the node to a clean state");

                    // Stop all the function instances.
                    let function_ids = function_instances
                        .keys()
                        .cloned()
                        .collect::<Vec<edgeless_api::function_instance::ComponentId>>();
                    for function_id in function_ids {
                        Self::stop_function(
                            &mut function_runtimes,
                            &mut function_instances,
                            edgeless_api::function_instance::InstanceId { node_id, function_id },
                        )
                        .await;
                    }
                    function_instances.clear();

                    // Stop all the resource instances.
                    let resource_ids = resource_instances
                        .keys()
                        .cloned()
                        .collect::<Vec<edgeless_api::function_instance::ComponentId>>();
                    for resource_id in resource_ids {
                        if let Err(err) = Self::stop_resource(
                            &mut resource_providers,
                            &mut resource_instances,
                            edgeless_api::function_instance::InstanceId {
                                node_id,
                                function_id: resource_id,
                            },
                        )
                        .await
                        {
                            log::warn!("Error stopping the resource with ID '{}': {}", resource_id, err);
                        }
                    }
                    resource_instances.clear();
                }
            }
        }
    }

    async fn stop_function(
        function_runtimes: &mut std::collections::HashMap<std::string::String, Box<dyn crate::base_runtime::RuntimeAPI + std::marker::Send>>,
        function_instances: &mut std::collections::HashMap<edgeless_api::function_instance::ComponentId, String>,
        function_id: edgeless_api::function_instance::InstanceId,
    ) {
        // Get function class by looking it up in the instanceId->functionClass map
        let function_class: String = match function_instances.get(&function_id.function_id) {
            Some(v) => v.clone(),
            None => {
                log::error!("Could not find function_class for instanceId {}", function_id);
                return;
            }
        };

        // Get runner for function_class
        match function_runtimes.get_mut(&function_class) {
            Some(runner) => {
                // Forward the stop request to the correct runner
                match runner.stop(function_id).await {
                    Ok(_) => {
                        // Successfully stopped - now delete the component_id -> function_class mapping
                        function_instances.remove(&function_id.function_id);
                        log::info!("Stopped function {} and cleared memory.", function_id);
                    }
                    Err(err) => {
                        log::error!("Unhandled Stop Error: {}", err);
                    }
                }
            }
            None => {
                log::error!("Could not find runner for {}", function_class);
            }
        }
    }

    async fn stop_resource(
        resource_providers: &mut std::collections::HashMap<String, ResourceDesc>,
        resource_instances: &mut std::collections::HashMap<edgeless_api::function_instance::ComponentId, String>,
        resource_id: edgeless_api::function_instance::InstanceId,
    ) -> anyhow::Result<()> {
        if let Some(provider_id) = resource_instances.get(&resource_id.function_id) {
            if let Some(resource_desc) = resource_providers.get_mut(provider_id) {
                log::info!(
                    "Stopped resource class_type {}, provider_id {} node_id {}, fid {}",
                    resource_desc.class_type,
                    provider_id,
                    resource_id.node_id,
                    resource_id.function_id
                );
                resource_desc.client.stop(resource_id).await
            } else {
                anyhow::bail!("Cannot stop a resource, provider not found with provider_id: {}", provider_id);
            }
        } else {
            anyhow::bail!("Cannot stop a resource, not found with fid: {}", resource_id.function_id);
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::outer::agent::AgentAPI + Send> {
        Box::new(AgentClient {
            function_instance_client: Box::new(FunctionInstanceNodeClient { sender: self.sender.clone() }),
            node_management_client: Box::new(NodeManagementClient { sender: self.sender.clone() }),
            resource_configuration_client: Box::new(ResourceConfigurationClient { sender: self.sender.clone() }),
        })
    }
}

#[derive(Clone)]
pub struct FunctionInstanceNodeClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}

#[derive(Clone)]
pub struct NodeManagementClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}

#[derive(Clone)]
pub struct ResourceConfigurationClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
}

#[derive(Clone)]
pub struct AgentClient {
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>>,
    node_management_client: Box<dyn edgeless_api::node_management::NodeManagementAPI>,
    resource_configuration_client:
        Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId> for FunctionInstanceNodeClient {
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let (rsp_sender, rsp_receiver) =
            futures::channel::oneshot::channel::<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>>();
        let _ = self
            .sender
            .send(AgentRequest::SpawnFunction(request, rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a function instance: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a function instance: {}", err.to_string()))
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.sender
            .send(AgentRequest::StopFunction(id))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when stopping a function instance: {}", err.to_string()))
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        self.sender
            .send(AgentRequest::PatchFunction(update))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when patching a function instance: {}", err.to_string()))
    }
}

#[async_trait::async_trait]
impl edgeless_api::node_management::NodeManagementAPI for NodeManagementClient {
    async fn update_peers(&mut self, request: edgeless_api::node_management::UpdatePeersRequest) -> anyhow::Result<()> {
        self.sender
            .send(AgentRequest::UpdatePeers(request))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when updating a node's peers: {}", err.to_string()))
    }
    async fn reset(&mut self) -> anyhow::Result<()> {
        self.sender
            .send(AgentRequest::Reset())
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when resetting a node: {}", err.to_string()))
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for ResourceConfigurationClient {
    async fn start(
        &mut self,
        request: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let (rsp_sender, rsp_receiver) =
            futures::channel::oneshot::channel::<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>>();
        let _ = self
            .sender
            .send(AgentRequest::SpawnResource(request, rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        let (rsp_sender, rsp_receiver) = futures::channel::oneshot::channel::<anyhow::Result<()>>();
        self.sender
            .send(AgentRequest::StopResource(id, rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        let (rsp_sender, rsp_receiver) = futures::channel::oneshot::channel::<anyhow::Result<()>>();
        self.sender
            .send(AgentRequest::PatchResource(update, rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when patching a resource instance: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when patching a resource instance: {}", err.to_string()))?
    }
}

impl edgeless_api::outer::agent::AgentAPI for AgentClient {
    fn function_instance_api(
        &mut self,
    ) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>> {
        self.function_instance_client.clone()
    }

    fn node_management_api(&mut self) -> Box<dyn edgeless_api::node_management::NodeManagementAPI> {
        self.node_management_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>> {
        self.resource_configuration_client.clone()
    }
}
