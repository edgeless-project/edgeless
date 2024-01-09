use edgeless_api::node_managment::UpdatePeersRequest;
use edgeless_dataplane::core::EdgelessDataplanePeerSettings;
use futures::{Future, SinkExt, StreamExt};

enum AgentRequest {
    Spawn(edgeless_api::function_instance::SpawnFunctionRequest),
    SpawnResource(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        futures::channel::oneshot::Sender<anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>>>,
    ),
    Stop(edgeless_api::function_instance::InstanceId),
    StopResource(
        edgeless_api::function_instance::InstanceId,
        futures::channel::oneshot::Sender<anyhow::Result<()>>,
    ),
    Patch(edgeless_api::common::PatchRequest),
    PatchResource(edgeless_api::common::PatchRequest, futures::channel::oneshot::Sender<anyhow::Result<()>>),
    UPDATEPEERS(edgeless_api::node_managment::UpdatePeersRequest),
}

pub struct Agent {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
    node_settings: crate::EdgelessNodeSettings,
}

impl Agent {
    pub fn new(
        runner: Box<dyn crate::base_runtime::RuntimeAPI + Send>,
        resources: std::collections::HashMap<
            String,
            Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
        >,
        node_settings: crate::EdgelessNodeSettings,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            Self::main_task(receiver, runner, resources, data_plane_provider).await;
        });

        (Agent { sender, node_settings }, main_task)
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<AgentRequest>,
        runner: Box<dyn crate::base_runtime::RuntimeAPI + Send>,
        resources: std::collections::HashMap<
            String,
            Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
        >,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
    ) {
        let mut receiver = std::pin::pin!(receiver);
        let mut runner = runner;
        let mut data_plane_provider = data_plane_provider;

        // key: class_type
        // value: resource configuration API
        let mut resource_providers = resources;
        // key: fid
        // value: provider_id
        let mut resource_instances = std::collections::HashMap::<edgeless_api::function_instance::ComponentId, String>::new();

        log::info!("Starting Edgeless Agent");
        while let Some(req) = receiver.next().await {
            match req {
                AgentRequest::Spawn(spawn_req) => {
                    log::debug!("Agent Spawn {:?}", spawn_req);
                    match runner.start(spawn_req).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled Start Error: {}", err);
                        }
                    }
                }
                AgentRequest::Stop(stop_function_id) => {
                    log::debug!("Agent Stop {:?}", stop_function_id);
                    match runner.stop(stop_function_id).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled Stop Error: {}", err);
                        }
                    }
                }
                AgentRequest::Patch(update) => {
                    log::debug!("Agent UpdatePeers {:?}", update);
                    match runner.patch(update).await {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled Patch Error: {}", err);
                        }
                    }
                }
                AgentRequest::UPDATEPEERS(request) => {
                    log::debug!("Agent UpdatePeers {:?}", request);
                    match request {
                        UpdatePeersRequest::Add(node_id, invocation_url) => {
                            data_plane_provider
                                .add_peer(EdgelessDataplanePeerSettings { node_id, invocation_url })
                                .await
                        }
                        UpdatePeersRequest::Del(node_id) => data_plane_provider.del_peer(node_id).await,
                        UpdatePeersRequest::Clear => panic!("UpdatePeersRequest::Clear not implemented"),
                    };
                }
                AgentRequest::SpawnResource(instance_specification, responder) => {
                    if let Some(resource) = resource_providers.get_mut(&instance_specification.class_type) {
                        let resource_class = instance_specification.class_type.clone();
                        let res = match resource.start(instance_specification).await {
                            Ok(val) => val,
                            Err(err) => {
                                responder
                                    .send(Err(anyhow::anyhow!("Internal Resource Error {}", err)))
                                    .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                                continue;
                            }
                        };
                        if let edgeless_api::common::StartComponentResponse::InstanceId(id) = res {
                            log::info!(
                                "Started resource class_type {}, node_id {}, fid {}",
                                resource_class,
                                id.node_id,
                                id.function_id
                            );
                            resource_instances.insert(id.function_id.clone(), resource_class.clone());
                            responder
                                .send(Ok(edgeless_api::common::StartComponentResponse::InstanceId(id)))
                                .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                        } else {
                            responder.send(Ok(res)).unwrap_or_else(|_| log::warn!("Responder Send Error"));
                        }
                    } else {
                        responder
                            .send(Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                                edgeless_api::common::ResponseError {
                                    summary: "Error when creating a resource".to_string(),
                                    detail: Some(format!("Provider for class_type does not exist: {}", instance_specification.class_type)),
                                },
                            )))
                            .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                    }
                }
                AgentRequest::StopResource(resource_id, responder) => {
                    if let Some(resource_class) = resource_instances.get(&resource_id.function_id) {
                        if let Some(provider) = resource_providers.get_mut(resource_class) {
                            log::info!(
                                "Stopped resource provider_id {} node_id {}, fid {}",
                                resource_class,
                                resource_id.node_id,
                                resource_id.function_id
                            );
                            responder
                                .send(provider.stop(resource_id).await)
                                .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                            continue;
                        } else {
                            responder
                                .send(Err(anyhow::anyhow!(
                                    "Cannot stop a resource, provider not found with provider_id: {}",
                                    resource_class
                                )))
                                .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                            continue;
                        }
                    }
                    responder
                        .send(Err(anyhow::anyhow!(
                            "Cannot stop a resource, not found with fid: {}",
                            resource_id.function_id
                        )))
                        .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                }
                AgentRequest::PatchResource(update, responder) => {
                    if let Some(provider_id) = resource_instances.get(&update.function_id) {
                        if let Some(provider) = resource_providers.get_mut(provider_id) {
                            log::info!("Patch resource provider_id {} fid {}", provider_id, update.function_id);
                            responder
                                .send(provider.patch(update).await)
                                .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                            continue;
                        } else {
                            responder
                                .send(Err(anyhow::anyhow!(
                                    "Cannot patch a resource, provider not found with provider_id: {}",
                                    provider_id
                                )))
                                .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                            continue;
                        }
                    }
                    responder
                        .send(Err(anyhow::anyhow!(
                            "Cannot patch a resource, not found with fid: {}",
                            update.function_id
                        )))
                        .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::agent::AgentAPI + Send> {
        Box::new(AgentClient {
            function_instance_client: Box::new(FunctionInstanceNodeClient {
                sender: self.sender.clone(),
                node_id: self.node_settings.node_id.clone(),
            }),
            node_management_client: Box::new(NodeManagementClient { sender: self.sender.clone() }),
            resource_configuration_client: Box::new(ResourceConfigurationClient { sender: self.sender.clone() }),
        })
    }
}

#[derive(Clone)]
pub struct FunctionInstanceNodeClient {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
    node_id: uuid::Uuid,
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
    node_management_client: Box<dyn edgeless_api::node_managment::NodeManagementAPI>,
    resource_configuration_client:
        Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId> for FunctionInstanceNodeClient {
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let mut request = request;
        let f_id = match request.instance_id.clone() {
            Some(id) => id,
            None => {
                let new_id = edgeless_api::function_instance::InstanceId::new(self.node_id);
                request.instance_id = Some(new_id.clone());
                new_id
            }
        };
        match self.sender.send(AgentRequest::Spawn(request)).await {
            Ok(_) => Ok(edgeless_api::common::StartComponentResponse::InstanceId(f_id)),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when creating a function instance: {}",
                err.to_string()
            )),
        }
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::Stop(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::Patch(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::node_managment::NodeManagementAPI for NodeManagementClient {
    async fn update_peers(&mut self, request: edgeless_api::node_managment::UpdatePeersRequest) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::UPDATEPEERS(request)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when updating the peers of a node: {}",
                err.to_string()
            )),
        }
    }

    async fn keep_alive(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for ResourceConfigurationClient {
    async fn start(
        &mut self,
        request: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let (rsp_sender, rsp_receiver) = futures::channel::oneshot::channel::<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>>,
        >();
        let _ = self
            .sender
            .send(AgentRequest::SpawnResource(request, rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?
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
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when creating a resource instance: {}", err.to_string()))?
    }
}

impl edgeless_api::agent::AgentAPI for AgentClient {
    fn function_instance_api(
        &mut self,
    ) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>> {
        self.function_instance_client.clone()
    }

    fn node_management_api(&mut self) -> Box<dyn edgeless_api::node_managment::NodeManagementAPI> {
        self.node_management_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>> {
        self.resource_configuration_client.clone()
    }
}
