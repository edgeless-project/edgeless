// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use futures::{SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::SeedableRng;

enum Pid {
    // 0: node_id, pid
    Function(edgeless_api::function_instance::InstanceId),
    // 0: node_id, pid
    Resource(edgeless_api::function_instance::InstanceId),
}

impl Pid {
    fn instance_id(&self) -> edgeless_api::function_instance::InstanceId {
        match self {
            Self::Function(id) => *id,
            Self::Resource(id) => *id,
        }
    }
}

pub(crate) struct OrchestratorTask {
    receiver: futures::channel::mpsc::UnboundedReceiver<crate::orchestrator::OrchestratorRequest>,
    nodes: std::collections::HashMap<uuid::Uuid, crate::client_desc::ClientDesc>,
    // known resources providers as advertised by the nodes upon registration
    // key: provider_id
    resource_providers: std::collections::HashMap<String, crate::resource_provider::ResourceProvider>,
    proxy: std::sync::Arc<tokio::sync::Mutex<dyn super::proxy::Proxy>>,
    subscriber_sender: futures::channel::mpsc::UnboundedSender<super::domain_subscriber::DomainSubscriberRequest>,
    orchestration_logic: crate::orchestration_logic::OrchestrationLogic,
    rng: rand::rngs::StdRng,
    // instances that the orchestrator promises to keep active
    // key: lid
    active_instances: std::collections::HashMap<uuid::Uuid, crate::active_instance::ActiveInstance>,
    active_instances_changed: bool,
    // active patches to which the orchestrator commits
    // key:   lid (origin function)
    // value: map of:
    //        key:   channel output name
    //        value: lid (target function)
    dependency_graph: std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>,
    dependency_graph_changed: bool,
    last_domain_capabilities: edgeless_api::domain_registration::DomainCapabilities,
}

impl OrchestratorTask {
    pub async fn new(
        receiver: futures::channel::mpsc::UnboundedReceiver<crate::orchestrator::OrchestratorRequest>,
        orchestrator_settings: crate::EdgelessOrcBaselineSettings,
        nodes: std::collections::HashMap<uuid::Uuid, crate::client_desc::ClientDesc>,
        resource_providers: std::collections::HashMap<String, crate::resource_provider::ResourceProvider>,
        proxy: std::sync::Arc<tokio::sync::Mutex<dyn super::proxy::Proxy>>,
        subscriber_sender: futures::channel::mpsc::UnboundedSender<super::domain_subscriber::DomainSubscriberRequest>,
    ) -> Self {
        for (node_id, client_desc) in &nodes {
            log::info!(
                "New node: node_id {}, agent URL {}, invocation URL {}",
                node_id,
                client_desc.agent_url,
                client_desc.invocation_url
            );
        }

        for (provider, resource_provider) in &resource_providers {
            log::info!("New resource: provider {}, {}", provider, resource_provider);
        }

        Self {
            receiver,
            nodes,
            resource_providers,
            proxy,
            subscriber_sender,
            orchestration_logic: crate::orchestration_logic::OrchestrationLogic::new(orchestrator_settings.orchestration_strategy),
            rng: rand::rngs::StdRng::from_entropy(),
            active_instances: std::collections::HashMap::new(),
            active_instances_changed: false,
            dependency_graph: std::collections::HashMap::new(),
            dependency_graph_changed: false,
            last_domain_capabilities: edgeless_api::domain_registration::DomainCapabilities::default(),
        }
    }

    // Main orchestration loop.
    pub async fn run(&mut self) {
        self.update_domain().await;
        while let Some(req) = self.receiver.next().await {
            match req {
                crate::orchestrator::OrchestratorRequest::StartFunction(spawn_req, reply_channel) => {
                    log::debug!("Orchestrator StartFunction {}", spawn_req.code.to_short_string());
                    let res = self.start_function(&spawn_req).await;
                    if let Err(err) = reply_channel.send(res) {
                        log::error!("Orchestrator channel error in SPAWN: {:?}", err);
                    }
                }
                crate::orchestrator::OrchestratorRequest::StopFunction(lid) => {
                    log::debug!("Orchestrator StopFunction {:?}", lid);
                    self.stop_function_lid(lid).await;
                }
                crate::orchestrator::OrchestratorRequest::StartResource(start_req, reply_channel) => {
                    log::debug!("Orchestrator StartResource {:?}", &start_req);
                    let res = self.start_resource(start_req.clone(), uuid::Uuid::new_v4()).await;
                    if let Err(err) = reply_channel.send(res) {
                        log::error!("Orchestrator channel error in STARTRESOURCE: {:?}", err);
                    }
                }
                crate::orchestrator::OrchestratorRequest::StopResource(lid) => {
                    log::debug!("Orchestrator StopResource {:?}", lid);
                    self.stop_resource_lid(lid).await;
                }
                crate::orchestrator::OrchestratorRequest::Patch(update) => {
                    log::debug!("Orchestrator Patch {:?}", update);
                    self.patch(update).await;
                }
                crate::orchestrator::OrchestratorRequest::AddNode(node_id, client_desc, resource_providers) => {
                    log::debug!("Orchestrator AddNode {}", client_desc.to_string_short());
                    self.add_node(node_id, client_desc, resource_providers).await;
                    self.update_domain().await;
                }
                crate::orchestrator::OrchestratorRequest::DelNode(node_id) => {
                    log::debug!("Orchestrator DelNode {:?}", node_id);
                    self.del_node(node_id).await;
                    self.update_domain().await;
                }
                crate::orchestrator::OrchestratorRequest::Refresh() => {
                    self.refresh().await;
                }
            }
        }
    }

    fn lid_to_pid(&self, lid: &edgeless_api::function_instance::ComponentId) -> Vec<Pid> {
        match self.active_instances.get(lid) {
            Some(active_instance) => match active_instance {
                crate::active_instance::ActiveInstance::Function(_req, instances) => instances
                    .iter()
                    .map(|x| {
                        Pid::Function(edgeless_api::function_instance::InstanceId {
                            node_id: x.node_id,
                            function_id: x.function_id,
                        })
                    })
                    .collect(),
                crate::active_instance::ActiveInstance::Resource(_req, instance) => {
                    vec![Pid::Resource(edgeless_api::function_instance::InstanceId {
                        node_id: instance.node_id,
                        function_id: instance.function_id,
                    })]
                }
            },
            None => vec![],
        }
    }

    /// Deploy an instance to a new set of targets, if possible. No repatching.
    ///
    /// * `lid` - The LID of the function/resource to be migrated.
    /// * `targets` - The set of nodes to which the instance has to be migrated.
    async fn migrate(&mut self, lid: &edgeless_api::function_instance::ComponentId, targets: &Vec<edgeless_api::function_instance::NodeId>) {
        let spawn_req = match self.active_instances.get(lid) {
            Some(active_instance) => match active_instance {
                crate::active_instance::ActiveInstance::Function(spawn_req, _) => spawn_req.clone(),
                crate::active_instance::ActiveInstance::Resource(_spec, origin) => {
                    log::warn!(
                        "Unsupported resource migration: ignoring request for LID {} to migrate from node_id {}",
                        lid,
                        origin.node_id
                    );
                    return;
                }
            },
            None => {
                log::warn!("Intent to migrate component LID {} that is not active: ignored", lid);
                return;
            }
        };

        // Stop all the function instances associated with this LID.
        self.stop_function_lid(*lid).await;

        // Filter out the unfeasible targets.
        let targets = self.orchestration_logic.feasible_nodes(&spawn_req, targets);

        // Select one feasible target as the candidate one.
        let target = targets.first();
        let mut to_be_started = vec![];
        if let Some(target) = target {
            if targets.len() > 1 {
                log::warn!(
                    "Currently supporting only a single target node per component: choosing {}, the others will be ignored",
                    target
                );
            }
            to_be_started.push((spawn_req.clone(), *target));
        } else {
            log::warn!("No (valid) target found for the migration of function LID {}", lid);
        }

        for (spawn_request, node_id) in to_be_started {
            match self.start_function_in_node(&spawn_request, lid, &node_id).await {
                Ok(_) => {}
                Err(err) => log::error!("Error when migrating function LID {} to node_id {}: {}", lid, node_id, err),
            }
        }
    }

    /// Apply patches on node's run-time agents.
    ///
    /// * `origin_lids` - The logical resource identifiers for which patches
    ///    must be applied.
    async fn apply_patches(&mut self, origin_lids: Vec<edgeless_api::function_instance::ComponentId>) {
        for origin_lid in origin_lids.iter() {
            let ext_output_mapping = match self.dependency_graph.get(origin_lid) {
                Some(x) => x,
                None => continue,
            };

            // Transform logical identifiers (LIDs) into internal ones (PIDs).
            for source in self.lid_to_pid(origin_lid) {
                let mut int_output_mapping = std::collections::HashMap::new();
                for (channel, target_lid) in ext_output_mapping {
                    for target in self.lid_to_pid(target_lid) {
                        // [TODO] Issue#96 The output_mapping structure
                        // should be changed so that multiple
                        // values are possible (with weights), and
                        // this change must be applied to runners,
                        // as well. For now, we just keep
                        // overwriting the same entry.
                        int_output_mapping.insert(channel.clone(), target.instance_id());
                    }
                }

                // Notify the new mapping to the node / resource.
                match source {
                    Pid::Function(instance_id) => match self.nodes.get_mut(&instance_id.node_id) {
                        Some(client_desc) => match client_desc
                            .api
                            .function_instance_api()
                            .patch(edgeless_api::common::PatchRequest {
                                function_id: instance_id.function_id,
                                output_mapping: int_output_mapping,
                            })
                            .await
                        {
                            Ok(_) => {
                                log::info!("Patched node_id {} pid {}", instance_id.node_id, instance_id.function_id);
                            }
                            Err(err) => {
                                log::error!(
                                    "Error when patching node_id {} pid {}: {}",
                                    instance_id.node_id,
                                    instance_id.function_id,
                                    err
                                );
                            }
                        },
                        None => {
                            log::error!("Cannot patch unknown node_id {}", instance_id.node_id);
                        }
                    },
                    Pid::Resource(instance_id) => match self.nodes.get_mut(&instance_id.node_id) {
                        Some(client_desc) => match client_desc
                            .api
                            .resource_configuration_api()
                            .patch(edgeless_api::common::PatchRequest {
                                function_id: instance_id.function_id,
                                output_mapping: int_output_mapping,
                            })
                            .await
                        {
                            Ok(_) => {
                                log::info!("Patched provider node_id {} pid {}", instance_id.node_id, instance_id.function_id);
                            }
                            Err(err) => {
                                log::error!(
                                    "Error when patching provider node_id {} pid {}: {}",
                                    instance_id.node_id,
                                    instance_id.function_id,
                                    err
                                );
                            }
                        },
                        None => {
                            log::error!("Cannot patch unknown provider node_id {}", instance_id.node_id);
                        }
                    },
                };
            }
        }
    }

    /// Create a new resource instance on a random provider.
    ///
    /// If the operation fails, then active_instances is not
    /// updated, i.e., it is as if the request to create the
    /// resource has never been issued.
    ///
    /// * `start_req` - The specifications of the resource.
    /// * `lid` - The logical identifier of the resource.
    async fn start_resource(
        &mut self,
        start_req: edgeless_api::resource_configuration::ResourceInstanceSpecification,
        lid: uuid::Uuid,
    ) -> Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>, anyhow::Error> {
        // Find all resource providers that can start this resource.
        let matching_providers = self
            .resource_providers
            .iter()
            .filter_map(|(id, p)| if p.class_type == start_req.class_type { Some(id.clone()) } else { None })
            .collect::<Vec<String>>();

        // Select one provider at random.
        match matching_providers.choose(&mut self.rng) {
            Some(provider_id) => {
                let resource_provider = self.resource_providers.get_mut(provider_id).unwrap();
                match self.nodes.get_mut(&resource_provider.node_id) {
                    Some(client) => match client
                        .api
                        .resource_configuration_api()
                        .start(edgeless_api::resource_configuration::ResourceInstanceSpecification {
                            class_type: resource_provider.class_type.clone(),
                            // [TODO] Issue #94 remove output mapping
                            output_mapping: std::collections::HashMap::new(),
                            configuration: start_req.configuration.clone(),
                        })
                        .await
                    {
                        Ok(start_response) => match start_response {
                            edgeless_api::common::StartComponentResponse::InstanceId(instance_id) => {
                                assert!(resource_provider.node_id == instance_id.node_id);
                                self.active_instances.insert(
                                    lid,
                                    crate::active_instance::ActiveInstance::Resource(
                                        start_req,
                                        edgeless_api::function_instance::InstanceId {
                                            node_id: resource_provider.node_id,
                                            function_id: instance_id.function_id,
                                        },
                                    ),
                                );
                                self.active_instances_changed = true;
                                log::info!(
                                    "Started resource provider_id {}, node_id {}, lid {}, pid {}",
                                    provider_id,
                                    resource_provider.node_id,
                                    &lid,
                                    instance_id.function_id
                                );
                                Ok(edgeless_api::common::StartComponentResponse::InstanceId(lid))
                            }
                            edgeless_api::common::StartComponentResponse::ResponseError(err) => {
                                Ok(edgeless_api::common::StartComponentResponse::ResponseError(err))
                            }
                        },
                        Err(err) => Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                            edgeless_api::common::ResponseError {
                                summary: "could not start resource".to_string(),
                                detail: Some(err.to_string()),
                            },
                        )),
                    },
                    None => Err(anyhow::anyhow!("Resource Client Missing")),
                }
            }
            None => Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "class type not found".to_string(),
                    detail: Some(format!("class_type: {}", start_req.class_type)),
                },
            )),
        }
    }

    /// Select the node to which to deploy a given function instance.
    ///
    /// Orchestration step: select the node to spawn this
    /// function instance by using the orchestration logic.
    /// Orchestration strategy can also be changed during
    /// runtime.
    ///
    /// * `spawn_req` - The specifications of the function.
    /// * `orchestration_logic` - The orchestration logic configured at run-time.
    fn select_node(
        &mut self,
        spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::function_instance::NodeId> {
        match self.orchestration_logic.next(spawn_req) {
            Some(node_id) => Ok(node_id),
            None => Err(anyhow::anyhow!("no valid node found")),
        }
    }

    /// Start a new function instance on node assigned by orchestration's logic.
    async fn start_function(
        &mut self,
        spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>> {
        // Create a new lid for this resource.
        let lid = uuid::Uuid::new_v4();

        // Select the target node.
        match self.select_node(spawn_req) {
            Ok(node_id) => {
                // Start the function instance.
                self.start_function_in_node(spawn_req, &lid, &node_id).await
            }
            Err(err) => Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: format!("Could not start function {}", spawn_req.code.to_short_string()),
                    detail: Some(err.to_string()),
                },
            )),
        }
    }

    /// Stop an active function with a given logical identifier.
    async fn stop_function_lid(&mut self, lid: uuid::Uuid) {
        match self.active_instances.remove(&lid) {
            Some(active_instance) => {
                self.active_instances_changed = true;
                match active_instance {
                    crate::active_instance::ActiveInstance::Function(_req, instances) => {
                        // Stop all the instances of this function.
                        for instance_id in instances {
                            self.stop_function(&instance_id).await;
                        }
                    }
                    crate::active_instance::ActiveInstance::Resource(_, _) => {
                        log::error!("Request to stop a function but the lid is associated with a resource: lid {}", lid);
                    }
                };
                self.apply_patches(self.dependencies(&lid)).await;
                self.dependency_graph.remove(&lid);
                self.dependency_graph_changed = true;
            }
            None => {
                log::error!("Request to stop a function that is not known: lid {}", lid);
            }
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) {
        // Extract the lid identifiers for the origin and
        // target logical functions.
        let origin_lid = update.function_id;
        let output_mapping = update
            .output_mapping
            .iter()
            .map(|x| (x.0.clone(), x.1.function_id))
            .collect::<std::collections::HashMap<String, edgeless_api::function_instance::ComponentId>>();

        // Save the patch request into an internal data structure,
        // keeping track only of the lid for both origin
        // and target (logical) functions.
        self.dependency_graph.insert(origin_lid, output_mapping);
        self.dependency_graph_changed = true;

        // Apply the patch.
        self.apply_patches(vec![origin_lid]).await;
    }

    /// Start a new function instance on a specific node.
    ///
    /// If the operation fails, then active_instances is not
    /// updated, i.e., it is as if the request to start the
    /// function has never been issued.
    ///
    /// * `spawn_req` - The specifications of the function.
    /// * `lid` - The logical identifier of the function.
    /// * `node_id` - The node where to deploy the function instance.
    async fn start_function_in_node(
        &mut self,
        spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest,
        lid: &uuid::Uuid,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>, anyhow::Error> {
        let mut fn_client = match self.nodes.get_mut(node_id) {
            Some(c) => c,
            None => panic!(
                "Invalid node_id {} selected by the orchestration logic when starting function instance lid {}",
                node_id, lid
            ),
        }
        .api
        .function_instance_api();

        log::debug!(
            "Orchestrator StartFunction {:?} lid {} at worker node with node_id {:?}",
            spawn_req,
            lid,
            node_id
        );

        // Finally try to spawn the function instance on the
        // selected client.
        // [TODO] Issue#96 We assume that one instance is spawned.
        match fn_client.start(spawn_req.clone()).await {
            Ok(res) => match res {
                edgeless_api::common::StartComponentResponse::ResponseError(err) => {
                    Err(anyhow::anyhow!("Could not start a function instance for lid {}: {}", lid, err))
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    assert!(*node_id == id.node_id);
                    self.active_instances.insert(
                        *lid,
                        crate::active_instance::ActiveInstance::Function(
                            spawn_req.clone(),
                            vec![edgeless_api::function_instance::InstanceId {
                                node_id: *node_id,
                                function_id: id.function_id,
                            }],
                        ),
                    );
                    self.active_instances_changed = true;
                    log::info!("Spawned at node_id {}, LID {}, pid {}", node_id, &lid, id.function_id);

                    Ok(edgeless_api::common::StartComponentResponse::InstanceId(*lid))
                }
            },
            Err(err) => {
                log::error!("Unhandled: {}", err);
                Err(anyhow::anyhow!("Could not start a function instance for LID {}: {}", lid, err))
            }
        }
    }

    /// Stop a running function instance.
    ///
    /// * `instance_id` - The function instance to be stopped.
    async fn stop_function(&mut self, instance_id: &edgeless_api::function_instance::InstanceId) {
        match self.nodes.get_mut(&instance_id.node_id) {
            Some(client_desc) => match client_desc.api.function_instance_api().stop(*instance_id).await {
                Ok(_) => {
                    log::info!("Stopped function instance_id {}", instance_id)
                }
                Err(err) => {
                    log::error!("Unhandled stop function instance_id {}: {}", instance_id, err)
                }
            },
            None => log::error!(
                "Cannot stop function instance_id {} because there is no node associated with it",
                instance_id
            ),
        }
    }

    /// Stop an active resource by its logical identifier.
    ///
    /// * `lid` - The logical identier of the resource.
    async fn stop_resource_lid(&mut self, lid: uuid::Uuid) {
        match self.active_instances.remove(&lid) {
            Some(active_instance) => {
                self.active_instances_changed = true;
                match active_instance {
                    crate::active_instance::ActiveInstance::Function(_, _) => {
                        log::error!("Request to stop a resource but the LID is associated with a function: lid {}", lid);
                    }
                    crate::active_instance::ActiveInstance::Resource(_req, instance_id) => {
                        self.stop_resource(&instance_id).await;
                    }
                }
                self.apply_patches(self.dependencies(&lid)).await;
                self.dependency_graph.remove(&lid);
                self.dependency_graph_changed = true;
            }
            None => {
                log::error!("Request to stop a resource that is not known: LID {}", lid);
            }
        }
    }

    /// Stop a running resource instance.
    ///
    /// * `instance_id` - The resource instance to be stopped.
    async fn stop_resource(&mut self, instance_id: &edgeless_api::function_instance::InstanceId) {
        match self.nodes.get_mut(&instance_id.node_id) {
            Some(node_client) => match node_client.api.resource_configuration_api().stop(*instance_id).await {
                Ok(_) => {
                    log::info!("Stopped resource instance_id {}", instance_id)
                }
                Err(err) => {
                    log::error!("Unhandled stop resource instance_id {}: {}", instance_id, err)
                }
            },
            None => log::error!(
                "Cannot stop resource instance_id {} because there is no node associated with it",
                instance_id
            ),
        }
    }

    /// Return the list of lids that depend on the given one, according
    /// to the active patches.
    ///
    /// If we see the functions and output_mappings as a graph where:
    /// - there is a vertex for every function/resource,
    /// - there is an edge for every output_mapping between two functions/resources
    ///
    /// this function will return all the ingress vertices of the vertex
    /// identified by `lid`.
    fn dependencies(&self, lid: &uuid::Uuid) -> Vec<uuid::Uuid> {
        let mut dependencies = vec![];
        for (origin_lid, output_mapping) in self.dependency_graph.iter() {
            for (_output, target_lid) in output_mapping.iter() {
                if target_lid == lid {
                    dependencies.push(*origin_lid);
                    break;
                }
            }
        }
        dependencies
    }

    /// Return the aggregated capabilities of the nodes in the domain.
    fn domain_capabilities(&self) -> edgeless_api::domain_registration::DomainCapabilities {
        let mut ret = edgeless_api::domain_registration::DomainCapabilities::default();
        for client_desc in self.nodes.values() {
            let caps = &client_desc.capabilities;
            ret.num_nodes += 1;
            ret.num_cpus += caps.num_cpus;
            ret.num_cores += caps.num_cores;
            ret.mem_size += caps.mem_size;
            ret.labels.extend(caps.labels.iter().cloned());
            if caps.is_tee_running {
                ret.num_tee += 1;
            }
            if caps.has_tpm {
                ret.num_tpm += 1;
            }
            ret.runtimes.extend(caps.runtimes.iter().cloned());
            ret.disk_tot_space += caps.disk_tot_space;
            ret.num_gpus += caps.num_gpus;
            ret.mem_size_gpu += caps.mem_size_gpu;
        }
        for (provider_id, provider) in self.resource_providers.iter() {
            ret.resource_providers.insert(provider_id.clone());
            ret.resource_classes.insert(provider.class_type.clone());
        }
        ret
    }

    async fn add_node(
        &mut self,
        node_id: uuid::Uuid,
        client_desc: crate::client_desc::ClientDesc,
        resource_providers: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
    ) {
        // Create the resource configuration APIs.
        for resource in resource_providers {
            log::info!("New resource advertised by node {}: {}", node_id, resource);

            if self.resource_providers.contains_key(&resource.provider_id) {
                log::warn!(
                    "cannot add resource because another one exists with the same provider_id: {}",
                    resource.provider_id
                )
            } else {
                self.resource_providers.insert(
                    resource.provider_id.clone(),
                    crate::resource_provider::ResourceProvider {
                        class_type: resource.class_type.clone(),
                        node_id: node_id.clone(),
                        outputs: resource.outputs.clone(),
                    },
                );
            }
        }

        // Create the node's descriptor, with associated client.
        log::info!("New node ID {} {}", node_id, client_desc.to_string_short());

        let invocation_url = client_desc.invocation_url.clone();
        self.nodes.insert(node_id, client_desc);

        // Update all the peers, including the new node.
        let mut num_failures: u32 = 0;
        for (_node_id, client) in self.nodes.iter_mut() {
            if client
                .api
                .node_management_api()
                .update_peers(edgeless_api::node_management::UpdatePeersRequest::Add(node_id, invocation_url.clone()))
                .await
                .is_err()
            {
                num_failures += 1;
            }
        }

        // Update the new node by adding as peers all the existing nodes.
        let mut new_node_client = self
            .nodes
            .get_mut(&node_id)
            .expect("New node added just vanished")
            .api
            .node_management_api();
        for (other_node_id, client_desc) in self.nodes.iter_mut() {
            if other_node_id.eq(&node_id) {
                continue;
            }
            if new_node_client
                .update_peers(edgeless_api::node_management::UpdatePeersRequest::Add(
                    *other_node_id,
                    client_desc.invocation_url.clone(),
                ))
                .await
                .is_err()
            {
                num_failures += 1;
            }
        }

        if num_failures > 0 {
            log::error!(
                "There have been failures ({}) when updating the peers following the addition of node '{}', the data plane may not work properly",
                num_failures,
                node_id
            );
        }
    }

    async fn del_node(&mut self, node_id: uuid::Uuid) {
        // Remove the node from the map of clients.
        log::info!("Removing node '{}'", node_id);
        if let Some(_client_desc) = self.nodes.remove(&node_id) {
            // Remove all the resource providers associated with it.
            self.resource_providers.retain(|_k, v| v.node_id != node_id);

            // Update the peers of (still alive) nodes by
            // deleting the missing-in-action peer.
            for (_, client_desc) in self.nodes.iter_mut() {
                if let Err(err) = client_desc
                    .api
                    .node_management_api()
                    .update_peers(edgeless_api::node_management::UpdatePeersRequest::Del(node_id))
                    .await
                {
                    log::error!("Unhandled: {}", err);
                }
            }
        } else {
            log::error!("Cannot delete non-existing node '{}'", node_id);
        }
    }

    async fn update_domain(&mut self) {
        let new_domain_capabilities = self.domain_capabilities();
        if new_domain_capabilities == self.last_domain_capabilities {
            return;
        }

        // Keep the last domain capabilities for the future.
        self.last_domain_capabilities = new_domain_capabilities.clone();

        // Notify the domain register of the updated capabilities.
        let _ = self
            .subscriber_sender
            .send(super::domain_subscriber::DomainSubscriberRequest::Update(Box::new(
                new_domain_capabilities,
            )))
            .await;

        // Update the orchestration logic.
        self.orchestration_logic.update_nodes(&self.nodes, &self.resource_providers);

        // Update the proxy.
        let mut proxy = self.proxy.lock().await;
        proxy.update_nodes(&self.nodes);
        proxy.update_resource_providers(&self.resource_providers);
    }

    async fn refresh(&mut self) {
        //
        // Make sure that all active logical functions are assigned
        // to one instance: for all the function instances that
        // were running in disconnected nodes, create new function
        // instances on other nodes, if possible and there were no
        // other running function instances.
        //

        // List of LIDs that will have to be repatched
        // because of the allocation of new function instances
        // following node disconnection.
        let mut to_be_repatched = vec![]; // lid

        // Function instances that have to be created to make up for
        // the loss of those assigned to disconnected nodes.
        // key:   lid
        // value: function request
        let mut fun_to_be_created = std::collections::HashMap::new();

        // Resources that have to be created to make up for the
        // loss of those assigned to disconnected nodes.
        // key:   lid
        // value: resource specs
        let mut res_to_be_created = std::collections::HashMap::new();

        // List of lid that will have to be repatched.
        let mut active_instances_to_be_updated = vec![];

        // Find all the functions/resources affected.
        // Also attempt to start functions and resources that
        // are active but for which no active instance is present
        // (this happens because in the past a node with active
        // functions/resources has disappeared and it was not
        // possible to fix the situation immediately).
        for (origin_lid, instance) in self.active_instances.iter() {
            match instance {
                crate::active_instance::ActiveInstance::Function(start_req, instances) => {
                    let num_disconnected = instances.iter().filter(|x| !self.nodes.contains_key(&x.node_id)).count();
                    assert!(num_disconnected <= instances.len());
                    if instances.is_empty() || num_disconnected > 0 {
                        to_be_repatched.push(*origin_lid);
                        if instances.is_empty() || num_disconnected == instances.len() {
                            // If all the function instances
                            // disappared, then we must enforce the
                            // creation of (at least) a new
                            // function instance.
                            fun_to_be_created.insert(*origin_lid, start_req.clone());
                        } else {
                            // Otherwise, we just remove the
                            // disappeared function instances and
                            // let the others still alive handle
                            // the logical function.
                            active_instances_to_be_updated.push(*origin_lid);
                        }
                    }
                }
                crate::active_instance::ActiveInstance::Resource(start_req, instance) => {
                    if instance.is_none() || !self.nodes.contains_key(&instance.node_id) {
                        to_be_repatched.push(*origin_lid);
                        res_to_be_created.insert(*origin_lid, start_req.clone());
                    }
                }
            }
        }

        // Also schedule to repatch all the functions that
        // depend on the functions/resources modified.
        for (origin_lid, output_mapping) in self.dependency_graph.iter() {
            for (_output, target_lid) in output_mapping.iter() {
                if active_instances_to_be_updated.contains(target_lid)
                    || fun_to_be_created.contains_key(target_lid)
                    || res_to_be_created.contains_key(target_lid)
                {
                    to_be_repatched.push(*origin_lid);
                }
            }
        }

        // Update the active instances of logical functions
        // where at least one function instance went missing but
        // there are others that are still assigned and alive.
        for lid in active_instances_to_be_updated.iter() {
            match self.active_instances.get_mut(lid) {
                None => panic!("lid {} just disappeared", lid),
                Some(active_instance) => match active_instance {
                    crate::active_instance::ActiveInstance::Resource(_, _) => panic!("expecting a function, found a resource for lid {}", lid),
                    crate::active_instance::ActiveInstance::Function(_, instances) => {
                        instances.retain(|x| self.nodes.contains_key(&x.node_id));
                        self.active_instances_changed = true;
                    }
                },
            }
        }

        // Create the functions that went missing.
        // If the operation fails for a function now, then the
        // function remains in the active_instances, but it is
        // assigned no function instance.
        for (lid, spawn_req) in fun_to_be_created.into_iter() {
            let res = match self.select_node(&spawn_req) {
                Ok(node_id) => {
                    // Start the function instance.
                    match self.start_function_in_node(&spawn_req, &lid, &node_id).await {
                        Ok(_) => Ok(()),
                        Err(err) => Err(err),
                    }
                }
                Err(err) => Err(err),
            };
            if let Err(err) = res {
                log::error!("Error when creating a new function assigned with lid {}: {}", lid, err);
                match self.active_instances.get_mut(&lid).unwrap() {
                    crate::active_instance::ActiveInstance::Function(_spawn_req, instances) => {
                        instances.clear();
                        self.active_instances_changed = true;
                    }
                    crate::active_instance::ActiveInstance::Resource(_, _) => {
                        panic!("Expecting a function to be associated with LID {}, found a resource", lid)
                    }
                }
            }
        }

        // Create the resources that went missing.
        // If the operation fails for a resource now, then the
        // resource remains in the active_instances, but it is
        // assigned an invalid function instance.
        for (lid, start_req) in res_to_be_created.into_iter() {
            if let Err(err) = self.start_resource(start_req, lid).await {
                log::error!("Error when creating a new resource assigned with lid {}: {}", lid, err);
                match self.active_instances.get_mut(&lid).unwrap() {
                    crate::active_instance::ActiveInstance::Function(_, _) => {
                        panic!("expecting a resource to be associated with LID {}, found a function", lid)
                    }
                    crate::active_instance::ActiveInstance::Resource(_start_req, instance_id) => {
                        *instance_id = edgeless_api::function_instance::InstanceId::none();
                        self.active_instances_changed = true;
                    }
                }
            }
        }

        // Check if there are intents from the proxy.
        let deploy_intents = self.proxy.lock().await.retrieve_deploy_intents();
        for intent in deploy_intents {
            match intent {
                crate::deploy_intent::DeployIntent::Migrate(component, targets) => {
                    self.migrate(&component, &targets).await;
                    to_be_repatched.push(component)
                }
            }
        }

        // Repatch everything that needs to be repatched.
        self.apply_patches(to_be_repatched).await;

        // Update the proxy.
        let mut proxy = self.proxy.lock().await;
        if self.active_instances_changed {
            proxy.update_active_instances(&self.active_instances);
            self.active_instances_changed = false;
        }
        if self.dependency_graph_changed {
            proxy.update_dependency_graph(&self.dependency_graph);
            self.dependency_graph_changed = false;
        }
    }
}
