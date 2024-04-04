// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_api::function_instance::{ComponentId, InstanceId};

use futures::{Future, SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::orchestration_logic::OrchestrationLogic;

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
    STARTFUNCTION(
        edgeless_api::function_instance::SpawnFunctionRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::orc::DomainManagedInstanceId>>>,
    ),
    STOPFUNCTION(edgeless_api::orc::DomainManagedInstanceId),
    STARTRESOURCE(
        edgeless_api::resource_configuration::ResourceInstanceSpecification,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::orc::DomainManagedInstanceId>>>,
    ),
    STOPRESOURCE(edgeless_api::orc::DomainManagedInstanceId),
    PATCH(edgeless_api::common::PatchRequest),
    UPDATENODE(
        edgeless_api::node_registration::UpdateNodeRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse>>,
    ),
    KEEPALIVE(),
}

pub struct ResourceProvider {
    pub class_type: String,
    pub node_id: edgeless_api::function_instance::NodeId,
    pub outputs: Vec<String>,
}

#[derive(Clone)]
enum ActiveInstance {
    // 0: request
    // 1: deployment requirements
    // 2: [ (node_id, int_fid) ]
    Function(
        edgeless_api::function_instance::SpawnFunctionRequest,
        DeploymentRequirements,
        Vec<InstanceId>,
    ),

    // 0: request
    // 1: (node_id, int_fid)
    Resource(edgeless_api::resource_configuration::ResourceInstanceSpecification, InstanceId),
}

impl std::fmt::Display for ActiveInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ActiveInstance::Function(_req, _reqs, instances) => write!(
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
    function_instance_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::orc::DomainManagedInstanceId>>,
    node_registration_client: Box<dyn edgeless_api::node_registration::NodeRegistrationAPI>,
    resource_configuration_client:
        Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::orc::DomainManagedInstanceId>>,
}

impl edgeless_api::orc::OrchestratorAPI for OrchestratorClient {
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::orc::DomainManagedInstanceId>> {
        self.function_instance_client.clone()
    }

    fn node_registration_api(&mut self) -> Box<dyn edgeless_api::node_registration::NodeRegistrationAPI> {
        self.node_registration_client.clone()
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::orc::DomainManagedInstanceId>> {
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
    pub api: Box<dyn edgeless_api::agent::AgentAPI + Send>,
    pub capabilities: edgeless_api::node_registration::NodeCapabilities,
}

enum IntFid {
    // 0: node_id, int_fid
    Function(InstanceId),
    // 0: node_id, int_fid
    Resource(InstanceId),
}

impl IntFid {
    fn instance_id(&self) -> InstanceId {
        match self {
            Self::Function(id) => *id,
            Self::Resource(id) => *id,
        }
    }
}

impl Orchestrator {
    pub async fn new(settings: crate::EdgelessOrcSettings) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            Self::main_task(receiver, settings, std::collections::HashMap::new(), std::collections::HashMap::new()).await;
        });

        (Orchestrator { sender }, main_task)
    }

    #[cfg(test)]
    pub async fn new_with_clients(
        settings: crate::EdgelessOrcSettings,
        clients: std::collections::HashMap<uuid::Uuid, ClientDesc>,
        resource_providers: std::collections::HashMap<String, ResourceProvider>,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            Self::main_task(receiver, settings, clients, resource_providers).await;
        });

        (Orchestrator { sender }, main_task)
    }

    pub async fn keep_alive(&mut self) {
        let _ = self.sender.send(OrchestratorRequest::KEEPALIVE()).await;
    }

    fn ext_to_int(active_instances: &std::collections::HashMap<ComponentId, ActiveInstance>, ext_fid: &ComponentId) -> Vec<IntFid> {
        match active_instances.get(ext_fid) {
            Some(active_instance) => match active_instance {
                ActiveInstance::Function(_req, _reqs, instances) => instances
                    .iter()
                    .map(|x| {
                        IntFid::Function(InstanceId {
                            node_id: x.node_id,
                            function_id: x.function_id,
                        })
                    })
                    .collect(),
                ActiveInstance::Resource(_req, instance) => {
                    vec![IntFid::Resource(InstanceId {
                        node_id: instance.node_id,
                        function_id: instance.function_id,
                    })]
                }
            },
            None => vec![],
        }
    }

    async fn apply_patches(
        active_instances: &std::collections::HashMap<ComponentId, ActiveInstance>,
        active_patches: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>,
        clients: &mut std::collections::HashMap<uuid::Uuid, ClientDesc>,
        origin_ext_fids: Vec<ComponentId>,
    ) {
        for origin_ext_fid in origin_ext_fids.iter() {
            let ext_output_mapping = match active_patches.get(origin_ext_fid) {
                Some(x) => x,
                None => continue,
            };

            // Transform the external function identifiers into
            // internal ones.
            for source in Self::ext_to_int(&active_instances, origin_ext_fid) {
                let mut int_output_mapping = std::collections::HashMap::new();
                for (channel, target_ext_fid) in ext_output_mapping {
                    for target in Self::ext_to_int(&active_instances, target_ext_fid) {
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
                                function_id: instance_id.function_id.clone(),
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
                                function_id: instance_id.function_id.clone(),
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
        active_instances: &mut std::collections::HashMap<ComponentId, ActiveInstance>,
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
            Some(class_type) => {
                let resource_provider = resource_providers.get_mut(class_type).unwrap();
                match clients.get_mut(&resource_provider.node_id) {
                    Some(client) => match client
                        .api
                        .resource_configuration_api()
                        .start(edgeless_api::resource_configuration::ResourceInstanceSpecification {
                            class_type: class_type.clone(),
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
                                    ext_fid.clone(),
                                    ActiveInstance::Resource(
                                        start_req,
                                        InstanceId {
                                            node_id: resource_provider.node_id.clone(),
                                            function_id: instance_id.function_id.clone(),
                                        },
                                    ),
                                );
                                log::info!(
                                    "Started resource provider_id {}, node_id {}, ext_fid {}, int_fid {}",
                                    class_type,
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

    async fn start_function(
        spawn_req: edgeless_api::function_instance::SpawnFunctionRequest,
        orchestration_logic: &mut OrchestrationLogic,
        active_instances: &mut std::collections::HashMap<ComponentId, ActiveInstance>,
        clients: &mut std::collections::HashMap<uuid::Uuid, ClientDesc>,
        ext_fid: uuid::Uuid,
    ) -> Result<edgeless_api::common::StartComponentResponse<uuid::Uuid>, anyhow::Error> {
        // Orchestration step: select the node to spawn this
        // function instance by using the orchestration logic.
        // Orchestration strategy can also be changed during
        // runtime.

        let reqs = DeploymentRequirements::from_annotations(&spawn_req.annotations);
        let selected_node_id = match orchestration_logic.next(&reqs) {
            Some(u) => u,
            None => {
                return Err(anyhow::anyhow!(
                    "Could not start a function instance for ext_fid {}: no valid node found",
                    ext_fid
                ))
            }
        };

        let mut fn_client = match clients.get_mut(&selected_node_id) {
            Some(c) => c,
            None => panic!(
                "Invalid node selected by the orchestration logic when starting function instance ext_fid {}: {}",
                ext_fid, selected_node_id
            ),
        }
        .api
        .function_instance_api();

        log::debug!(
            "Orchestrator StartFunction {:?} ext_fid {} at worker node with node_id {:?}",
            spawn_req,
            ext_fid,
            selected_node_id
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
                    assert!(selected_node_id == id.node_id);
                    active_instances.insert(
                        ext_fid,
                        ActiveInstance::Function(
                            spawn_req,
                            reqs,
                            vec![InstanceId {
                                node_id: selected_node_id,
                                function_id: id.function_id,
                            }],
                        ),
                    );
                    log::info!(
                        "Spawned at node_id {}, ext_fid {}, int_fid {}",
                        selected_node_id,
                        &ext_fid,
                        id.function_id
                    );

                    Ok(edgeless_api::common::StartComponentResponse::InstanceId(ext_fid))
                }
            },
            Err(err) => {
                log::error!("Unhandled: {}", err);
                Err(anyhow::anyhow!("Could not start a function instance for ext_fid {}: {}", ext_fid, err))
            }
        }
    }

    /// Return the list of ext_fids that depend on the given one, according
    /// to the active patches.
    fn dependencies(
        active_patches: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>,
        ext_fid: &uuid::Uuid,
    ) -> Vec<uuid::Uuid> {
        let mut dependencies = vec![];
        for (origin_ext_fid, output_mapping) in active_patches.iter() {
            for (_output, target_ext_fid) in output_mapping.iter() {
                if target_ext_fid == ext_fid {
                    dependencies.push(origin_ext_fid.clone());
                    break;
                }
            }
        }
        dependencies
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<OrchestratorRequest>,
        orchestrator_settings: crate::EdgelessOrcSettings,
        clients: std::collections::HashMap<uuid::Uuid, ClientDesc>,
        resource_providers: std::collections::HashMap<String, ResourceProvider>,
    ) {
        let mut receiver = receiver;
        let mut orchestration_logic = crate::orchestration_logic::OrchestrationLogic::new(orchestrator_settings.orchestration_strategy);
        let mut rng = rand::rngs::StdRng::from_entropy();

        // known agents
        // key: node_id
        let mut clients = clients;
        orchestration_logic.update_nodes(&clients, &resource_providers);
        for (node_id, client_desc) in &clients {
            log::info!(
                "added function instance client: node_id {}, agent URL {}, invocation URL {}, capabilities {}",
                node_id,
                client_desc.agent_url,
                client_desc.invocation_url,
                client_desc.capabilities
            );
        }

        // known resources providers as notified by nodes upon registration
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

        // instances that the orchestrator promised to keep active
        // key: ext_fid
        let mut active_instances = std::collections::HashMap::new();

        // active patches to which the orchestrator committed
        // key:   ext_fid (origin function)
        // value: map of:
        //        key:   channel output name
        //        value: ext_fid (target function)
        let mut active_patches: std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>> =
            std::collections::HashMap::new();

        // Main loop that reacts to events on the receiver channel
        while let Some(req) = receiver.next().await {
            match req {
                OrchestratorRequest::STARTFUNCTION(spawn_req, reply_channel) => {
                    // Create a new ext_fid for this resource.
                    let ext_fid = uuid::Uuid::new_v4();

                    // Start the function instance.
                    let res = Self::start_function(spawn_req.clone(), &mut orchestration_logic, &mut active_instances, &mut clients, ext_fid).await;

                    // Send back the response to the caller.
                    if let Err(err) = reply_channel.send(res) {
                        log::error!("Orchestrator channel error in SPAWN: {:?}", err);
                    }
                }
                OrchestratorRequest::STOPFUNCTION(ext_fid) => {
                    log::debug!("Orchestrator StopFunction {:?}", ext_fid);

                    match active_instances.remove(&ext_fid) {
                        Some(active_instance) => {
                            match active_instance {
                                ActiveInstance::Function(_req, _reqs, instances) => {
                                    // Stop all the instances of this function.
                                    for instance in instances {
                                        match clients.get_mut(&instance.node_id) {
                                            Some(c) => match c.api.function_instance_api().stop(instance).await {
                                                Ok(_) => {
                                                    log::info!(
                                                        "Stopped function ext_fid {}, node_id {}, int_fid {}",
                                                        ext_fid,
                                                        instance.node_id,
                                                        instance.function_id
                                                    );
                                                }
                                                Err(err) => {
                                                    log::error!("Unhandled stop function ext_fid {}: {}", ext_fid, err);
                                                }
                                            },
                                            None => {
                                                log::error!(
                                                    "This orchestrator does not manage the node where the function instance is located: {}",
                                                    ext_fid
                                                );
                                            }
                                        }
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
                                &mut active_instances,
                                &active_patches,
                                &mut clients,
                                Self::dependencies(&active_patches, &ext_fid),
                            )
                            .await;
                            active_patches.remove(&ext_fid);
                        }
                        None => {
                            log::error!("Request to stop a function that is not known: ext_fid {}", ext_fid);
                        }
                    }
                }
                OrchestratorRequest::STARTRESOURCE(start_req, reply_channel) => {
                    log::debug!("Orchestrator StartResource {:?}", &start_req);

                    // Create a new ext_fid for this resource.
                    let ext_fid = uuid::Uuid::new_v4();

                    // Start the resource.
                    let res = Self::start_resource(
                        start_req.clone(),
                        &mut resource_providers,
                        &mut active_instances,
                        &mut clients,
                        ext_fid,
                        &mut rng,
                    )
                    .await;

                    // Send back the response to the caller.
                    if let Err(err) = reply_channel.send(res) {
                        log::error!("Orchestrator channel error in STARTRESOURCE: {:?}", err);
                    }
                }
                OrchestratorRequest::STOPRESOURCE(ext_fid) => {
                    log::debug!("Orchestrator StopResource {:?}", ext_fid);

                    match active_instances.remove(&ext_fid) {
                        Some(active_instance) => {
                            match active_instance {
                                ActiveInstance::Function(_, _, _) => {
                                    log::error!(
                                        "Request to stop a resource but the ext_fid is associated with a function: ext_fid {}",
                                        ext_fid
                                    );
                                }
                                ActiveInstance::Resource(_req, instance) => {
                                    // Stop the instance of this resource.
                                    match clients.get_mut(&instance.node_id) {
                                        Some(node_client) => match node_client.api.resource_configuration_api().stop(instance).await {
                                            Ok(_) => {
                                                log::info!(
                                                    "Stopped resource, ext_fid {}, node_id {}, int_fid {}",
                                                    ext_fid,
                                                    instance.node_id,
                                                    instance.function_id
                                                );
                                            }
                                            Err(err) => {
                                                log::error!("Unhandled stop resource ext_fid {}: {}", ext_fid, err);
                                            }
                                        },
                                        None => {
                                            log::error!("Request to stop a resource but the provider does not exist anymore, ext_fid {}", ext_fid);
                                        }
                                    }
                                }
                            }
                            Self::apply_patches(
                                &mut active_instances,
                                &active_patches,
                                &mut clients,
                                Self::dependencies(&active_patches, &ext_fid),
                            )
                            .await;
                            active_patches.remove(&ext_fid);
                        }
                        None => {
                            log::error!("Request to stop a resource that is not known: ext_fid {}", ext_fid);
                        }
                    }
                }
                OrchestratorRequest::PATCH(update) => {
                    log::debug!("Orchestrator Patch {:?}", update);

                    // Extract the ext_fid identifiers for the origin and
                    // target logical functions.
                    let origin_ext_fid = update.function_id.clone();
                    let output_mapping = update
                        .output_mapping
                        .iter()
                        .map(|x| (x.0.clone(), x.1.function_id.clone()))
                        .collect::<std::collections::HashMap<String, ComponentId>>();

                    // Save the patch request into an internal data structure,
                    // keeping track only of the ext_fid for both origin
                    // and target (logical) functions.
                    active_patches.insert(origin_ext_fid, output_mapping);

                    // Apply the patch.
                    Self::apply_patches(&active_instances, &active_patches, &mut clients, vec![origin_ext_fid]).await;
                }
                OrchestratorRequest::UPDATENODE(request, reply_channel) => {
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
                            if let Some(client_desc) = clients.get(&node_id) {
                                if client_desc.agent_url == agent_url && client_desc.invocation_url == invocation_url {
                                    dup_entry = true;
                                }
                            }
                            if dup_entry {
                                // A client with same node_id, agent_url, and
                                // invocation_url already exists.
                                None
                            } else {
                                this_node_id = Some(node_id.clone());

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
                                                node_id: this_node_id.unwrap().clone(),
                                                outputs: resource.outputs.clone(),
                                            },
                                        );
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
                                clients.insert(
                                    node_id,
                                    ClientDesc {
                                        agent_url: agent_url.clone(),
                                        invocation_url: invocation_url.clone(),
                                        api: Box::new(edgeless_api::grpc_impl::agent::AgentAPIClient::new(&agent_url).await),
                                        capabilities,
                                    },
                                );
                                Some(edgeless_api::node_management::UpdatePeersRequest::Add(node_id, invocation_url))
                            }
                        }
                        edgeless_api::node_registration::UpdateNodeRequest::Deregistration(node_id) => {
                            if let None = clients.get(&node_id) {
                                // There is no client with that node_id
                                None
                            } else {
                                clients.remove(&node_id);
                                Some(edgeless_api::node_management::UpdatePeersRequest::Del(node_id))
                            }
                        }
                    };

                    // If no operation was done (either a new node was already
                    // present with same agent/invocation URLs or a deregistering
                    // node did not exist) we accept the command.
                    let mut response = edgeless_api::node_registration::UpdateNodeResponse::Accepted;

                    if let Some(msg) = msg {
                        // Update the orchestration logic with the new set of nodes.
                        orchestration_logic.update_nodes(&clients, &resource_providers);

                        // Update all the peers (including the node, unless it
                        // was a deregister operation).
                        let mut num_failures: u32 = 0;
                        for (_node_id, client) in clients.iter_mut() {
                            if let Err(_) = client.api.node_management_api().update_peers(msg.clone()).await {
                                num_failures += 1;
                            }
                        }

                        // Only with registration, we also update the new node
                        // by adding as peers all the existing nodes.
                        if let Some(this_node_id) = this_node_id {
                            let mut new_node_client = clients.get_mut(&this_node_id).unwrap().api.node_management_api();
                            for (other_node_id, client_desc) in clients.iter_mut() {
                                if other_node_id.eq(&this_node_id) {
                                    continue;
                                }
                                if let Err(_) = new_node_client
                                    .update_peers(edgeless_api::node_management::UpdatePeersRequest::Add(
                                        *other_node_id,
                                        client_desc.invocation_url.clone(),
                                    ))
                                    .await
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
                OrchestratorRequest::KEEPALIVE() => {
                    log::debug!("keep alive");

                    // First check if there are nodes that must be disconnected
                    // because they failed to reply to a keep-alive.
                    let mut to_be_disconnected = std::collections::HashSet::new();
                    for (node_id, client_desc) in &mut clients {
                        if let Err(_) = client_desc.api.node_management_api().keep_alive().await {
                            to_be_disconnected.insert(*node_id);
                        }
                    }

                    // Second, remove all those nodes from the map of clients.
                    for node_id in to_be_disconnected.iter() {
                        log::info!("disconnected node not replying to keep alive: {}", &node_id);
                        let val = clients.remove(&node_id);
                        assert!(val.is_some());
                    }

                    // Third, remove all the resource providers associated with
                    // the removed nodes.
                    resource_providers.retain(|_k, v| {
                        if to_be_disconnected.contains(&v.node_id) {
                            log::info!("removed resource from disconnected node: {}", v);
                            false
                        } else {
                            true
                        }
                    });

                    // Update the peers of (still alive) nodes by
                    // deleting the missing-in-action peers.
                    for removed_node_id in &to_be_disconnected {
                        for (_, client_desc) in clients.iter_mut() {
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

                    // Update the orchestration logic.
                    orchestration_logic.update_nodes(&clients, &resource_providers);

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
                    for (origin_ext_fid, instance) in active_instances.iter() {
                        match instance {
                            ActiveInstance::Function(start_req, _reqs, instances) => {
                                let num_disconnected = instances.iter().filter(|x| to_be_disconnected.contains(&x.node_id)).count();
                                assert!(num_disconnected <= instances.len());
                                if num_disconnected > 0 {
                                    to_be_repatched.push(origin_ext_fid.clone());
                                    if num_disconnected == instances.len() {
                                        // If all the function instances
                                        // disappared, then we must enforce the
                                        // creation of (at least) a new
                                        // function instance.
                                        fun_to_be_created.insert(origin_ext_fid.clone(), start_req.clone());
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
                                if to_be_disconnected.contains(&instance.node_id) {
                                    to_be_repatched.push(origin_ext_fid.clone());
                                    res_to_be_created.insert(origin_ext_fid.clone(), start_req.clone());
                                }
                            }
                        }
                    }

                    // Also schedule to be repatch all the functions that
                    // depend on the functions/resources modified.
                    for (origin_ext_fid, output_mapping) in active_patches.iter() {
                        for (_output, target_ext_fid) in output_mapping.iter() {
                            if active_instances_to_be_updated.contains(target_ext_fid)
                                || fun_to_be_created.contains_key(target_ext_fid)
                                || res_to_be_created.contains_key(target_ext_fid)
                            {
                                to_be_repatched.push(origin_ext_fid.clone());
                            }
                        }
                    }

                    // Update the active instances of logical functions
                    // where at least one function instance went missing but
                    // there are others that are still assigned and alive.
                    for ext_fid in active_instances_to_be_updated.iter() {
                        match active_instances.get_mut(ext_fid) {
                            None => panic!("ext_fid {} just disappeared", ext_fid),
                            Some(active_instance) => match active_instance {
                                ActiveInstance::Resource(_, _) => panic!("expecting a function, found a resource for ext_fid {}", ext_fid),
                                ActiveInstance::Function(_, _, instances) => instances.retain(|x| !to_be_disconnected.contains(&x.node_id)),
                            },
                        }
                    }

                    // Create the functions that went missing.
                    for (ext_fid, spawn_req) in fun_to_be_created.into_iter() {
                        match Self::start_function(spawn_req, &mut orchestration_logic, &mut active_instances, &mut clients, ext_fid).await {
                            Ok(_) => {}
                            Err(err) => {
                                log::error!("error when creating a new function assigned with ext_fid {}: {}", ext_fid, err);
                            }
                        }
                    }

                    // Create the resources that went missing.
                    for (ext_fid, start_req) in res_to_be_created.into_iter() {
                        match Self::start_resource(start_req, &mut resource_providers, &mut active_instances, &mut clients, ext_fid, &mut rng).await {
                            Ok(_) => {}
                            Err(err) => {
                                log::error!("error when creating a new resource assigned with ext_fid {}: {}", ext_fid, err);
                            }
                        }
                    }

                    // Repatch everything that needs to be repatched.
                    Self::apply_patches(&mut active_instances, &active_patches, &mut clients, to_be_repatched).await;
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::orc::OrchestratorAPI + Send> {
        Box::new(OrchestratorClient {
            function_instance_client: Box::new(OrchestratorFunctionInstanceOrcClient { sender: self.sender.clone() }),
            node_registration_client: Box::new(NodeRegistrationClient { sender: self.sender.clone() }),
            resource_configuration_client: Box::new(ResourceConfigurationClient { sender: self.sender.clone() }),
        })
    }
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::orc::DomainManagedInstanceId> for OrchestratorFunctionInstanceOrcClient {
    async fn start(
        &mut self,
        request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::orc::DomainManagedInstanceId>> {
        log::debug!("FunctionInstance::start() {:?}", request);
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::orc::DomainManagedInstanceId>>,
        >();
        if let Err(err) = self.sender.send(OrchestratorRequest::STARTFUNCTION(request, reply_sender)).await {
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

    async fn stop(&mut self, id: edgeless_api::orc::DomainManagedInstanceId) -> anyhow::Result<()> {
        log::debug!("FunctionInstance::stop() {:?}", id);
        match self.sender.send(OrchestratorRequest::STOPFUNCTION(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        log::debug!("FunctionInstance::patch() {:?}", update);
        match self.sender.send(OrchestratorRequest::PATCH(update)).await {
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
        if let Err(err) = self.sender.send(OrchestratorRequest::UPDATENODE(request, reply_sender)).await {
            return Err(anyhow::anyhow!("Orchestrator channel error when updating a node: {}", err.to_string()));
        }
        match reply_receiver.await {
            Ok(res) => res,
            Err(err) => Err(anyhow::anyhow!("Orchestrator channel error  when updating a node: {}", err.to_string())),
        }
    }
    async fn keep_alive(&mut self) {
        log::debug!("NodeRegistrationAPI::keep_alive()");
        let _ = self.sender.send(OrchestratorRequest::KEEPALIVE()).await;
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::orc::DomainManagedInstanceId> for ResourceConfigurationClient {
    async fn start(
        &mut self,
        request: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::orc::DomainManagedInstanceId>> {
        log::debug!("ResourceConfigurationAPI::start() {:?}", request);
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<
            anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::orc::DomainManagedInstanceId>>,
        >();
        if let Err(err) = self.sender.send(OrchestratorRequest::STARTRESOURCE(request, reply_sender)).await {
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

    async fn stop(&mut self, id: edgeless_api::orc::DomainManagedInstanceId) -> anyhow::Result<()> {
        log::debug!("ResourceConfigurationAPI::stop() {:?}", id);
        match self.sender.send(OrchestratorRequest::STOPRESOURCE(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when stopping a resource: {}",
                err.to_string()
            )),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        log::debug!("ResourceConfigurationAPI::patch() {:?}", update);
        match self.sender.send(OrchestratorRequest::PATCH(update)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Orchestrator channel error when updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}
