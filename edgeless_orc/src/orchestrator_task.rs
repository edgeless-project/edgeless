// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use futures::{SinkExt, StreamExt};
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::active_instance::ActiveInstance;
use edgeless_telemetry::control_plane_tracer::TraceSpan;

#[derive(Debug)]
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

/// Precomputed patch request ready to be sent to a node.
/// This structure allows for deterministic/real-time processing by having
/// all patch information ready before the actual send operation.
struct PrecomputedPatch {
    source_pid: Pid,
    output_mapping: std::collections::HashMap<String, edgeless_api::function_instance::InstanceId>,
}

/// result of the critical failover detection phase.
/// contains all information needed for fast-path repatching.
struct CriticalFailoverResult {
    // lids that need immediate repatching (hot-standby was promoted)
    to_repatch: Vec<uuid::Uuid>,
    // new replicas needed to maintain replication factor
    replicas_to_create: Vec<NewReplicaRequest>,
    // workflow_ids that should be stopped due to KPI-13 failure (no hot-standby available)
    workflows_to_stop: Vec<String>,
}

/// information needed to create a new physical function instance
struct NewReplicaRequest {
    lid: uuid::Uuid,
    spawn_req: edgeless_api::function_instance::SpawnFunctionRequest,
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
    // set of lids that have replication_factor (critical functions)
    // this allows fast iteration over only critical functions during failover
    critical_lids: std::collections::HashSet<uuid::Uuid>,
    // active patches to which the orchestrator commits
    // key:   lid (origin function)
    // value: map of:
    //        key:   channel output name
    //        value: lid (target function)
    dependency_graph: std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>,
    dependency_graph_changed: bool,
    tracer: Option<edgeless_telemetry::control_plane_tracer::ControlPlaneTracer>,
}

impl OrchestratorTask {
    pub async fn new(
        receiver: futures::channel::mpsc::UnboundedReceiver<crate::orchestrator::OrchestratorRequest>,
        orchestrator_settings: crate::EdgelessOrcBaselineSettings,
        proxy: std::sync::Arc<tokio::sync::Mutex<dyn super::proxy::Proxy>>,
        subscriber_sender: futures::channel::mpsc::UnboundedSender<super::domain_subscriber::DomainSubscriberRequest>,
    ) -> Self {
        Self {
            receiver,
            nodes: std::collections::HashMap::new(),
            resource_providers: std::collections::HashMap::new(),
            proxy,
            subscriber_sender,
            orchestration_logic: crate::orchestration_logic::OrchestrationLogic::new(orchestrator_settings.orchestration_strategy),
            rng: rand::rngs::StdRng::from_entropy(),
            active_instances: std::collections::HashMap::new(),
            active_instances_changed: false,
            critical_lids: std::collections::HashSet::new(),
            dependency_graph: std::collections::HashMap::new(),
            dependency_graph_changed: false,
            tracer: edgeless_telemetry::control_plane_tracer::ControlPlaneTracer::new("/tmp/orchestrator_kpi_samples.csv".to_string()).ok(),
        }
    }

    // Main orchestration loop.
    pub async fn run(&mut self) {
        self.update_domain().await;
        while let Some(req) = self.receiver.next().await {
            match req {
                crate::orchestrator::OrchestratorRequest::StartFunction(spawn_req, reply_channel) => {
                    log::debug!("Orchestrator StartFunction {}", spawn_req.spec.to_short_string());
                    let res = self.start_function(&spawn_req).await;
                    if let Err(err) = reply_channel.send(res) {
                        log::error!("Orchestrator channel error in SPAWN: {:?}", err);
                    }
                    self.refresh(None).await;
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
                crate::orchestrator::OrchestratorRequest::AddNode(node_id, mut client_desc, resource_providers) => {
                    log::debug!("Orchestrator AddNode {}", client_desc.to_string_short());

                    // Reset the node to clean state before adding - this removes any stale
                    // function/resource instances that may have survived a temporary disconnection
                    if let Err(err) = client_desc.api.node_management_api().reset().await {
                        log::error!("Failed to reset node '{}' before adding, node may have stale state: {}", node_id, err);
                    }

                    self.add_node(node_id, client_desc, resource_providers).await;
                    self.update_domain().await;
                    self.refresh(None).await;
                }
                crate::orchestrator::OrchestratorRequest::DelNode(node_id) => {
                    log::debug!("Orchestrator DelNode {:?}", node_id);
                    let del_node_span = self.tracer.as_ref().map(|t| t.start_span("del_node"));
                    self.del_node(node_id).await;
                    self.update_domain().await;
                    self.refresh(del_node_span.as_ref()).await;
                }
                crate::orchestrator::OrchestratorRequest::Refresh(reply_sender) => {
                    log::debug!("Orchestrator Refresh");
                    self.refresh(None).await;
                    let _ = reply_sender.send(());
                }
                crate::orchestrator::OrchestratorRequest::Reset() => {
                    log::debug!("Orchestrator Reset");
                    self.reset().await;
                }
            }
        }
    }

    fn lid_to_pid(&self, lid: &edgeless_api::function_instance::ComponentId) -> Vec<Pid> {
        match self.active_instances.get(lid) {
            Some(active_instance) => match active_instance {
                crate::active_instance::ActiveInstance::Function(_req, instances) => instances
                    .iter()
                    // NOTE: this is important: we only patch active instances, not hot-standby ones!
                    .filter(|x| x.1) // only include active instances (x.1 == true), not hot-standby - hot-standby are special
                    .map(|x| {
                        Pid::Function(edgeless_api::function_instance::InstanceId {
                            node_id: x.0.node_id,
                            function_id: x.0.function_id,
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
    /// If the component cannot be migrated to the target, then the current
    /// component instances are not stopped.
    ///
    /// If the component is already allocated precisely on the same targets
    /// then nothing happens.
    ///
    /// * `lid` - The LID of the function/resource to be migrated.
    /// * `targets` - The set of nodes to which the instance has to be migrated.
    ///
    /// * Return the id of the node to which this instance has been migrated,
    ///   in case of success.
    async fn migrate(
        &mut self,
        lid: &edgeless_api::function_instance::ComponentId,
        targets: &Vec<edgeless_api::function_instance::NodeId>,
    ) -> anyhow::Result<uuid::Uuid> {
        // Retrieve the origin logical IDs and:
        // - if it's a function: the spawn request
        // - if it's a resource: the specification
        // One or the other must be set to some value.
        let (spawn_req, resource_req, origin_instances) = match self.active_instances.get(lid) {
            Some(active_instance) => match active_instance {
                crate::active_instance::ActiveInstance::Function(spawn_req, origin_instances) => {
                    (Some(spawn_req.clone()), None, origin_instances.iter().map(|(id, _)| *id).collect())
                }
                crate::active_instance::ActiveInstance::Resource(resource_spec, origin_lid) => (None, Some(resource_spec.clone()), vec![*origin_lid]),
            },
            None => {
                anyhow::bail!("Intent to migrate component LID {} that is not active: ignored", lid);
            }
        };

        assert!(spawn_req.is_some() ^ resource_req.is_some());

        // Return immediately if the migration is requested to precisely the
        // set of nodes to which the instance is already assigned.
        let target_node_ids: std::collections::HashSet<&uuid::Uuid> = std::collections::HashSet::from_iter(targets.iter());
        let origin_node_ids: std::collections::HashSet<&uuid::Uuid> =
            std::collections::HashSet::from_iter(origin_instances.iter().map(|x| &x.node_id));
        anyhow::ensure!(target_node_ids != origin_node_ids, "instance already running on the migration target(s)");

        // Do the migration of the function or resource.
        if let Some(spawn_req) = spawn_req {
            // Filter out the unfeasible targets.
            let target_node_ids = self.orchestration_logic.feasible_nodes(&spawn_req, targets);

            // Select one feasible target as the candidate one.
            let target = target_node_ids.first();
            let mut to_be_started = vec![];
            if let Some(target) = target {
                if target_node_ids.len() > 1 {
                    log::warn!(
                        "Currently supporting only a single target node per component: choosing {}, the others will be ignored",
                        target
                    );
                }
                to_be_started.push((spawn_req.clone(), *target));
            } else {
                anyhow::bail!("No (valid) target found for the migration of function LID {}", lid);
            }

            // Stop all the function instances associated with this LID.
            for origin_instance in &origin_instances {
                self.stop_function(origin_instance).await;
            }

            // Remove the association of the component with origin instances.
            // If the start below fails, then the function instance will remain
            // associated with no instances.
            if let Some(crate::active_instance::ActiveInstance::Function(_spawn_req, origin_instances)) = self.active_instances.get_mut(lid) {
                origin_instances.clear();
            }
            self.active_instances_changed = true;

            // Start the new function instances.
            assert_eq!(1, to_be_started.len());
            for (spawn_request, node_id) in to_be_started {
                if let Err(err) = self.start_function_in_node(&spawn_request, lid, &node_id).await {
                    // TODO: if migration to multiple instances is supported,
                    // then we should choose how to consider the case of a
                    // function start failing while others succeed:
                    // - if this is considered a failure, then the function
                    // instances already started should be stopped (rollback)
                    // - otherwise, an Ok must be returned instead of an Err
                    anyhow::bail!("Error when migrating function LID {} to node_id {}: {}", lid, node_id, err);
                }
            }
            Ok(*target.expect("impossible: the target node must have a value"))
        } else if let Some(resource_req) = resource_req {
            assert!(origin_instances.len() <= 1);

            // Try to allocate the resource on the given node.
            if let Some(target_node_id) = targets.first() {
                if self.is_node_feasible_for_resource(&resource_req, target_node_id) {
                    // Stop the resource instances associated with this LID, if any.
                    for origin_lid in &origin_instances {
                        self.stop_resource(origin_lid).await;
                    }

                    // Remove the association of the component with origin instances.
                    // If the start below fails, then the function instance will remain
                    // associated with no instances.
                    if let Some(crate::active_instance::ActiveInstance::Resource(_resource_req, origin_instance)) = self.active_instances.get_mut(lid)
                    {
                        *origin_instance = edgeless_api::function_instance::InstanceId::none();
                    }
                    self.active_instances_changed = true;

                    if let Err(err) = self.start_resource_in_node(resource_req, lid, target_node_id).await {
                        anyhow::bail!("Error when migrating resource LID {} to node_id {}: {}", lid, target_node_id, err);
                    } else {
                        Ok(*target_node_id)
                    }
                } else {
                    anyhow::bail!(
                        "Request to migrate resource '{}' to node_id '{}', which does not have matching resource providers",
                        lid,
                        target_node_id
                    );
                }
            } else {
                anyhow::bail!("Request to migrate resource '{}' to a null target", lid);
            }
        } else {
            panic!("the impossible happened, this branch should never be reached")
        }
    }

    /// Precompute patch requests for the given LIDs.
    /// Returns a list of patches ready to be sent, without performing any I/O.
    /// This separation enables future deterministic/real-time processing where
    /// patch data is prepared ahead of time.
    fn precompute_patches(&self, origin_lids: &[edgeless_api::function_instance::ComponentId]) -> Vec<PrecomputedPatch> {
        let mut patches = Vec::new();

        for origin_lid in origin_lids.iter() {
            let logical_output_mapping = match self.dependency_graph.get(origin_lid) {
                Some(x) => x,
                None => continue,
            };

            // Transform logical identifiers (LIDs) into physical ones (PIDs)
            for source in self.lid_to_pid(origin_lid) {
                let mut physical_output_mapping = std::collections::HashMap::new();
                for (channel, target_lid) in logical_output_mapping {
                    for target in self.lid_to_pid(target_lid) {
                        // [TODO] Issue#96 The output_mapping structure
                        // should be changed so that multiple
                        // values are possible (with weights), and
                        // this change must be applied to runners,
                        // as well. For now, we just keep
                        // overwriting the same entry.
                        physical_output_mapping.insert(channel.clone(), target.instance_id());
                    }
                }

                patches.push(PrecomputedPatch {
                    source_pid: source,
                    output_mapping: physical_output_mapping,
                });
            }
        }

        patches
    }

    /// Send a single precomputed patch to the appropriate node.
    /// This is a low-level operation that performs the actual I/O.
    async fn send_patch(nodes: &mut std::collections::HashMap<uuid::Uuid, crate::client_desc::ClientDesc>, patch: PrecomputedPatch) {
        match patch.source_pid {
            Pid::Function(instance_id) => {
                if let Some(client_desc) = nodes.get_mut(&instance_id.node_id) {
                    match client_desc
                        .api
                        .function_instance_api()
                        .patch(edgeless_api::common::PatchRequest {
                            function_id: instance_id.function_id,
                            output_mapping: patch.output_mapping,
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
                    }
                } else {
                    log::error!("Cannot patch unknown node_id {}", instance_id.node_id);
                }
            }
            Pid::Resource(instance_id) => {
                if let Some(client_desc) = nodes.get_mut(&instance_id.node_id) {
                    match client_desc
                        .api
                        .resource_configuration_api()
                        .patch(edgeless_api::common::PatchRequest {
                            function_id: instance_id.function_id,
                            output_mapping: patch.output_mapping,
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
                    }
                } else {
                    log::error!("Cannot patch unknown provider node_id {}", instance_id.node_id);
                }
            }
        }
    }

    /// Apply patches on node's run-time agents.
    /// This method precomputes patches and sends them sequentially.
    ///
    /// * `origin_lids` - The logical resource identifiers for which patches
    ///   must be applied.
    /// * `parent_span` - Optional parent tracing span for KPI measurement.
    async fn apply_patches(&mut self, origin_lids: Vec<edgeless_api::function_instance::ComponentId>, parent_span: Option<&TraceSpan>) {
        let _span = parent_span.map(|s| s.child("apply_patches"));

        // Precompute all patches first
        let patches = self.precompute_patches(&origin_lids);

        // Send patches sequentially (for backward compatibility)
        for patch in patches {
            Self::send_patch(&mut self.nodes, patch).await;
        }
    }

    /// Create a new resource instance on a random provider.
    ///
    /// If the operation fails, then active_instances is not
    /// updated, i.e., it is as if the request to create the
    /// resource has never been issued.
    ///
    /// * `resource_req` - The specifications of the resource.
    /// * `lid` - The logical identifier of the resource.
    async fn start_resource(
        &mut self,
        resource_req: edgeless_api::resource_configuration::ResourceInstanceSpecification,
        lid: uuid::Uuid,
    ) -> Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>, anyhow::Error> {
        // Find all resource providers that can start this resource.
        let matching_providers = self.feasible_providers(&resource_req);

        // Select one provider at random.
        match matching_providers.choose(&mut self.rng) {
            Some(provider_id) => {
                let resource_provider = self.resource_providers.get(provider_id).unwrap();
                let node_id = resource_provider.node_id;
                self.start_resource_in_node(resource_req, &lid, &node_id).await
            }
            None => Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "class type not found".to_string(),
                    detail: Some(format!("class_type: {}", resource_req.class_type)),
                },
            )),
        }
    }

    /// Return the list of resource providers that are feasible for the given
    /// resource specification.
    fn feasible_providers(&self, resource_req: &edgeless_api::resource_configuration::ResourceInstanceSpecification) -> Vec<String> {
        // Special case for portal resources: always select that for the
        // domain specified in the configuration.
        if resource_req.class_type == "portal" && Some(&String::from("portal")) == resource_req.configuration.get("domain") {
            if let Some(domain_name) = resource_req.configuration.get("domain_name") {
                let resource_name = format!("portal-{domain_name}");
                if let Some(provider_id) = self.resource_providers.keys().find(|provider_id| **provider_id == resource_name) {
                    return vec![provider_id.to_string()];
                }
            }
            return vec![];
        }

        let cordoned_nodes = self
            .nodes
            .iter()
            .filter_map(|(node_id, desc)| if desc.cordoned { Some(*node_id) } else { None })
            .collect::<std::collections::HashSet<edgeless_api::function_instance::NodeId>>();
        self.resource_providers
            .iter()
            .filter_map(|(provider_id, provider)| {
                let capabilities = &self.nodes.get(&provider.node_id).unwrap().capabilities;
                let deployment_requirements = crate::deployment_requirements::DeploymentRequirements::from_annotations(&resource_req.configuration);
                if provider.class_type == resource_req.class_type
                    && !cordoned_nodes.contains(&provider.node_id)
                    && deployment_requirements.is_feasible(&provider.node_id, capabilities, &std::collections::HashSet::default())
                {
                    Some(provider_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>()
    }

    // Return true if the given resource can be created on this node.
    fn is_node_feasible_for_resource(
        &self,
        resource_req: &edgeless_api::resource_configuration::ResourceInstanceSpecification,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> bool {
        if let Some(desc) = self.nodes.get(node_id) {
            if desc.cordoned {
                return false;
            }
        }
        let capabilities = &self.nodes.get(node_id).unwrap().capabilities;
        if !crate::deployment_requirements::DeploymentRequirements::from_annotations(&resource_req.configuration).is_feasible(
            node_id,
            capabilities,
            &std::collections::HashSet::default(),
        ) {
            return false;
        }
        for provider in self.resource_providers.values() {
            if resource_req.class_type == provider.class_type && *node_id == provider.node_id {
                return true;
            }
        }
        false
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

    fn select_node_excluding(
        &mut self,
        spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest,
        exclude_node_ids: &Vec<edgeless_api::function_instance::NodeId>,
    ) -> anyhow::Result<edgeless_api::function_instance::NodeId> {
        match self.orchestration_logic.next_excluding(spawn_req, exclude_node_ids) {
            Some(node_id) => Ok(node_id),
            None => Err(anyhow::anyhow!("no valid node found (redundancy requires that there are enough nodes)")),
        }
    }

    /// Start a new logical function in this orchestration domain, as assigned
    /// by the controller
    async fn start_function(
        &mut self,
        spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>> {
        // Create a new lid for this function.
        let lid = uuid::Uuid::new_v4();
        let mut results: Vec<anyhow::Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>>> = vec![];

        // check if we have enough nodes to satisfy the replication factor before starting anything
        // this prevents the situation in which a workflow gets partially started
        if let Some(replication_factor) = spawn_req.replication_factor {
            if replication_factor > 1 {
                // get all feasible nodes without modifying any state
                let all_node_ids: Vec<uuid::Uuid> = self.nodes.keys().cloned().collect();
                let feasible_nodes = self.orchestration_logic.feasible_nodes(spawn_req, &all_node_ids);

                if (feasible_nodes.len() as u32) < replication_factor {
                    return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: "Failed to start function".to_string(),
                            detail: Some(format!(
                                "Not enough suitable nodes to satisfy replication_factor={}. Found {} suitable nodes.",
                                replication_factor,
                                feasible_nodes.len()
                            )),
                        },
                    ));
                }
            }
        }

        // Select the target node.
        let (res, active_instance_node_id) = match self.select_node(spawn_req) {
            Ok(node_id) => {
                // Start the function instance.
                (self.start_function_in_node(spawn_req, &lid, &node_id).await, node_id)
            }
            Err(err) => (
                Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                    edgeless_api::common::ResponseError {
                        summary: format!("Could not start function {}", spawn_req.spec.to_short_string()),
                        detail: Some(err.to_string()),
                    },
                )),
                uuid::Uuid::nil(),
            ),
        };
        results.push(res);

        // start the replicas for hot-standby redundancy, if replication factor is > 1
        if let Some(replication_factor) = spawn_req.replication_factor {
            if replication_factor > 1 {
                // start replicas on different nodes to provide good coverage and good fault tolerance
                let mut used_node_ids = vec![active_instance_node_id];
                for _ in 1..replication_factor {
                    let res = match self.select_node_excluding(spawn_req, &used_node_ids) {
                        Ok(node_id) => {
                            // Start the function instance.
                            let res = self.start_function_in_node(spawn_req, &lid, &node_id).await;
                            // update the list of used node ids
                            used_node_ids.push(node_id);
                            res
                        }
                        Err(err) => Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                            edgeless_api::common::ResponseError {
                                summary: format!("Could not start replica function {}", spawn_req.spec.to_short_string()),
                                detail: Some(err.to_string()),
                            },
                        )),
                    };
                    results.push(res);
                }
            }
        }

        // Check if there was any error along the way
        let mut error_count = 0;
        let mut last_error: Option<String> = None;

        for result in &results {
            match result {
                Ok(edgeless_api::common::StartComponentResponse::ResponseError(err)) => {
                    error_count += 1;
                    last_error = Some(format!("{}: {}", err.summary, err.detail.as_ref().unwrap_or(&String::new())));
                }
                Err(err) => {
                    error_count += 1;
                    last_error = Some(err.to_string());
                }
                Ok(edgeless_api::common::StartComponentResponse::InstanceId(_)) => {
                    // Success case - no action needed
                }
            }
        }

        // If all instances failed, return error and clean up
        if error_count == results.len() {
            // Clean up any partial state
            if self.active_instances.contains_key(&lid) {
                self.active_instances.remove(&lid);
                self.active_instances_changed = true;
            }

            return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Failed to start function".to_string(),
                    detail: last_error,
                },
            ));
        }

        // even if some replicas failed, we consider this a success as long as
        // the first instance is running - check it, if not clean up and return the error
        match &results[0] {
            Ok(edgeless_api::common::StartComponentResponse::InstanceId(_)) => {
                // track this function as critical if it has replication_factor
                if spawn_req.replication_factor.is_some() {
                    self.critical_lids.insert(lid);
                }
                Ok(edgeless_api::common::StartComponentResponse::InstanceId(lid))
            }
            _ => {
                // first instance failed, clean up and return error
                if self.active_instances.contains_key(&lid) {
                    self.active_instances.remove(&lid);
                    self.active_instances_changed = true;
                }

                Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                    edgeless_api::common::ResponseError {
                        summary: "Failed to start first function instance".to_string(),
                        detail: last_error,
                    },
                ))
            }
        }
    }

    /// Stop an active function with a given logical identifier.
    async fn stop_function_lid(&mut self, lid: uuid::Uuid) {
        match self.active_instances.remove(&lid) {
            Some(active_instance) => {
                self.active_instances_changed = true;
                self.critical_lids.remove(&lid);
                match active_instance {
                    crate::active_instance::ActiveInstance::Function(_req, instances) => {
                        // Stop all the instances of this function.
                        for instance_id in instances {
                            self.stop_function(&instance_id.0).await;
                        }
                    }
                    crate::active_instance::ActiveInstance::Resource(_, _) => {
                        log::error!("Request to stop a function but the lid is associated with a resource: lid {}", lid);
                    }
                };
                self.apply_patches(self.dependencies(&lid), None).await;
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
        self.apply_patches(vec![origin_lid], None).await;
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
        // [TODO] Issue#96 We assume that one "active" instance is spawned per node.
        // When replication_factor is specifid, we start more instances in standby mode.
        // Other instances, spawned on other nodes are considered "hot-standby" ones.
        match fn_client.start(spawn_req.clone()).await {
            Ok(res) => match res {
                edgeless_api::common::StartComponentResponse::ResponseError(err) => {
                    Err(anyhow::anyhow!("Could not start a function instance for lid {}: {}", lid, err))
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    assert!(*node_id == id.node_id);
                    // if the lid is already present, append the new instance id to the list
                    if let Some(existing_instance) = self.active_instances.get_mut(lid) {
                        let is_active = existing_instance.instance_ids().is_empty();
                        existing_instance.instance_ids_mut().append(&mut vec![(
                            edgeless_api::function_instance::InstanceId {
                                node_id: *node_id,
                                function_id: id.function_id,
                            },
                            is_active,
                        )]); // hot-standby instance (false = standby, true = active)
                        log::info!(
                            "Spawned {} instance number {} at node_id {}, LID {}, pid {}",
                            if is_active { "active" } else { "hot-standby" },
                            self.active_instances.get(lid).unwrap().instance_ids().len(),
                            node_id,
                            &lid,
                            id.function_id
                        );
                    } else {
                        self.active_instances.insert(
                            *lid,
                            crate::active_instance::ActiveInstance::Function(
                                spawn_req.clone(),
                                vec![(
                                    edgeless_api::function_instance::InstanceId {
                                        node_id: *node_id,
                                        function_id: id.function_id,
                                    },
                                    true,
                                )], // first instance is active (true = active, false = standby)
                            ),
                        );
                        log::info!(
                            "Spawned active instance number {} at node_id {}, LID {}, pid {}",
                            self.active_instances.get(lid).unwrap().instance_ids().len(),
                            node_id,
                            &lid,
                            id.function_id
                        );
                    }
                    self.active_instances_changed = true;

                    Ok(edgeless_api::common::StartComponentResponse::InstanceId(*lid))
                }
            },
            Err(err) => {
                log::error!("Unhandled: {}", err);
                Err(anyhow::anyhow!("Could not start a function instance for LID {}: {}", lid, err))
            }
        }
    }

    /// Start a new resource instance on a specific node/resource provider.
    ///
    /// If the operation fails, then active_instances is not
    /// updated, i.e., it is as if the request to start the
    /// resource has never been issued.
    ///
    /// * `resource_spec` - The specifications of the function.
    /// * `lid` - The logical identifier of the function.
    /// * `node_id` - The node hosting the given resource provider.
    async fn start_resource_in_node(
        &mut self,
        resource_req: edgeless_api::resource_configuration::ResourceInstanceSpecification,
        lid: &uuid::Uuid,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>, anyhow::Error> {
        let class_type = resource_req.class_type.clone();
        match self.nodes.get_mut(node_id) {
            Some(client) => match client.api.resource_configuration_api().start(resource_req.clone()).await {
                Ok(start_response) => match start_response {
                    edgeless_api::common::StartComponentResponse::InstanceId(instance_id) => {
                        self.active_instances.insert(
                            *lid,
                            crate::active_instance::ActiveInstance::Resource(
                                resource_req,
                                edgeless_api::function_instance::InstanceId {
                                    node_id: *node_id,
                                    function_id: instance_id.function_id,
                                },
                            ),
                        );
                        self.active_instances_changed = true;
                        log::info!(
                            "Started resource type {}, node_id {}, lid {}, pid {}",
                            class_type,
                            node_id,
                            &lid,
                            instance_id.function_id
                        );
                        Ok(edgeless_api::common::StartComponentResponse::InstanceId(*lid))
                    }
                    edgeless_api::common::StartComponentResponse::ResponseError(err) => {
                        Ok(edgeless_api::common::StartComponentResponse::ResponseError(err))
                    }
                },
                Err(err) => Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                    edgeless_api::common::ResponseError {
                        summary: "could not start resource".to_string(),
                        detail: Some(format!("resource type {}, node_id {}, lid {}: {}", class_type, node_id, &lid, err)),
                    },
                )),
            },
            None => Err(anyhow::anyhow!("Resource client missing for node_id {}", node_id)),
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
                self.apply_patches(self.dependencies(&lid), None).await;
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
                        node_id,
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
        let mut error_messages = Vec::new();
        for (other_node_id, client_desc) in self.nodes.iter_mut() {
            if other_node_id.eq(&node_id) {
                continue;
            }
            if let Err(err) = new_node_client
                .update_peers(edgeless_api::node_management::UpdatePeersRequest::Add(
                    *other_node_id,
                    client_desc.invocation_url.clone(),
                ))
                .await
            {
                num_failures += 1;
                // we want to collect the error_messages
                error_messages.push(format!("node {}: {}", other_node_id, err));
            }
        }

        if num_failures > 0 {
            log::error!(
                "There have been failures ({}) when updating the peers following the addition of node '{}', the data plane may not work properly. Errors: [{}]",
                num_failures,
                node_id,
                error_messages.join("; ")
            );
        }
    }

    async fn del_node(&mut self, node_id: uuid::Uuid) {
        // Remove the node from the map of clients.
        log::info!("Removing node '{}'", node_id);
        if self.nodes.remove(&node_id).is_none() {
            log::error!("Cannot delete non-existing node '{}'", node_id);
            return;
        }

        // Remove all the resource providers associated with the node removed.
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

        // Remove the node from all the active instances.
        for (_origin_lid, instance) in self.active_instances.iter_mut() {
            match instance {
                crate::active_instance::ActiveInstance::Function(_start_req, ref mut instances) => {
                    instances.retain(|cur_node_id| node_id != cur_node_id.0.node_id);
                }
                crate::active_instance::ActiveInstance::Resource(_start_req, ref mut instance) => {
                    if instance.node_id == node_id {
                        *instance = edgeless_api::function_instance::InstanceId::none();
                    }
                }
            }
        }
    }

    async fn update_domain(&mut self) {
        // Notify the domain register of the updated capabilities.
        let new_domain_capabilities = self.domain_capabilities();
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

    /// detect failures in critical functions and prepare for fast-path failover.
    /// this method only iterates over functions with replication_factor set,
    /// making it efficient for the time-critical KPI-13 path.
    ///
    /// returns information needed for immediate repatching and background replica creation.
    fn detect_critical_failures(&mut self) -> CriticalFailoverResult {
        let mut to_repatch: Vec<uuid::Uuid> = Vec::new();
        let mut replicas_to_create: Vec<NewReplicaRequest> = Vec::new();
        let mut workflows_to_stop: Vec<String> = Vec::new();

        // only iterate over critical functions (those with replication_factor)
        for lid in self.critical_lids.iter() {
            let instance = match self.active_instances.get_mut(lid) {
                Some(i) => i,
                None => continue,
            };

            let (spawn_req, instances) = match instance {
                crate::active_instance::ActiveInstance::Function(req, inst) => (req, inst),
                _ => continue,
            };

            let replicas = match spawn_req.replication_factor {
                Some(r) => r,
                None => continue, // shouldn't happen since we're iterating critical_lids
            };

            let num_disconnected = instances.iter().filter(|x| !self.nodes.contains_key(&x.0.node_id)).count();

            // all replicas healthy, nothing to do
            if num_disconnected == 0 && instances.len() == replicas as usize {
                continue;
            }

            self.active_instances_changed = true;

            // remove disconnected instances
            instances.retain(|x| self.nodes.contains_key(&x.0.node_id));

            // check if the active replica died
            let has_active = instances.iter().any(|x| x.1);
            let hot_standby_available = instances.iter().any(|x| !x.1);

            if !has_active {
                log::info!("active replica died for lid {}, attempting graceful failover", lid);

                if hot_standby_available {
                    // promote hot-standby to active (fast path)
                    if let Some((_instance_id, is_active)) = instances.iter_mut().find(|x| !x.1) {
                        log::info!("promoting hot-standby to active for lid {}", lid);
                        *is_active = true;
                        to_repatch.push(*lid);
                    }
                } else {
                    // no hot-standby available - stop the workflow, as the kpi-13 cannot be guaranteed
                    log::error!(
                        "kpi-13 not possible: no hot-standby for lid {}, stopping workflow '{}'",
                        lid,
                        spawn_req.workflow_id
                    );
                    workflows_to_stop.push(spawn_req.workflow_id.clone());
                }
            } else if num_disconnected > 0 {
                // some hot-standby replicas died but active is fine
                // the function itself needs repatching
                to_repatch.push(*lid);
            }

            // queue new replicas to maintain replication factor
            let missing_replicas = (replicas as usize).saturating_sub(instances.len());
            for _ in 0..missing_replicas {
                log::info!("scheduling new replica for lid {} to maintain replication factor", lid);
                replicas_to_create.push(NewReplicaRequest {
                    lid: *lid,
                    spawn_req: spawn_req.clone(),
                });
            }
        }

        CriticalFailoverResult {
            to_repatch,
            replicas_to_create,
            workflows_to_stop,
        }
    }

    /// detect failures in non-critical instances (resources, non-replicated functions).
    /// also finds dependencies that need repatching due to critical function changes.
    fn detect_non_critical_failures(
        &mut self,
        critical_result: &CriticalFailoverResult,
    ) -> (
        Vec<uuid::Uuid>,                                                                        // to_repatch
        Vec<(uuid::Uuid, edgeless_api::resource_configuration::ResourceInstanceSpecification)>, // resources_to_create
        Vec<String>,                                                                            // workflows_to_stop (non-replicated function died)
    ) {
        let mut to_repatch: Vec<uuid::Uuid> = Vec::new();
        let mut resources_to_create = Vec::new();
        let mut workflows_to_stop: Vec<String> = Vec::new();
        let mut lids_with_new_instances: std::collections::HashSet<uuid::Uuid> = std::collections::HashSet::new();

        // track lids from critical path that will have new replicas
        for req in &critical_result.replicas_to_create {
            lids_with_new_instances.insert(req.lid);
        }

        // scan non-critical active instances
        for (lid, instance) in self.active_instances.iter_mut() {
            // skip critical functions (already handled)
            if self.critical_lids.contains(lid) {
                // but check if critical function lost all instances (no hot-standby case)
                if let crate::active_instance::ActiveInstance::Function(spawn_req, instances) = instance {
                    if spawn_req.replication_factor.is_some() && instances.is_empty() {
                        to_repatch.push(*lid);
                        lids_with_new_instances.insert(*lid);
                    }
                }
                continue;
            }

            match instance {
                crate::active_instance::ActiveInstance::Function(spawn_req, instances) => {
                    // non-replicated function (replication_factor is None)
                    if spawn_req.replication_factor.is_none()
                        && (instances.is_empty() || instances.iter().all(|x| !self.nodes.contains_key(&x.0.node_id))) {
                            log::error!(
                                "non-replicated function lid {} has died, stopping workflow '{}'",
                                lid,
                                spawn_req.workflow_id
                            );
                            workflows_to_stop.push(spawn_req.workflow_id.clone());
                        }
                }
                crate::active_instance::ActiveInstance::Resource(spec, instance) => {
                    if instance.is_none() || !self.nodes.contains_key(&instance.node_id) {
                        to_repatch.push(*lid);
                        resources_to_create.push((*lid, spec.clone()));
                        lids_with_new_instances.insert(*lid);
                    }
                }
            }
        }

        // find all functions that depend on modified lids
        for (origin_lid, output_mapping) in self.dependency_graph.iter() {
            for target_lid in output_mapping.values() {
                if lids_with_new_instances.contains(target_lid) {
                    to_repatch.push(*origin_lid);
                }
            }
        }

        // deduplicate and remove any that are already in critical path
        to_repatch.sort();
        to_repatch.dedup();
        let critical_set: std::collections::HashSet<_> = critical_result.to_repatch.iter().copied().collect();
        to_repatch.retain(|lid| !critical_set.contains(lid));

        // deduplicate workflows to stop
        workflows_to_stop.sort();
        workflows_to_stop.dedup();

        (to_repatch, resources_to_create, workflows_to_stop)
    }

    async fn refresh(&mut self, parent_span: Option<&TraceSpan>) {
        let refresh_span = parent_span.map(|s| s.child("refresh"));
        let kpi_13_span = refresh_span.as_ref().map(|s| s.child("kpi_13_failover"));

        // 1. start with critical functions first (KPI13 requirement)
        let critical_result = self.detect_critical_failures();

        // precompute critical patches - this is the "decision" point
        let critical_patches = self.precompute_patches(&critical_result.to_repatch);

        // end kpi-13 span now - failover decision is made, patches are ready to send
        if let Some(span) = kpi_13_span {
            span.end();
        }

        // send critical patches (I/O operation, not measured in kpi-13)
        if !critical_patches.is_empty() {
            log::info!("fast path: repatching {} critical functions", critical_patches.len());
            for patch in critical_patches {
                Self::send_patch(&mut self.nodes, patch).await;
            }
        }

        // stop workflows that failed KPI-13 (no hot-standby was available)
        if !critical_result.workflows_to_stop.is_empty() {
            log::warn!("stopping {} workflows due to KPI-13 failure", critical_result.workflows_to_stop.len());
            let lids_to_stop: Vec<uuid::Uuid> = self
                .active_instances
                .iter()
                .filter_map(|(lid, instance)| {
                    if let crate::active_instance::ActiveInstance::Function(req, _) = instance {
                        if critical_result.workflows_to_stop.contains(&req.workflow_id) {
                            return Some(*lid);
                        }
                    }
                    None
                })
                .collect();

            for lid in lids_to_stop {
                log::info!("stopping function lid {} due to workflow KPI-13 failure", lid);
                self.stop_function_lid(lid).await;
            }
        }

        // 2. now handle the non-critical functions
        let (mut non_critical_to_repatch, resources_to_create, non_critical_workflows_to_stop) = self.detect_non_critical_failures(&critical_result);

        // stop workflows where non-replicated functions died
        if !non_critical_workflows_to_stop.is_empty() {
            log::warn!(
                "stopping {} workflows due to non-replicated function failure",
                non_critical_workflows_to_stop.len()
            );
            let lids_to_stop: Vec<uuid::Uuid> = self
                .active_instances
                .iter()
                .filter_map(|(lid, instance)| {
                    match instance {
                        crate::active_instance::ActiveInstance::Function(req, _) => {
                            if non_critical_workflows_to_stop.contains(&req.workflow_id) {
                                return Some(*lid);
                            }
                        }
                        crate::active_instance::ActiveInstance::Resource(req, _) => {
                            if non_critical_workflows_to_stop.contains(&req.workflow_id) {
                                return Some(*lid);
                            }
                        }
                    }
                    None
                })
                .collect();

            for lid in lids_to_stop {
                log::info!("stopping component lid {} due to non-replicated function failure in workflow", lid);
                self.stop_function_lid(lid).await;
            }
        }

        // create new function replicas (to maintain replication factor)
        for req in critical_result.replicas_to_create {
            match self.select_node(&req.spawn_req) {
                Ok(node_id) => {
                    log::info!("creating new replica for lid {} on node {}", req.lid, node_id);
                    if let Err(err) = self.start_function_in_node(&req.spawn_req, &req.lid, &node_id).await {
                        log::error!("failed to create replica for lid {}: {}", req.lid, err);
                        if let Some(crate::active_instance::ActiveInstance::Function(_, instances)) = self.active_instances.get_mut(&req.lid) {
                            instances.clear();
                            self.active_instances_changed = true;
                        }
                    }
                }
                Err(err) => {
                    log::error!("no suitable node for replica lid {}: {}", req.lid, err);
                }
            }
        }

        // create missing resources
        for (lid, spec) in resources_to_create {
            if let Err(err) = self.start_resource(spec, lid).await {
                log::error!("failed to create resource lid {}: {}", lid, err);
                if let Some(crate::active_instance::ActiveInstance::Resource(_, instance_id)) = self.active_instances.get_mut(&lid) {
                    *instance_id = edgeless_api::function_instance::InstanceId::none();
                    self.active_instances_changed = true;
                }
            }
        }

        // handle deploy intents from proxy
        let deploy_intents = self.proxy.lock().await.retrieve_deploy_intents();
        let mut cordoned_uncordoned_nodes = false;

        for intent in deploy_intents {
            match intent {
                crate::deploy_intent::DeployIntent::Migrate(lid, targets) => {
                    match self.migrate(&lid, &targets).await {
                        Err(err) => log::warn!("migration of '{}' declined: {}", lid, err),
                        Ok(target_node_id) => {
                            log::info!("migrated '{}' to '{}'", lid, target_node_id);
                            non_critical_to_repatch.push(lid);

                            // also repatch dependents
                            for (origin_lid, output_mapping) in self.dependency_graph.iter() {
                                if output_mapping.values().contains(&lid) {
                                    non_critical_to_repatch.push(*origin_lid);
                                }
                            }
                        }
                    }
                }
                crate::deploy_intent::DeployIntent::Cordon(node_id) => {
                    if let Some(desc) = self.nodes.get_mut(&node_id) {
                        desc.cordoned = true;
                        cordoned_uncordoned_nodes = true;
                    } else {
                        log::warn!("cordon unknown node '{}' ignored", node_id);
                    }
                }
                crate::deploy_intent::DeployIntent::Uncordon(node_id) => {
                    if let Some(desc) = self.nodes.get_mut(&node_id) {
                        desc.cordoned = false;
                        cordoned_uncordoned_nodes = true;
                    } else {
                        log::warn!("uncordon unknown node '{}' ignored", node_id);
                    }
                }
            }
        }

        if cordoned_uncordoned_nodes {
            self.orchestration_logic.update_nodes(&self.nodes, &self.resource_providers);
        }

        // apply non-critical patches (resources, migrations, dependencies of new replicas)
        if !non_critical_to_repatch.is_empty() {
            non_critical_to_repatch.sort();
            non_critical_to_repatch.dedup();
            self.apply_patches(non_critical_to_repatch, refresh_span.as_ref()).await;
        }

        // update proxy state
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

    async fn reset(&mut self) {
        log::info!("Resetting the orchestration domain to a clean state");
        let mut function_lids = vec![];
        let mut resource_lids = vec![];
        for (lid, active_instance) in &self.active_instances {
            match active_instance {
                ActiveInstance::Function(_, _) => function_lids.push(*lid),
                ActiveInstance::Resource(_, _) => resource_lids.push(*lid),
            }
        }
        for lid in function_lids {
            self.stop_function_lid(lid).await;
        }
        for lid in resource_lids {
            self.stop_resource_lid(lid).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_two_functions_then_stop_one() {
        // channels
        let (_tx, rx) = futures::channel::mpsc::unbounded();
        let (subscriber_tx, _subscriber_rx) = futures::channel::mpsc::unbounded();

        // mock proxy expectations setting
        let mut mock_proxy = crate::proxy::MockProxy::new();
        mock_proxy.expect_update_nodes().returning(|_| ());
        mock_proxy.expect_update_resource_providers().returning(|_| ());
        mock_proxy.expect_update_active_instances().returning(|_| ());
        mock_proxy.expect_update_dependency_graph().returning(|_| ());
        mock_proxy.expect_retrieve_deploy_intents().returning(|| vec![]);

        // mock AgentAPI expectations
        let mut mock_agent_api = edgeless_api::outer::agent::MockAgentAPI::new();
        mock_agent_api.expect_node_management_api().returning(|| {
            let mut mock_node_mgmt_api = edgeless_api::node_management::MockNodeManagementAPI::new();
            mock_node_mgmt_api.expect_update_peers().returning(|_| Ok(()));
            Box::new(mock_node_mgmt_api)
        });

        let proxy = std::sync::Arc::new(tokio::sync::Mutex::new(mock_proxy));

        let settings = crate::EdgelessOrcBaselineSettings {
            orchestration_strategy: crate::OrchestrationStrategy::RoundRobin,
        };

        let mut _orchestrator = OrchestratorTask::new(rx, settings, proxy.clone(), subscriber_tx).await;

        // Add three nodes
        let node_1_id = uuid::Uuid::new_v4();
        let client_desc = crate::client_desc::ClientDesc {
            agent_url: "http://node1/agent".to_string(),
            invocation_url: "http://node1".to_string(),
            capabilities: edgeless_api::node_registration::NodeCapabilities::default(),
            cordoned: false,
            api: Box::new(mock_agent_api),
        };
        let resource_providers = vec![];
        _orchestrator.add_node(node_1_id, client_desc, resource_providers).await;
        // test if it works
        assert_eq!(1, 1);
    }
}
