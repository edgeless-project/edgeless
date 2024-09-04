// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_api::node_management::UpdatePeersRequest;
use edgeless_dataplane::core::EdgelessDataplanePeerSettings;
use edgeless_telemetry::performance_target;
use futures::{Future, SinkExt, StreamExt};

#[cfg(test)]
pub mod test;

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
    UpdatePeers(edgeless_api::node_management::UpdatePeersRequest),
    HealthStatus(futures::channel::oneshot::Sender<anyhow::Result<edgeless_api::node_management::HealthStatus>>),
}

pub struct Agent {
    sender: futures::channel::mpsc::UnboundedSender<AgentRequest>,
    node_id: uuid::Uuid,
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
        telemetry_performance_target: edgeless_telemetry::performance_target::PerformanceTargetInner,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        for class_type in runners.keys() {
            log::info!("new runner, class_type: {}", class_type);
        }

        let main_task = Box::pin(async move {
            Self::main_task(receiver, runners, resources, data_plane_provider, telemetry_performance_target).await;
        });

        (Agent { sender, node_id }, main_task)
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<AgentRequest>,
        // runners-key: class_type
        // runners-value: RuntimeBox
        mut runners: std::collections::HashMap<String, Box<dyn crate::base_runtime::RuntimeAPI + Send>>,
        resources: std::collections::HashMap<String, ResourceDesc>,
        data_plane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_performance_target: edgeless_telemetry::performance_target::PerformanceTargetInner,
    ) {
        let mut receiver = std::pin::pin!(receiver);
        let mut data_plane_provider = data_plane_provider;
        let mut telemetry_performance_target = telemetry_performance_target;

        // key: provider_id
        // value: class_type
        //        client (resource configuration API)
        let mut resource_providers = resources;
        // key: fid
        // value: provider_id
        let mut resource_instances = std::collections::HashMap::<edgeless_api::function_instance::ComponentId, String>::new();

        // After spawning a new function, the function´s class is only used to determine which runner to start it on.
        // When stopping, only the stop_function_id is provided which does not allow to know which runner it is
        // currently deployed on. Here, we implement a instance_id -> function_class HashMap
        let mut component_id_to_class_map = std::collections::HashMap::<edgeless_api::function_instance::ComponentId, String>::new();

        // Internal data structures to query system/process information.
        let mut sys = sysinfo::System::new();
        let my_pid = sysinfo::Pid::from_u32(std::process::id());

        log::info!("Starting Edgeless Agent");
        while let Some(req) = receiver.next().await {
            match req {
                AgentRequest::Spawn(spawn_req) => {
                    log::debug!("Agent Spawn {:?}", spawn_req);

                    // Save function_class for further interaction.
                    // We can assume that the Optional<instance_id> is present.
                    if spawn_req.instance_id.is_none() {
                        log::error!("No instance_id provided for SpawnFunctionRequest!");
                        continue;
                    }
                    component_id_to_class_map.insert(
                        spawn_req.instance_id.clone().unwrap().function_id,
                        spawn_req.code.function_class_type.clone(),
                    );

                    // Get runner for function_class of spawn_req
                    match runners.get_mut(&spawn_req.code.function_class_type) {
                        Some(r) => {
                            // Forward the start request to the correct runner
                            match r.start(spawn_req).await {
                                Ok(_) => {}
                                Err(err) => {
                                    log::error!("Unhandled Start Error: {}", err);
                                    continue;
                                }
                            }
                        }
                        None => {
                            log::warn!("Could not find runner for {}", spawn_req.code.function_class_type);
                            continue;
                        }
                    }
                }
                AgentRequest::Stop(stop_function_id) => {
                    log::debug!("Agent Stop {:?}", stop_function_id);

                    // Get function class by looking it up in the instanceId->functionClass map
                    let function_class: String = match component_id_to_class_map.get(&stop_function_id.function_id) {
                        Some(v) => v.clone(),
                        None => {
                            log::error!("Could not find function_class for instanceId {}", stop_function_id);
                            continue;
                        }
                    };

                    // Get runner for function_class
                    match runners.get_mut(&function_class) {
                        Some(r) => {
                            // Forward the stop request to the correct runner
                            match r.stop(stop_function_id).await {
                                Ok(_) => {
                                    // Successfully stopped - now delete the component_id -> function_class mapping
                                    component_id_to_class_map.remove(&stop_function_id.function_id);
                                    log::info!("Stopped function {} and cleared memory.", stop_function_id);
                                }
                                Err(err) => {
                                    log::error!("Unhandled Stop Error: {}", err);
                                    continue;
                                }
                            }
                        }
                        None => {
                            log::error!("Could not find runner for {}", function_class);
                            continue;
                        }
                    }
                }

                // PatchRequest contains function_id: ComponentId
                AgentRequest::Patch(update) => {
                    log::debug!("Agent UpdatePeers {:?}", update);

                    // Get function class by looking it up in the instanceId->functionClass map
                    let function_class: String = match component_id_to_class_map.get(&update.function_id) {
                        Some(v) => v.clone(),
                        None => {
                            log::error!("Could not find function_class for instanceId {}", update.function_id);
                            continue;
                        }
                    };

                    // Get runner for function_class
                    match runners.get_mut(&function_class) {
                        Some(r) => {
                            // Forward the patch request to the correct runner
                            match r.patch(update).await {
                                Ok(_) => {}
                                Err(err) => {
                                    log::error!("Unhandled Patch Error: {}", err);
                                }
                            }
                        }
                        None => {
                            log::error!("Could not find runner for {}", function_class);
                            continue;
                        }
                    }
                }
                AgentRequest::UpdatePeers(request) => {
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
                    if let Some((provider_id, resource_desc)) = resource_providers
                        .iter_mut()
                        .find(|(_provider_id, resource_desc)| resource_desc.class_type == instance_specification.class_type)
                    {
                        let res = match resource_desc.client.start(instance_specification).await {
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
                                "Started resource class_type {}, provider_id {}, node_id {}, fid {}",
                                resource_desc.class_type,
                                provider_id,
                                id.node_id,
                                id.function_id
                            );
                            resource_instances.insert(id.function_id.clone(), provider_id.clone());
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
                    if let Some(provider_id) = resource_instances.get(&resource_id.function_id) {
                        if let Some(resource_desc) = resource_providers.get_mut(provider_id) {
                            log::info!(
                                "Stopped resource class_type {}, provider_id {} node_id {}, fid {}",
                                resource_desc.class_type,
                                provider_id,
                                resource_id.node_id,
                                resource_id.function_id
                            );
                            responder
                                .send(resource_desc.client.stop(resource_id).await)
                                .unwrap_or_else(|_| log::warn!("Responder Send Error"));
                            continue;
                        } else {
                            responder
                                .send(Err(anyhow::anyhow!(
                                    "Cannot stop a resource, provider not found with provider_id: {}",
                                    provider_id
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
                        if let Some(resource_desc) = resource_providers.get_mut(provider_id) {
                            log::info!("Patch resource provider_id {} fid {}", provider_id, update.function_id);
                            responder
                                .send(resource_desc.client.patch(update).await)
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
                AgentRequest::HealthStatus(responder) => {
                    // Refresh system/process information.
                    sys.refresh_cpu();
                    sys.refresh_memory();
                    sys.refresh_process(my_pid);

                    let to_kb = |x| (x / 1024) as i32;
                    let proc = sys.process(my_pid).unwrap();
                    let health_status = edgeless_api::node_management::HealthStatus {
                        cpu_usage: sys.global_cpu_info().cpu_usage() as i32,
                        cpu_load: sys.cpus().iter().map(|x| x.cpu_usage() / 100_f32).sum::<f32>() as i32,
                        mem_free: to_kb(sys.free_memory()),
                        mem_used: to_kb(sys.used_memory()),
                        mem_total: to_kb(sys.total_memory()),
                        mem_available: to_kb(sys.available_memory()),
                        proc_cpu_usage: proc.cpu_usage() as i32,
                        proc_memory: to_kb(proc.memory()),
                        proc_vmemory: to_kb(proc.virtual_memory()),
                        function_execution_times: telemetry_performance_target.get_metrics().function_execution_times,
                    };
                    responder.send(Ok(health_status)).unwrap_or_else(|_| log::warn!("Responder Send Error"));
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::agent::AgentAPI + Send> {
        Box::new(AgentClient {
            function_instance_client: Box::new(FunctionInstanceNodeClient {
                sender: self.sender.clone(),
                node_id: self.node_id.clone(),
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
impl edgeless_api::node_management::NodeManagementAPI for NodeManagementClient {
    async fn update_peers(&mut self, request: edgeless_api::node_management::UpdatePeersRequest) -> anyhow::Result<()> {
        match self.sender.send(AgentRequest::UpdatePeers(request)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Agent channel error when updating the peers of a node: {}",
                err.to_string()
            )),
        }
    }

    async fn keep_alive(&mut self) -> anyhow::Result<edgeless_api::node_management::HealthStatus> {
        let (rsp_sender, rsp_receiver) = futures::channel::oneshot::channel::<anyhow::Result<edgeless_api::node_management::HealthStatus>>();
        let _ = self
            .sender
            .send(AgentRequest::HealthStatus(rsp_sender))
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when querying health status: {}", err.to_string()))?;
        rsp_receiver
            .await
            .map_err(|err| anyhow::anyhow!("Agent channel error when querying health status: {}", err.to_string()))?
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

    fn node_management_api(&mut self) -> Box<dyn edgeless_api::node_management::NodeManagementAPI> {
        self.node_management_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>> {
        self.resource_configuration_client.clone()
    }
}
