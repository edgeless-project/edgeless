// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use serde::ser::{Serialize, SerializeTupleVariant, Serializer};

use futures::{Future, SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::str::FromStr;

#[cfg(test)]
pub mod test;

#[derive(PartialEq, Debug, Clone)]
pub enum AffinityLevel {
    Required,
    NotRequired,
}

impl std::fmt::Display for AffinityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AffinityLevel::Required => "required",
                AffinityLevel::NotRequired => "not-required",
            }
        )
    }
}

impl AffinityLevel {
    pub fn from_string(val: &str) -> Self {
        if val.to_lowercase() == "required" {
            AffinityLevel::Required
        } else {
            AffinityLevel::NotRequired
        }
    }
}

/// Intent to update/change deployment.
pub enum DeployIntent {
    /// The component with givel logical identifier should be migrated to
    /// the given target nodes, if possible.
    Migrate(edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::NodeId>),
}

impl DeployIntent {
    pub fn new(key: &str, value: &str) -> anyhow::Result<Self> {
        let tokens: Vec<&str> = key.split(':').collect();
        assert!(!tokens.is_empty());
        anyhow::ensure!(tokens[0] == "intent", "intent not starting with \"intent\"");
        if tokens.len() >= 2 {
            match tokens[1] {
                "migrate" => {
                    anyhow::ensure!(tokens.len() == 3);
                    let component_id = uuid::Uuid::from_str(tokens[2])?;
                    let mut targets = vec![];
                    for target in value.split(',') {
                        if target.is_empty() {
                            continue;
                        }
                        targets.push(uuid::Uuid::from_str(target)?);
                    }
                    Ok(DeployIntent::Migrate(component_id, targets))
                }
                _ => anyhow::bail!("unknown intent type '{}'", tokens[1]),
            }
        } else {
            anyhow::bail!("ill-formed intent");
        }
    }

    pub fn key(&self) -> String {
        match self {
            Self::Migrate(component, _) => format!("intent:migrate:{}", component),
        }
    }

    pub fn value(&self) -> String {
        match self {
            Self::Migrate(_, targets) => targets.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
        }
    }
}

impl std::fmt::Display for DeployIntent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DeployIntent::Migrate(component, target) => write!(
                f,
                "migrate component {} to [{}]",
                component,
                target.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")
            ),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
/// Deployment requirements for functions, as specified by annotations.
pub struct DeploymentRequirements {
    /// Maximum number of function instances in this orchestration domain.
    /// 0 means unlimited.
    pub max_instances: usize,
    /// The function instance must be created on a node matching one
    /// of the given UUIDs, if any is given.
    pub node_id_match_any: Vec<uuid::Uuid>,
    /// The function instance must be created on a node that matches all
    /// the labels specified, if any is given.
    pub label_match_all: Vec<String>,
    /// The function instance must be created on a node that hosts all the
    /// resources providers specified, if any is given.
    pub resource_match_all: Vec<String>,
    /// Function instance's node affinity with Trusted Execution Environment.
    pub tee: AffinityLevel,
    /// Function instance's node affinity with Trusted Platform Module.
    pub tpm: AffinityLevel,
}

impl std::fmt::Display for DeploymentRequirements {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "max_instances {}, node_id_match_any {}, label_match_all {}, resource_match_all {}, tee {}, tpm {}",
            self.max_instances,
            self.node_id_match_any.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
            self.label_match_all.join(","),
            self.resource_match_all.join(","),
            self.tee,
            self.tpm
        )
    }
}

impl DeploymentRequirements {
    #[cfg(test)]
    /// No specific deployment requirements.
    pub fn none() -> Self {
        Self {
            max_instances: 0,
            node_id_match_any: vec![],
            label_match_all: vec![],
            resource_match_all: vec![],
            tee: AffinityLevel::NotRequired,
            tpm: AffinityLevel::NotRequired,
        }
    }
    /// Deployment requirements from the annotations in the function's spawn request.
    pub fn from_annotations(annotations: &std::collections::HashMap<String, String>) -> Self {
        let mut max_instances = 0;
        if let Some(val) = annotations.get("max_instances") {
            max_instances = val.parse::<usize>().unwrap_or_default();
        }

        let mut node_id_match_any = vec![];
        if let Some(val) = annotations.get("node_id_match_any") {
            node_id_match_any = val.split(",").filter_map(|x| uuid::Uuid::parse_str(x).ok()).collect();
        }

        let mut label_match_all = vec![];
        if let Some(val) = annotations.get("label_match_all") {
            label_match_all = val.split(",").map(|x| x.to_string()).collect();
        }

        let mut resource_match_all = vec![];
        if let Some(val) = annotations.get("resource_match_all") {
            resource_match_all = val.split(",").map(|x| x.to_string()).collect();
        }

        let mut tee = AffinityLevel::NotRequired;
        if let Some(val) = annotations.get("tee") {
            tee = AffinityLevel::from_string(val);
        }

        let mut tpm = AffinityLevel::NotRequired;
        if let Some(val) = annotations.get("tpm") {
            tpm = AffinityLevel::from_string(val);
        }

        Self {
            max_instances,
            node_id_match_any,
            label_match_all,
            resource_match_all,
            tee,
            tpm,
        }
    }
}

pub struct Orchestrator {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

enum OrchestratorRequest {
    StartFunction(
        edgeless_api::function_instance::SpawnFunctionRequest,
        tokio::sync::oneshot::Sender<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>>,
        >,
    ),
    StopFunction(edgeless_api::function_instance::DomainManagedInstanceId),
    StartResource(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        tokio::sync::oneshot::Sender<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>>,
        >,
    ),
    StopResource(edgeless_api::function_instance::DomainManagedInstanceId),
    Patch(edgeless_api::common::PatchRequest),
    UpdateNode(
        edgeless_api::node_registration::UpdateNodeRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse>>,
    ),
    KeepAlive(),
}

#[derive(serde::Serialize)]
pub struct ResourceProvider {
    pub class_type: String,
    pub node_id: edgeless_api::function_instance::NodeId,
    pub outputs: Vec<String>,
}

#[derive(Clone)]
pub enum ActiveInstance {
    // 0: request
    // 1: [ (node_id, int_fid) ]
    Function(
        edgeless_api::function_instance::SpawnFunctionRequest,
        Vec<edgeless_api::function_instance::InstanceId>,
    ),

    // 0: request
    // 1: (node_id, int_fid)
    Resource(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        edgeless_api::function_instance::InstanceId,
    ),
}

impl ActiveInstance {
    pub fn instance_ids(&self) -> Vec<edgeless_api::function_instance::InstanceId> {
        match self {
            ActiveInstance::Function(_, ids) => ids.clone(),
            ActiveInstance::Resource(_, id) => vec![*id],
        }
    }
}

impl Serialize for ActiveInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ActiveInstance::Function(ref req, ref ids) => {
                let mut tv = serializer.serialize_tuple_variant("ActiveInstance", 0, "Function", 2)?;
                tv.serialize_field(req)?;
                tv.serialize_field::<Vec<String>>(ids.iter().map(|x| x.to_string()).collect::<Vec<String>>().as_ref())?;
                tv.end()
            }
            ActiveInstance::Resource(ref req, ref id) => {
                let mut tv = serializer.serialize_tuple_variant("ActiveInstance", 1, "Resource", 2)?;
                tv.serialize_field(req)?;
                tv.serialize_field(id.to_string().as_str())?;
                tv.end()
            }
        }
    }
}

impl std::fmt::Display for ActiveInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ActiveInstance::Function(_req, instances) => write!(
                f,
                "function, instances {}",
                instances
                    .iter()
                    .map(|x| format!("node_id {}, int_fid {}", x.node_id, x.function_id))
                    .collect::<Vec<String>>()
                    .join(",")
            ),
            ActiveInstance::Resource(req, instance_id) => write!(
                f,
                "resource class type {}, node_id {}, function_id {}",
                req.class_type, instance_id.node_id, instance_id.function_id
            ),
        }
    }
}

impl std::fmt::Display for ResourceProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "class_type {}, node_id {}, outputs [{}]",
            self.class_type,
            self.node_id,
            self.outputs.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
        )
    }
}

pub struct OrchestratorClient {
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
    node_registration_client: Box<dyn edgeless_api::node_registration::NodeRegistrationAPI>,
    resource_configuration_client:
        Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
}

impl edgeless_api::api::orc::OrchestratorAPI for OrchestratorClient {
    fn function_instance_api(
        &mut self,
    ) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::DomainManagedInstanceId>> {
        self.function_instance_client.clone()
    }

    fn node_registration_api(&mut self) -> Box<dyn edgeless_api::node_registration::NodeRegistrationAPI> {
        self.node_registration_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::DomainManagedInstanceId>> {
        self.resource_configuration_client.clone()
    }
}

#[derive(Clone)]
pub struct OrchestratorFunctionInstanceOrcClient {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

#[derive(Clone)]
pub struct NodeRegistrationClient {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

#[derive(Clone)]
pub struct ResourceConfigurationClient {
    sender: futures::channel::mpsc::UnboundedSender<OrchestratorRequest>,
}

impl OrchestratorFunctionInstanceOrcClient {}

pub struct ClientDesc {
    pub agent_url: String,
    pub invocation_url: String,
    pub api: Box<dyn edgeless_api::api::agent::AgentAPI + Send>,
    pub capabilities: edgeless_api::node_registration::NodeCapabilities,
}

enum IntFid {
    // 0: node_id, int_fid
    Function(edgeless_api::function_instance::InstanceId),
    // 0: node_id, int_fid
    Resource(edgeless_api::function_instance::InstanceId),
}

impl IntFid {
    fn instance_id(&self) -> edgeless_api::function_instance::InstanceId {
        match self {
            Self::Function(id) => *id,
            Self::Resource(id) => *id,
        }
    }
}

impl Orchestrator {
    pub async fn new(
        settings: crate::EdgelessOrcBaselineSettings,
        proxy: Box<dyn super::proxy::Proxy>,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            Self::main_task(
                receiver,
                settings,
                std::collections::HashMap::new(),
                std::collections::HashMap::new(),
                proxy,
            )
            .await;
        });

        (Orchestrator { sender }, main_task)
    }

    #[cfg(test)]
    pub async fn new_with_clients(
        settings: crate::EdgelessOrcBaselineSettings,
        clients: std::collections::HashMap<uuid::Uuid, ClientDesc>,
        resource_providers: std::collections::HashMap<String, ResourceProvider>,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            Self::main_task(receiver, settings, clients, resource_providers, Box::new(super::proxy_none::ProxyNone {})).await;
        });

        (Orchestrator { sender }, main_task)
    }

    pub async fn keep_alive(&mut self) {
        let _ = self.sender.send(OrchestratorRequest::KeepAlive()).await;
    }

    fn ext_to_int(
        active_instances: &std::collections::HashMap<edgeless_api::function_instance::ComponentId, ActiveInstance>,
        ext_fid: &edgeless_api::function_instance::ComponentId,
    ) -> Vec<IntFid> {
        match active_instances.get(ext_fid) {
            Some(active_instance) => match active_instance {
                ActiveInstance::Function(_req, instances) => instances
                    .iter()
                    .map(|x| {
                        IntFid::Function(edgeless_api::function_instance::InstanceId {
                            node_id: x.node_id,
                            function_id: x.function_id,
                        })
                    })
                    .collect(),
                ActiveInstance::Resource(_req, instance) => {
                    vec![IntFid::Resource(edgeless_api::function_instance::InstanceId {
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
    /// * `active_instances` - The set of active instances, resources/functions.
    /// * `clients` - The nodes' descriptors.
    /// * `orchestration_logic` - The baseline orchestration logic.
    /// * `component` - The logical identifier of the function/resource to be
    ///   migrated.
    /// * `targets` - The set of nodes to which the instance has to be migrated.
    async fn migrate(
        active_instances: &mut std::collections::HashMap<edgeless_api::function_instance::ComponentId, ActiveInstance>,
        clients: &mut std::collections::HashMap<uuid::Uuid, ClientDesc>,
        orchestration_logic: &crate::orchestration_logic::OrchestrationLogic,
        component: &edgeless_api::function_instance::ComponentId,
        targets: &Vec<edgeless_api::function_instance::NodeId>,
    ) {
        let mut to_be_started = vec![];
        match active_instances.get_mut(component) {
            Some(active_instance) => match active_instance {
                ActiveInstance::Function(spawn_req, origins) => {
                    // Stop the origin nodes.
                    for origin in origins.drain(..) {
                        Self::stop_function(clients, &origin).await;
                    }

                    // Filter out the unfeasible targets.
                    let targets = orchestration_logic.feasible_nodes(spawn_req, targets);

                    let target = targets.first();
                    if let Some(target) = target {
                        if targets.len() > 1 {
                            log::warn!(
                                "Currently supporting only a single target node per component: choosing {}, the others will be ignored",
                                target
                            );
                        }
                        to_be_started.push((spawn_req.clone(), *target));
                    } else {
                        log::warn!("No (valid) target found for the migration of function ext_fid {}", component);
                    }
                }
                ActiveInstance::Resource(_spec, origin) => log::warn!(
                    "Currently not supporting the migration of resources: ignoring request for ext_fid {} to migrate from node_id {}",
                    component,
                    origin
                ),
            },
            None => log::warn!("Intent to migrate component {} that is not active: ignored", component),
        }
        for element in to_be_started {
            match Self::start_function(&element.0, active_instances, clients, component, &element.1).await {
                Ok(_) => {}
                Err(err) => log::error!("Error when migrating function ext_id {} to node_id {}: {}", component, element.1, err),
            }
        }
    }

    /// Apply patches on node's run-time agents.
    ///
    /// * `active_instances` - The set of active instances, resources/functions.
    /// * `dependency_graph` - The logical dependencies.
    /// * `clients` - The nodes' descriptor.s
    /// * `origin_ext_fids` - The logical resource identifiers for which patches
    ///    must be applied.
    async fn apply_patches(
        active_instances: &std::collections::HashMap<edgeless_api::function_instance::ComponentId, ActiveInstance>,
        dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>,
        clients: &mut std::collections::HashMap<uuid::Uuid, ClientDesc>,
        origin_ext_fids: Vec<edgeless_api::function_instance::ComponentId>,
    ) {
        for origin_ext_fid in origin_ext_fids.iter() {
            let ext_output_mapping = match dependency_graph.get(origin_ext_fid) {
                Some(x) => x,
                None => continue,
            };

            // Transform the external function identifiers into
            // internal ones.
            for source in Self::ext_to_int(active_instances, origin_ext_fid) {
                let mut int_output_mapping = std::collections::HashMap::new();
                for (channel, target_ext_fid) in ext_output_mapping {
                    for target in Self::ext_to_int(active_instances, target_ext_fid) {
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
                    IntFid::Function(instance_id) => match clients.get_mut(&instance_id.node_id) {
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
                                log::info!("Patched node_id {} int_fid {}", instance_id.node_id, instance_id.function_id);
                            }
                            Err(err) => {
                                log::error!(
                                    "Error when patching node_id {} int_fid {}: {}",
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
                    IntFid::Resource(instance_id) => match clients.get_mut(&instance_id.node_id) {
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
                                log::info!("Patched provider node_id {} int_fid {}", instance_id.node_id, instance_id.function_id);
                            }
                            Err(err) => {
                                log::error!(
                                    "Error when patching provider node_id {} int_fid {}: {}",
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

    async fn start_resource(
        start_req: edgeless_api::resource_configuration::ResourceInstanceSpecification,
        resource_providers: &mut std::collections::HashMap<String, ResourceProvider>,
        active_instances: &mut std::collections::HashMap<edgeless_api::function_instance::ComponentId, ActiveInstance>,
        clients: &mut std::collections::HashMap<uuid::Uuid, ClientDesc>,
        ext_fid: uuid::Uuid,
        rng: &mut rand::rngs::StdRng,
    ) -> Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>, anyhow::Error> {
        // Find all resource providers that can start this resource.
        let matching_providers = resource_providers
            .iter()
            .filter_map(|(id, p)| if p.class_type == start_req.class_type { Some(id.clone()) } else { None })
            .collect::<Vec<String>>();

        // Select one provider at random.
        match matching_providers.choose(rng) {
            Some(provider_id) => {
                let resource_provider = resource_providers.get_mut(provider_id).unwrap();
                match clients.get_mut(&resource_provider.node_id) {
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
                                active_instances.insert(
                                    ext_fid,
                                    ActiveInstance::Resource(
                                        start_req,
                                        edgeless_api::function_instance::InstanceId {
                                            node_id: resource_provider.node_id,
                                            function_id: instance_id.function_id,
                                        },
                                    ),
                                );
                                log::info!(
                                    "Started resource provider_id {}, node_id {}, ext_fid {}, int_fid {}",
                                    provider_id,
                                    resource_provider.node_id,
                                    &ext_fid,
                                    instance_id.function_id
                                );
                                Ok(edgeless_api::common::StartComponentResponse::InstanceId(ext_fid))
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
        spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest,
        orchestration_logic: &mut crate::orchestration_logic::OrchestrationLogic,
    ) -> anyhow::Result<edgeless_api::function_instance::NodeId> {
        match orchestration_logic.next(spawn_req) {
            Some(node_id) => Ok(node_id),
            None => Err(anyhow::anyhow!("no valid node found")),
        }
    }

    /// Start a new function instance on a specific node.
    ///
    /// If the operation fails, then active_instances is not
    /// updated, i.e., it is as if the request to start the
    /// function has never been issued.
    ///
    /// * `spawn_req` - The specifications of the function.
    /// * `active_instances` - The set of active instances, resources/functions.
    /// * `dependency_graph` - The logical dependencies.
    /// * `clients` - The nodes' descriptors.
    /// * `ext_fid` - The logical identifier of the function.
    /// * `node_id` - The node where to deploy the function instance.
    async fn start_function(
        spawn_req: &edgeless_api::function_instance::SpawnFunctionRequest,
        active_instances: &mut std::collections::HashMap<edgeless_api::function_instance::ComponentId, ActiveInstance>,
        clients: &mut std::collections::HashMap<uuid::Uuid, ClientDesc>,
        ext_fid: &uuid::Uuid,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>, anyhow::Error> {
        let mut fn_client = match clients.get_mut(node_id) {
            Some(c) => c,
            None => panic!(
                "Invalid node_id {} selected by the orchestration logic when starting function instance ext_fid {}",
                node_id, ext_fid
            ),
        }
        .api
        .function_instance_api();

        log::debug!(
            "Orchestrator StartFunction {:?} ext_fid {} at worker node with node_id {:?}",
            spawn_req,
            ext_fid,
            node_id
        );

        // Finally try to spawn the function instance on the
        // selected client.
        // [TODO] Issue#96 We assume that one instance is spawned.
        match fn_client.start(spawn_req.clone()).await {
            Ok(res) => match res {
                edgeless_api::common::StartComponentResponse::ResponseError(err) => {
                    Err(anyhow::anyhow!("Could not start a function instance for ext_fid {}: {}", ext_fid, err))
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    assert!(*node_id == id.node_id);
                    active_instances.insert(
                        *ext_fid,
                        ActiveInstance::Function(
                            spawn_req.clone(),
                            vec![edgeless_api::function_instance::InstanceId {
                                node_id: *node_id,
                                function_id: id.function_id,
                            }],
                        ),
                    );
                    log::info!("Spawned at node_id {}, ext_fid {}, int_fid {}", node_id, &ext_fid, id.function_id);

                    Ok(edgeless_api::common::StartComponentResponse::InstanceId(*ext_fid))
                }
            },
            Err(err) => {
                log::error!("Unhandled: {}", err);
                Err(anyhow::anyhow!("Could not start a function instance for ext_fid {}: {}", ext_fid, err))
            }
        }
    }

    /// Stop a running function instance.
    ///
    /// * `clients` - The nodes' descriptors.
    /// * `instance_id` - The function instance to be stopped.
    async fn stop_function(
        clients: &mut std::collections::HashMap<uuid::Uuid, ClientDesc>,
        instance_id: &edgeless_api::function_instance::InstanceId,
    ) {
        match clients.get_mut(&instance_id.node_id) {
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

    /// Stop a running resource instance.
    ///
    /// * `clients` - The nodes' descriptors.
    /// * `instance_id` - The resource instance to be stopped.
    async fn stop_resource(
        clients: &mut std::collections::HashMap<uuid::Uuid, ClientDesc>,
        instance_id: &edgeless_api::function_instance::InstanceId,
    ) {
        match clients.get_mut(&instance_id.node_id) {
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

    /// Return the list of ext_fids that depend on the given one, according
    /// to the active patches.
    ///
    /// If we see the functions and output_mappings as a graph where:
    /// - there is a vertex for every function/resource,
    /// - there is an edge for every output_mapping between two functions/resources
    ///
    /// this function will return all the ingress vertices of the vertex
    /// identified by `ext_fid`.
    fn dependencies(
        dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>,
        ext_fid: &uuid::Uuid,
    ) -> Vec<uuid::Uuid> {
        let mut dependencies = vec![];
        for (origin_ext_fid, output_mapping) in dependency_graph.iter() {
            for (_output, target_ext_fid) in output_mapping.iter() {
                if target_ext_fid == ext_fid {
                    dependencies.push(*origin_ext_fid);
                    break;
                }
            }
        }
        dependencies
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<OrchestratorRequest>,
        orchestrator_settings: crate::EdgelessOrcBaselineSettings,
        nodes: std::collections::HashMap<uuid::Uuid, ClientDesc>,
        resource_providers: std::collections::HashMap<String, ResourceProvider>,
        mut proxy: Box<dyn super::proxy::Proxy>,
    ) {
        let mut receiver = receiver;
        let mut orchestration_logic = crate::orchestration_logic::OrchestrationLogic::new(orchestrator_settings.orchestration_strategy);
        let mut rng = rand::rngs::StdRng::from_entropy();

        // known nodes
        // key: node_id
        let mut nodes = nodes;
        orchestration_logic.update_nodes(&nodes, &resource_providers);
        for (node_id, client_desc) in &nodes {
            log::info!(
                "added function instance client: node_id {}, agent URL {}, invocation URL {}, capabilities {}",
                node_id,
                client_desc.agent_url,
                client_desc.invocation_url,
                client_desc.capabilities
            );
        }
        proxy.update_nodes(&nodes);

        // known resources providers as advertised by the nodes upon registration
        // key: provider_id
        let mut resource_providers = resource_providers;
        for (provider, resource_provider) in &resource_providers {
            log::info!(
                "added resource: provider {}, class_type {}, node_id {}, outputs [{}]",
                provider,
                resource_provider.class_type,
                resource_provider.node_id,
                resource_provider.outputs.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")
            );
        }
        let mut resource_providers_changed = false;

        // instances that the orchestrator promises to keep active
        // key: ext_fid
        let mut active_instances = std::collections::HashMap::new();
        let mut active_instances_changed = false;

        // active patches to which the orchestrator commits
        // key:   ext_fid (origin function)
        // value: map of:
        //        key:   channel output name
        //        value: ext_fid (target function)
        let mut dependency_graph = std::collections::HashMap::new();
        let mut dependency_graph_changed = false;

        // main orchestration loop that reacts to events on the receiver channel
        while let Some(req) = receiver.next().await {
            match req {
                OrchestratorRequest::StartFunction(spawn_req, reply_channel) => {
                    // Create a new ext_fid for this resource.
                    let ext_fid = uuid::Uuid::new_v4();

                    // Select the target node.
                    match Self::select_node(&spawn_req, &mut orchestration_logic) {
                        Ok(node_id) => {
                            // Start the function instance.
                            let res = Self::start_function(&spawn_req, &mut active_instances, &mut nodes, &ext_fid, &node_id).await;

                            // Send back the response to the caller.
                            if let Err(err) = reply_channel.send(res) {
                                log::error!("Orchestrator channel error in SPAWN: {:?}", err);
                            }

                            active_instances_changed = true;
                        }
                        Err(err) => log::warn!("Could not start function ext_fid {}: {}", ext_fid, err),
                    }
                }
                OrchestratorRequest::StopFunction(ext_fid) => {
                    log::debug!("Orchestrator StopFunction {:?}", ext_fid);

                    match active_instances.remove(&ext_fid) {
                        Some(active_instance) => {
                            match active_instance {
                                ActiveInstance::Function(_req, instances) => {
                                    // Stop all the instances of this function.
                                    for instance_id in instances {
                                        Self::stop_function(&mut nodes, &instance_id).await;
                                    }
                                }
                                ActiveInstance::Resource(_, _) => {
                                    log::error!(
                                        "Request to stop a function but the ext_fid is associated with a resource: ext_fid {}",
                                        ext_fid
                                    );
                                }
                            };
                            Self::apply_patches(
                                &active_instances,
                                &dependency_graph,
                                &mut nodes,
                                Self::dependencies(&dependency_graph, &ext_fid),
                            )
                            .await;
                            dependency_graph.remove(&ext_fid);
                            dependency_graph_changed = true;
                        }
                        None => {
                            log::error!("Request to stop a function that is not known: ext_fid {}", ext_fid);
                        }
                    }

                    active_instances_changed = true;
                }
                OrchestratorRequest::StartResource(start_req, reply_channel) => {
                    log::debug!("Orchestrator StartResource {:?}", &start_req);

                    // Create a new ext_fid for this resource.
                    let ext_fid = uuid::Uuid::new_v4();

                    // Start the resource.
                    // If the operation fails, active_instances is not updated,
                    // i.e., it is as if the request to start the resource has
                    // never been issued.
                    let res = Self::start_resource(
                        start_req.clone(),
                        &mut resource_providers,
                        &mut active_instances,
                        &mut nodes,
                        ext_fid,
                        &mut rng,
                    )
                    .await;

                    // Send back the response to the caller.
                    if let Err(err) = reply_channel.send(res) {
                        log::error!("Orchestrator channel error in STARTRESOURCE: {:?}", err);
                    }

                    active_instances_changed = true;
                }
                OrchestratorRequest::StopResource(ext_fid) => {
                    log::debug!("Orchestrator StopResource {:?}", ext_fid);

                    match active_instances.remove(&ext_fid) {
                        Some(active_instance) => {
                            match active_instance {
                                ActiveInstance::Function(_, _) => {
                                    log::error!(
                                        "Request to stop a resource but the ext_fid is associated with a function: ext_fid {}",
                                        ext_fid
                                    );
                                }
                                ActiveInstance::Resource(_req, instance_id) => {
                                    // Stop the instance of this resource.
                                    Self::stop_resource(&mut nodes, &instance_id).await;
                                }
                            }
                            Self::apply_patches(
                                &active_instances,
                                &dependency_graph,
                                &mut nodes,
                                Self::dependencies(&dependency_graph, &ext_fid),
                            )
                            .await;
                            dependency_graph.remove(&ext_fid);
                            dependency_graph_changed = true;
                        }
                        None => {
                            log::error!("Request to stop a resource that is not known: ext_fid {}", ext_fid);
                        }
                    }

                    active_instances_changed = true;
                }
                OrchestratorRequest::Patch(update) => {
                    log::debug!("Orchestrator Patch {:?}", update);

                    // Extract the ext_fid identifiers for the origin and
                    // target logical functions.
                    let origin_ext_fid = update.function_id;
                    let output_mapping = update
                        .output_mapping
                        .iter()
                        .map(|x| (x.0.clone(), x.1.function_id))
                        .collect::<std::collections::HashMap<String, edgeless_api::function_instance::ComponentId>>();

                    // Save the patch request into an internal data structure,
                    // keeping track only of the ext_fid for both origin
                    // and target (logical) functions.
                    dependency_graph.insert(origin_ext_fid, output_mapping);
                    dependency_graph_changed = true;

                    // Apply the patch.
                    Self::apply_patches(&active_instances, &dependency_graph, &mut nodes, vec![origin_ext_fid]).await;
                }
                OrchestratorRequest::UpdateNode(request, reply_channel) => {
                    // Update the map of clients and, at the same time, prepare
                    // the edgeless_api::node_management::UpdatePeersRequest message to be sent to all the
                    // clients to notify that a new node exists (Register) or
                    // that an existing node left the system (Deregister).
                    let mut this_node_id = None;
                    let msg = match request {
                        edgeless_api::node_registration::UpdateNodeRequest::Registration(
                            node_id,
                            agent_url,
                            invocation_url,
                            resources,
                            capabilities,
                        ) => {
                            let mut dup_entry = false;
                            if let Some(client_desc) = nodes.get(&node_id) {
                                if client_desc.agent_url == agent_url && client_desc.invocation_url == invocation_url {
                                    dup_entry = true;
                                }
                            }
                            if dup_entry {
                                // A client with same node_id, agent_url, and
                                // invocation_url already exists.
                                None
                            } else {
                                this_node_id = Some(node_id);

                                // Create the resource configuration APIs.
                                for resource in &resources {
                                    log::info!("new resource advertised by node {}: {}", this_node_id.unwrap(), resource);

                                    if resource_providers.contains_key(&resource.provider_id) {
                                        log::warn!(
                                            "cannot add resource because another one exists with the same provider_id: {}",
                                            resource.provider_id
                                        )
                                    } else {
                                        assert!(this_node_id.is_some());

                                        resource_providers.insert(
                                            resource.provider_id.clone(),
                                            ResourceProvider {
                                                class_type: resource.class_type.clone(),
                                                node_id: this_node_id.unwrap(),
                                                outputs: resource.outputs.clone(),
                                            },
                                        );
                                        resource_providers_changed = true;
                                    }
                                }

                                // Create the agent API.
                                log::info!(
                                    "added function instance client: node_id {}, agent URL {}, invocation URL {}, capabilities {}",
                                    node_id,
                                    agent_url,
                                    invocation_url,
                                    capabilities
                                );

                                let (proto, host, port) = edgeless_api::util::parse_http_host(&agent_url).unwrap();
                                let api: Box<dyn edgeless_api::api::agent::AgentAPI + Send> = match proto {
                                    edgeless_api::util::Proto::COAP => {
                                        let addr = std::net::SocketAddrV4::new(host.parse().unwrap(), port);
                                        Box::new(edgeless_api::coap_impl::CoapClient::new(addr).await)
                                    }
                                    _ => Box::new(edgeless_api::grpc_impl::agent::AgentAPIClient::new(&agent_url).await),
                                };
                                log::info!("got api");

                                nodes.insert(
                                    node_id,
                                    ClientDesc {
                                        agent_url: agent_url.clone(),
                                        invocation_url: invocation_url.clone(),
                                        api,
                                        capabilities,
                                    },
                                );

                                Some(edgeless_api::node_management::UpdatePeersRequest::Add(node_id, invocation_url))
                            }
                        }
                        edgeless_api::node_registration::UpdateNodeRequest::Deregistration(node_id) => {
                            if !nodes.contains_key(&node_id) {
                                // There is no client with that node_id
                                None
                            } else {
                                nodes.remove(&node_id);
                                Some(edgeless_api::node_management::UpdatePeersRequest::Del(node_id))
                            }
                        }
                    };

                    // If no operation was done (either a new node was already
                    // present with same agent/invocation URLs or a deregistering
                    // node did not exist) we accept the command.
                    let mut response = edgeless_api::node_registration::UpdateNodeResponse::Accepted;

                    if let Some(msg) = msg {
                        // Update the orchestration logic & proxy with the new set of nodes.
                        orchestration_logic.update_nodes(&nodes, &resource_providers);
                        proxy.update_nodes(&nodes);

                        // Update all the peers (including the node, unless it
                        // was a deregister operation).
                        let mut num_failures: u32 = 0;
                        for (_node_id, client) in nodes.iter_mut() {
                            if client.api.node_management_api().update_peers(msg.clone()).await.is_err() {
                                num_failures += 1;
                            }
                        }

                        log::info!("updated peers");

                        // Only with registration, we also update the new node
                        // by adding as peers all the existing nodes.
                        if let Some(this_node_id) = this_node_id {
                            let mut new_node_client = nodes.get_mut(&this_node_id).unwrap().api.node_management_api();
                            for (other_node_id, client_desc) in nodes.iter_mut() {
                                if other_node_id.eq(&this_node_id) {
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
                        }

                        response = match num_failures {
                            0 => edgeless_api::node_registration::UpdateNodeResponse::Accepted,
                            _ => edgeless_api::node_registration::UpdateNodeResponse::ResponseError(edgeless_api::common::ResponseError {
                                summary: "UpdatePeers() failed on some node when updating a node".to_string(),
                                detail: None,
                            }),
                        };
                    }

                    if let Err(err) = reply_channel.send(Ok(response)) {
                        log::error!("Orchestrator channel error in UPDATENODE: {:?}", err);
                    }
                }
                OrchestratorRequest::KeepAlive() => {
                    // First check if there are nodes that must be disconnected
                    // because they failed to reply to a keep-alive.
                    let mut to_be_disconnected = std::collections::HashSet::new();
                    log::debug!(
                        "nodes to be polled: {}",
                        nodes.keys().map(|x| x.to_string()).collect::<Vec<String>>().join(",")
                    );

                    let mut keep_alive_responses = vec![];
                    for (node_id, client_desc) in &mut nodes {
                        log::debug!("polling node {} begin", node_id);
                        match client_desc.api.node_management_api().keep_alive().await {
                            Ok(keep_alive_response) => {
                                log::debug!(
                                    "node uuid {} health status {} performance [function execution times: {} samples]",
                                    node_id,
                                    keep_alive_response.health_status,
                                    keep_alive_response.performance_samples.function_execution_times.len()
                                );
                                keep_alive_responses.push((*node_id, keep_alive_response));
                            }
                            Err(_) => {
                                to_be_disconnected.insert(*node_id);
                            }
                        };
                        log::debug!("polling node {} end", node_id);
                    }

                    // Second, remove all those nodes from the map of clients.
                    for node_id in to_be_disconnected.iter() {
                        log::info!("disconnected node not replying to keep-alive: {}", &node_id);
                        let val = nodes.remove(node_id);
                        assert!(val.is_some());
                    }

                    // Third, remove all the resource providers associated with
                    // the removed nodes.
                    resource_providers.retain(|_k, v| {
                        if to_be_disconnected.contains(&v.node_id) {
                            log::info!("removed resource from disconnected node: {}", v);
                            resource_providers_changed = true;
                            false
                        } else {
                            true
                        }
                    });

                    // Update the peers of (still alive) nodes by
                    // deleting the missing-in-action peers.
                    for removed_node_id in &to_be_disconnected {
                        for (_, client_desc) in nodes.iter_mut() {
                            match client_desc
                                .api
                                .node_management_api()
                                .update_peers(edgeless_api::node_management::UpdatePeersRequest::Del(*removed_node_id))
                                .await
                            {
                                Ok(_) => {}
                                Err(err) => {
                                    log::error!("Unhandled: {}", err);
                                }
                            }
                        }
                    }

                    // Update the orchestration logic and proxy.
                    orchestration_logic.update_nodes(&nodes, &resource_providers);
                    proxy.update_nodes(&nodes);
                    proxy.push_keep_alive_responses(keep_alive_responses);

                    //
                    // Make sure that all active logical functions are assigned
                    // to one instance: for all the function instances that
                    // were running in disconnected nodes, create new function
                    // instances on other nodes, if possible and there were no
                    // other running function instances.
                    //

                    // List of ext_fid that will have to be repatched
                    // because of the allocation of new function instances
                    // following node disconnection.
                    let mut to_be_repatched = vec![]; // ext_fid

                    // Function instances that have to be created to make up for
                    // the loss of those assigned to disconnected nodes.
                    // key:   ext_fid
                    // value: function request
                    let mut fun_to_be_created = std::collections::HashMap::new();

                    // Resources that have to be created to make up for the
                    // loss of those assigned to disconnected nodes.
                    // key:   ext_fid
                    // value: resource specs
                    let mut res_to_be_created = std::collections::HashMap::new();

                    // List of ext_fid that will have to be repatched.
                    let mut active_instances_to_be_updated = vec![];

                    // Find all the functions/resources affected.
                    // Also attempt to start functions and resources that
                    // are active but for which no active instance is present
                    // (this happens because in the past a node with active
                    // functions/resources has disappeared and it was not
                    // possible to fix the situation immediately).
                    for (origin_ext_fid, instance) in active_instances.iter() {
                        match instance {
                            ActiveInstance::Function(start_req, instances) => {
                                let num_disconnected = instances.iter().filter(|x| to_be_disconnected.contains(&x.node_id)).count();
                                assert!(num_disconnected <= instances.len());
                                if instances.is_empty() || num_disconnected > 0 {
                                    to_be_repatched.push(*origin_ext_fid);
                                    if instances.is_empty() || num_disconnected == instances.len() {
                                        // If all the function instances
                                        // disappared, then we must enforce the
                                        // creation of (at least) a new
                                        // function instance.
                                        fun_to_be_created.insert(*origin_ext_fid, start_req.clone());
                                    } else {
                                        // Otherwise, we just remove the
                                        // disappeared function instances and
                                        // let the others still alive handle
                                        // the logical function.
                                        active_instances_to_be_updated.push(*origin_ext_fid);
                                    }
                                }
                            }
                            ActiveInstance::Resource(start_req, instance) => {
                                if instance.is_none() || to_be_disconnected.contains(&instance.node_id) {
                                    to_be_repatched.push(*origin_ext_fid);
                                    res_to_be_created.insert(*origin_ext_fid, start_req.clone());
                                }
                            }
                        }
                    }

                    // Also schedule to repatch all the functions that
                    // depend on the functions/resources modified.
                    for (origin_ext_fid, output_mapping) in dependency_graph.iter() {
                        for (_output, target_ext_fid) in output_mapping.iter() {
                            if active_instances_to_be_updated.contains(target_ext_fid)
                                || fun_to_be_created.contains_key(target_ext_fid)
                                || res_to_be_created.contains_key(target_ext_fid)
                            {
                                to_be_repatched.push(*origin_ext_fid);
                            }
                        }
                    }

                    // Update the active instances of logical functions
                    // where at least one function instance went missing but
                    // there are others that are still assigned and alive.
                    for ext_fid in active_instances_to_be_updated.iter() {
                        match active_instances.get_mut(ext_fid) {
                            None => panic!("ext_fid {} just disappeared", ext_fid),
                            Some(active_instance) => {
                                active_instances_changed = true;
                                match active_instance {
                                    ActiveInstance::Resource(_, _) => panic!("expecting a function, found a resource for ext_fid {}", ext_fid),
                                    ActiveInstance::Function(_, instances) => instances.retain(|x| !to_be_disconnected.contains(&x.node_id)),
                                }
                            }
                        }
                    }

                    // Create the functions that went missing.
                    // If the operation fails for a function now, then the
                    // function remains in the active_instances, but it is
                    // assigned no function instance.
                    for (ext_fid, spawn_req) in fun_to_be_created.into_iter() {
                        let res = match Self::select_node(&spawn_req, &mut orchestration_logic) {
                            Ok(node_id) => {
                                // Start the function instance.
                                match Self::start_function(&spawn_req, &mut active_instances, &mut nodes, &ext_fid, &node_id).await {
                                    Ok(_) => Ok(()),
                                    Err(err) => Err(err),
                                }
                            }
                            Err(err) => Err(err),
                        };
                        if let Err(err) = res {
                            log::error!("error when creating a new function assigned with ext_fid {}: {}", ext_fid, err);
                            match active_instances.get_mut(&ext_fid).unwrap() {
                                ActiveInstance::Function(_spawn_req, instances) => instances.clear(),
                                ActiveInstance::Resource(_, _) => {
                                    panic!("expecting a function to be associated with ext_fid {}, found a resource", ext_fid)
                                }
                            }
                        }
                        active_instances_changed = true;
                    }

                    // Create the resources that went missing.
                    // If the operation fails for a resource now, then the
                    // resource remains in the active_instances, but it is
                    // assigned an invalid function instance.
                    for (ext_fid, start_req) in res_to_be_created.into_iter() {
                        match Self::start_resource(start_req, &mut resource_providers, &mut active_instances, &mut nodes, ext_fid, &mut rng).await {
                            Ok(_) => {}
                            Err(err) => {
                                log::error!("error when creating a new resource assigned with ext_fid {}: {}", ext_fid, err);
                                match active_instances.get_mut(&ext_fid).unwrap() {
                                    ActiveInstance::Function(_, _) => {
                                        panic!("expecting a resource to be associated with ext_fid {}, found a function", ext_fid)
                                    }
                                    ActiveInstance::Resource(_start_req, instance_id) => {
                                        *instance_id = edgeless_api::function_instance::InstanceId::none();
                                    }
                                }
                            }
                        }
                        active_instances_changed = true;
                    }

                    // Check if there are intents from the proxy.
                    for intent in proxy.retrieve_deploy_intents() {
                        match intent {
                            DeployIntent::Migrate(component, targets) => {
                                Orchestrator::migrate(&mut active_instances, &mut nodes, &orchestration_logic, &component, &targets).await;
                                to_be_repatched.push(component)
                            }
                        }
                        active_instances_changed = true;
                    }

                    // Repatch everything that needs to be repatched.
                    Self::apply_patches(&active_instances, &dependency_graph, &mut nodes, to_be_repatched).await;

                    // Update the proxy, if necessary.
                    if resource_providers_changed {
                        proxy.update_resource_providers(&resource_providers);
                        resource_providers_changed = false;
                    }
                    if active_instances_changed {
                        proxy.update_active_instances(&active_instances);
                        active_instances_changed = false;
                    }
                    if dependency_graph_changed {
                        proxy.update_dependency_graph(&dependency_graph);
                        dependency_graph_changed = false;
                    }
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::api::orc::OrchestratorAPI + Send> {
        Box::new(OrchestratorClient {
            function_instance_client: Box::new(OrchestratorFunctionInstanceOrcClient { sender: self.sender.clone() }),
            node_registration_client: Box::new(NodeRegistrationClient { sender: self.sender.clone() }),
            resource_configuration_client: Box::new(ResourceConfigurationClient { sender: self.sender.clone() }),
        })
    }
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::DomainManagedInstanceId>
    for OrchestratorFunctionInstanceOrcClient
{
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>> {
        log::debug!("FunctionInstance::start() {:?}", request);
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>>,
        >();
        if let Err(err) = self.sender.send(OrchestratorRequest::StartFunction(request, reply_sender)).await {
            return Err(anyhow::anyhow!(
                "Orchestrator channel error when creating a function instance: {}",
                err.to_string()
            ));
        }
        match reply_receiver.await {
            Ok(f_id) => f_id,
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when creating a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn stop(&mut self, id: edgeless_api::function_instance::DomainManagedInstanceId) -> anyhow::Result<()> {
        log::debug!("FunctionInstance::stop() {:?}", id);
        match self.sender.send(OrchestratorRequest::StopFunction(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        log::debug!("FunctionInstance::patch() {:?}", update);
        match self.sender.send(OrchestratorRequest::Patch(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::node_registration::NodeRegistrationAPI for NodeRegistrationClient {
    async fn update_node(
        &mut self,
        request: edgeless_api::node_registration::UpdateNodeRequest,
    ) -> anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse> {
        log::debug!("NodeRegistrationAPI::update_node() {:?}", request);
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse>>();
        if let Err(err) = self.sender.send(OrchestratorRequest::UpdateNode(request, reply_sender)).await {
            return Err(anyhow::anyhow!("Orchestrator channel error when updating a node: {}", err.to_string()));
        }
        match reply_receiver.await {
            Ok(res) => res,
            Err(err) => Err(anyhow::anyhow!("Orchestrator channel error  when updating a node: {}", err.to_string())),
        }
    }
    async fn keep_alive(&mut self) {
        log::debug!("NodeRegistrationAPI::keep_alive()");
        let _ = self.sender.send(OrchestratorRequest::KeepAlive()).await;
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::DomainManagedInstanceId>
    for ResourceConfigurationClient
{
    async fn start(
        &mut self,
        request: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>> {
        log::debug!("ResourceConfigurationAPI::start() {:?}", request);
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::DomainManagedInstanceId>>,
        >();
        if let Err(err) = self.sender.send(OrchestratorRequest::StartResource(request, reply_sender)).await {
            return Err(anyhow::anyhow!(
                "Orchestrator channel error when starting a resource: {}",
                err.to_string()
            ));
        }
        match reply_receiver.await {
            Ok(f_id) => f_id,
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when starting a resource: {}",
                err.to_string()
            )),
        }
    }

    async fn stop(&mut self, id: edgeless_api::function_instance::DomainManagedInstanceId) -> anyhow::Result<()> {
        log::debug!("ResourceConfigurationAPI::stop() {:?}", id);
        match self.sender.send(OrchestratorRequest::StopResource(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when stopping a resource: {}",
                err.to_string()
            )),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        log::debug!("ResourceConfigurationAPI::patch() {:?}", update);
        match self.sender.send(OrchestratorRequest::Patch(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}
