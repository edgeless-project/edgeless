// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use futures::StreamExt;
use rand::{seq::SliceRandom, SeedableRng};
use std::{io::Write, str::FromStr};

use crate::controller::deployment_state::ActiveWorkflow;

pub struct OrchestratorDesc {
    pub client: Box<dyn edgeless_api::outer::orc::OrchestratorAPI>,
    pub orchestrator_url: String,
    pub capabilities: edgeless_api::domain_registration::DomainCapabilities,
    pub refresh_deadline: std::time::SystemTime,
    pub counter: u64,
    pub nonce: u64,
}

#[derive(Default, Debug)]
pub struct PortalDesc {
    /// Name of the domain that acts as a portal for inter-domain workflows.
    pub domain_bal: String,
    /// Domains interconnected.
    pub domains: std::collections::HashSet<String>,
}

pub struct ControllerTask {
    persistence_filename: String,
    workflow_instance_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
    domain_registration_receiver: futures::channel::mpsc::UnboundedReceiver<super::DomainRegisterRequest>,
    internal_receiver: futures::channel::mpsc::UnboundedReceiver<super::InternalRequest>,
    orchestrators: std::collections::HashMap<String, OrchestratorDesc>,
    portal_desc: Option<PortalDesc>,
    active_workflows: std::collections::HashMap<edgeless_api::workflow_instance::WorkflowId, super::deployment_state::ActiveWorkflow>,
    orphan_workflows: std::collections::BTreeMap<edgeless_api::workflow_instance::WorkflowId, edgeless_api::workflow_instance::SpawnWorkflowRequest>,
    rng: rand::rngs::StdRng,
    last_portal_resource_id: u64,
}

type PersistedWorkflows = Vec<(String, edgeless_api::workflow_instance::SpawnWorkflowRequest)>;

#[derive(Default, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct PersistedState {
    workflows: PersistedWorkflows,
}

impl ControllerTask {
    pub fn new(
        persistence_filename: String,
        workflow_instance_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
        domain_registration_receiver: futures::channel::mpsc::UnboundedReceiver<super::DomainRegisterRequest>,
        internal_receiver: futures::channel::mpsc::UnboundedReceiver<super::InternalRequest>,
    ) -> Self {
        let orphan_workflows = ControllerTask::load_persistence(&persistence_filename);
        Self {
            persistence_filename,
            workflow_instance_receiver,
            domain_registration_receiver,
            internal_receiver,
            orchestrators: std::collections::HashMap::new(),
            portal_desc: None,
            active_workflows: std::collections::HashMap::new(),
            orphan_workflows,
            rng: rand::rngs::StdRng::from_entropy(),
            last_portal_resource_id: 0,
        }
    }

    #[cfg(test)]
    pub fn new_with_orchestrators(
        workflow_instance_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
        domain_registration_receiver: futures::channel::mpsc::UnboundedReceiver<super::DomainRegisterRequest>,
        internal_receiver: futures::channel::mpsc::UnboundedReceiver<super::InternalRequest>,
        orchestrators: std::collections::HashMap<String, OrchestratorDesc>,
    ) -> Self {
        Self {
            persistence_filename: String::default(),
            workflow_instance_receiver,
            domain_registration_receiver,
            internal_receiver,
            orchestrators,
            portal_desc: None,
            active_workflows: std::collections::HashMap::new(),
            orphan_workflows: std::collections::BTreeMap::new(),
            rng: rand::rngs::StdRng::from_entropy(),
            last_portal_resource_id: 0,
        }
    }

    fn load_persistence(
        filename: &str,
    ) -> std::collections::BTreeMap<edgeless_api::workflow_instance::WorkflowId, edgeless_api::workflow_instance::SpawnWorkflowRequest> {
        let mut ret = std::collections::BTreeMap::new();

        if filename.is_empty() {
            return ret;
        }

        let file = match std::fs::File::open(filename) {
            Ok(file) => file,
            Err(err) => {
                log::warn!("could not load from persistence file '{}': {}", filename, err);
                return ret;
            }
        };
        let reader = std::io::BufReader::new(file);
        let data: PersistedState = match serde_json::from_reader(reader) {
            Ok(data) => data,
            Err(err) => {
                log::warn!("invalid content found in persistence file '{}': {}", filename, err);
                return ret;
            }
        };

        for (uuid, request) in data.workflows {
            let workflow_id = match uuid::Uuid::from_str(&uuid) {
                Ok(uuid) => uuid,
                Err(err) => {
                    log::warn!("invalid workflow UUID found in persistence file '{}': {}", filename, err);
                    return ret;
                }
            };
            ret.insert(edgeless_api::workflow_instance::WorkflowId { workflow_id }, request);
        }

        ret
    }

    /// Save the currently active/orphan workflows to a file.
    /// Do nothing if the name of the persistence file is empty.
    fn persist(&self) {
        if self.persistence_filename.is_empty() {
            return;
        }

        let mut persistence = match std::fs::OpenOptions::new()
            .write(true)
            .append(false)
            .create(true)
            .truncate(true)
            .open(&self.persistence_filename)
        {
            Ok(file) => file,
            Err(err) => {
                log::warn!("could not open the persistence file '{}': {}", self.persistence_filename, err);
                return;
            }
        };

        // Copy all the workflow information into the data structure to be
        // serialized.
        let mut persisted_state = PersistedState::default();
        for (wid, active_workflow) in &self.orphan_workflows {
            persisted_state.workflows.push((wid.to_string(), active_workflow.clone()));
        }
        for (wid, active_workflow) in &self.active_workflows {
            persisted_state.workflows.push((wid.to_string(), active_workflow.desired_state.clone()));
        }

        match serde_json::to_string(&persisted_state) {
            Ok(serialized) => {
                if let Err(err) = write!(&mut persistence, "{}", serialized) {
                    log::warn!("error saving the persistence state to '{}': {}", self.persistence_filename, err)
                }
            }
            Err(err) => log::warn!("error serializing the persistence state: {}", err),
        }
    }

    /// Main loop of the controller task serving events received on the
    /// WorkflowInstanceAPI or DomainRegistrationAPI.
    pub async fn run(&mut self) {
        loop {
            tokio::select! {
                biased;
                Some(req) = self.domain_registration_receiver.next() => {
                    match req {
                        super::DomainRegisterRequest::Update(update_domain_request, reply_sender) => {
                            let reply = self.update_domain(&update_domain_request).await;
                            match reply_sender.send(reply) {
                                Ok(_) => {}
                                Err(err) => {
                                    log::error!("Unhandled: {:?}", err);
                                }
                            }
                        }
                    }
                }
                Some(req) = self.workflow_instance_receiver.next() => {
                    match req {
                        super::ControllerRequest::Start(spawn_workflow_request, reply_sender) => {
                            let reply = match self.start_workflow(spawn_workflow_request).await {
                                Ok(val) => Ok(val),
                                Err(spawn_req) => Err(anyhow::anyhow!("could not start workflow: {:?}", spawn_req))
                            };
                            if let Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(_)) = &reply {
                                self.persist();
                            }
                            if let Err(err) = reply_sender.send(reply) {
                                log::error!("Unhandled: {:?}", err);
                            }
                        }
                        super::ControllerRequest::Stop(wf_id) => {
                            if self.stop_workflow(&wf_id).await.is_some() {
                                self.persist();
                            }
                        }
                        super::ControllerRequest::List(reply_sender) => {
                            let reply = self.list();
                            if let Err(err) =  reply_sender.send(Ok(reply)) {
                                log::error!("Unhandled: {:?}", err);
                            }
                        }
                        super::ControllerRequest::Inspect(wf_id, reply_sender) => {
                            let reply = self.inspect(wf_id);
                            if let Err(err) =  reply_sender.send(reply) {
                                log::error!("Unhandled: {:?}", err);
                            }
                        }
                        super::ControllerRequest::Domains(domain_id, reply_sender) => {
                            let reply = self.domains(&domain_id);
                            if let Err(err) = reply_sender.send(reply) {
                                log::error!("Unhandled: {:?}", err);
                            }
                        }
                        super::ControllerRequest::Migrate(request, reply_sender) => {
                            let reply = match self.migrate_workflow(&request).await {
                                Ok(val) => Ok(val),
                                Err(spawn_req) => Err(anyhow::anyhow!("could not migrate workflow: {:?}", spawn_req))
                            };
                            if let Err(err) = reply_sender.send(reply) {
                                log::error!("Unhandled: {:?}", err);
                            }
                        }
                    }
                },
                Some(req) = self.internal_receiver.next() => {
                    match req {
                        super::InternalRequest::Refresh(reply_sender) => {
                            self.refresh().await;
                            let _ = reply_sender.send(());
                        }
                    }
                }
            }
        }
    }

    async fn start_workflow(
        &mut self,
        spawn_workflow_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse, edgeless_api::workflow_instance::SpawnWorkflowRequest> {
        if !spawn_workflow_request.annotations.is_empty() {
            log::warn!(
                "Workflow annotations ({}) are currently ignored",
                spawn_workflow_request.annotations.len()
            );
        }

        // Optimistically identify a new identifier for the workflow that
        // will be created, which will go unused if creation fails.
        let wf_id = edgeless_api::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::new_v4(),
        };

        // Find a domain that can host all the workflow's functions and
        // resources.
        let candidate_domains = Self::workflow_compatible_domains(&self.orchestrators, &spawn_workflow_request);

        match candidate_domains.choose(&mut self.rng) {
            Some(target_domain) => {
                let domain_assignments = Self::fill_domains(&spawn_workflow_request, target_domain);
                self.relocate_workflow(&wf_id, spawn_workflow_request, domain_assignments).await
            }
            None => {
                // No single domain was able to host the workflow.
                // Try again with multiple domains attached to the portal, if any.
                let domain_assignments = self.domain_assignments_portal(&spawn_workflow_request);

                if domain_assignments.is_empty() {
                    Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: "Workflow creation failed".to_string(),
                            detail: None,
                        },
                    ))
                } else {
                    self.relocate_workflow(&wf_id, spawn_workflow_request, domain_assignments).await
                }
            }
        }
    }

    /// Assign to all function/resources the same `target_domain`.
    fn fill_domains(
        spawn_workflow_request: &edgeless_api::workflow_instance::SpawnWorkflowRequest,
        target_domain: &str,
    ) -> std::collections::HashMap<String, String> {
        let functions: std::collections::HashMap<String, String> = spawn_workflow_request
            .workflow_functions
            .iter()
            .map(|function| (function.name.clone(), target_domain.to_string()))
            .collect();
        let mut resources: std::collections::HashMap<String, String> = spawn_workflow_request
            .workflow_resources
            .iter()
            .map(|resource| (resource.name.clone(), target_domain.to_string()))
            .collect();
        resources.extend(functions);
        resources
    }

    async fn relocate_workflow(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        spawn_workflow_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
        domain_assignments: std::collections::HashMap<String, String>,
    ) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse, edgeless_api::workflow_instance::SpawnWorkflowRequest> {
        // Return immediately if the workflow spec is not valid.
        if let Err(err) = spawn_workflow_request.is_valid() {
            return Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Workflow creation failed".to_string(),
                    detail: Some(err.to_string()),
                },
            ));
        }

        assert!(
            !self.active_workflows.contains_key(wf_id),
            "trying to activate WF {} which is already active",
            wf_id
        );

        // Make sure that all the function/resources are mapped.
        if log::log_enabled!(log::Level::Debug) {
            for component in spawn_workflow_request.all_component_names() {
                assert!(
                    domain_assignments.contains_key(&component),
                    "function/resource {} is not mapped to a domain",
                    component
                );
            }
        }

        // Define the workflow specification supporting inter-domain patches.
        #[derive(Debug)]
        struct NewResource {
            name: String,
            configurations: std::collections::HashMap<String, String>,
            output_mapping: std::collections::HashMap<String, String>,
            domain: String,
        }
        let mut new_resources = vec![];
        #[derive(Debug)]
        struct UpdateOutputMapping {
            name: String,
            channel: String,
            new_target: String,
        }
        let mut update_output_mappings = vec![];
        let mut augmented_spec = spawn_workflow_request.clone();
        for (component, output_mappings) in augmented_spec.output_mappings() {
            let origin_domain = domain_assignments.get(&component).unwrap();
            for (channel, target_component_name) in output_mappings {
                let target_domain = domain_assignments.get(&target_component_name).unwrap();

                if origin_domain == target_domain {
                    continue;
                }

                assert!(
                    self.portal_desc.is_some(),
                    "trying to patch functions/resources across domains without a portal"
                );
                let domain_bal = self.portal_desc.as_ref().unwrap().domain_bal.clone();

                // Create the first pair of portal resources (sink).
                self.last_portal_resource_id += 1;
                let id = self.last_portal_resource_id;
                let first_resource_name = format!("portal-{}-sink-local", id);
                new_resources.push(NewResource {
                    name: first_resource_name.clone(),
                    configurations: std::collections::HashMap::from([
                        (String::from("role"), String::from("sink")),
                        (String::from("domain"), String::from("local")),
                        (String::from("id"), id.to_string()),
                    ]),
                    output_mapping: std::collections::HashMap::new(),
                    domain: origin_domain.clone(),
                });
                let next_resource_name = format!("portal-{}-source-portal", id + 1);
                new_resources.push(NewResource {
                    name: format!("portal-{}-sink-portal", id),
                    configurations: std::collections::HashMap::from([
                        (String::from("role"), String::from("sink")),
                        (String::from("domain"), String::from("portal")),
                        (String::from("domain_name"), origin_domain.clone()),
                        (String::from("id"), id.to_string()),
                    ]),
                    output_mapping: std::collections::HashMap::from([(String::from("out"), next_resource_name.clone())]),
                    domain: domain_bal.clone(),
                });

                // Create the second pair of portal resources (source).
                self.last_portal_resource_id += 1;
                let id = self.last_portal_resource_id;
                new_resources.push(NewResource {
                    name: next_resource_name,
                    configurations: std::collections::HashMap::from([
                        (String::from("role"), String::from("source")),
                        (String::from("domain"), String::from("portal")),
                        (String::from("domain_name"), target_domain.clone()),
                        (String::from("id"), id.to_string()),
                    ]),
                    output_mapping: std::collections::HashMap::new(),
                    domain: domain_bal.clone(),
                });
                new_resources.push(NewResource {
                    name: format!("portal-{}-source-local", id),
                    configurations: std::collections::HashMap::from([
                        (String::from("role"), String::from("source")),
                        (String::from("domain"), String::from("local")),
                        (String::from("id"), id.to_string()),
                    ]),
                    output_mapping: std::collections::HashMap::from([(String::from("out"), target_component_name.clone())]),
                    domain: target_domain.clone(),
                });

                // Change the target in the original component.
                update_output_mappings.push(UpdateOutputMapping {
                    name: component.clone(),
                    channel,
                    new_target: first_resource_name,
                });
            }
        }

        // Add the new resources to the augmented workflow and update the
        // domain mapping.
        let mut domain_assignments = domain_assignments;
        for new_resource in new_resources {
            domain_assignments.insert(new_resource.name.clone(), new_resource.domain);
            augmented_spec.workflow_resources.push(edgeless_api::workflow_instance::WorkflowResource {
                name: new_resource.name,
                class_type: String::from("portal"),
                output_mapping: new_resource.output_mapping,
                configurations: new_resource.configurations,
            });
        }

        // Update the output mappings of regular functions/resources.
        for UpdateOutputMapping { name, channel, new_target } in update_output_mappings {
            augmented_spec.update_mapping(&name, &channel, new_target);
        }

        // Create the descriptor that will hold the active workflow.
        let mut workflow = super::deployment_state::ActiveWorkflow {
            desired_state: spawn_workflow_request,
            augmented_spec: None, // set later
            domain_mapping: std::collections::HashMap::new(),
        };

        // Keep the last error.
        let mut res: Result<(), String> = Ok(());

        //
        // First pass: create instances for all the functions and resources.
        //

        // Start the functions on the orchestration domain.
        for function in &augmented_spec.workflow_functions {
            if res.is_err() {
                log::error!("Could not start a function {}", res.clone().unwrap_err());
                break;
            }

            res = self
                .start_workflow_function_in_domain(wf_id, &mut workflow, function, domain_assignments.get(&function.name).unwrap())
                .await;
        }

        // Start the resources on the orchestration domain.
        for resource in &augmented_spec.workflow_resources {
            if res.is_err() {
                log::error!("Could not start a resource {}", res.clone().unwrap_err());
                break;
            }

            res = self
                .start_workflow_resource_in_domain(wf_id, &mut workflow, resource, domain_assignments.get(&resource.name).unwrap())
                .await;
        }

        //
        // Second pass: patch the workflow, if all the functions
        // have been created successfully.
        //

        // Loop on all the functions and resources of the workflow.
        for component_name in augmented_spec.source_components() {
            if res.is_err() {
                log::error!("Could not patch the component {}, reason: {}", component_name, res.clone().unwrap_err());
                break;
            }

            // Loop on all the identifiers for this function/resource
            // (once for each orchestration domain to which the
            // function/resource was allocated).
            for origin_fid in workflow.mapped_fids(&component_name).unwrap() {
                let physical_mapping = workflow.physical_mapping(augmented_spec.output_mappings().get(&component_name).unwrap());

                if physical_mapping.is_empty() {
                    continue;
                }

                let component_type = workflow.component_type(&component_name).unwrap();
                let origin_domain = domain_assignments.get(&component_name).unwrap();

                // Make sure that the all the components to be patched are in
                // the same domain.
                if log::log_enabled!(log::Level::Debug) {
                    for target_component in augmented_spec.output_mappings().get(&component_name).unwrap().values() {
                        let target_domain = domain_assignments.get(target_component).unwrap();
                        assert!(
                            origin_domain == target_domain,
                            "invalid mapping at WFID {} across domains {} ({}) -> {} ({}) ",
                            wf_id,
                            component_name,
                            origin_domain,
                            target_component,
                            target_domain
                        );
                    }
                }

                res = self
                    .patch_outputs(origin_domain, origin_fid, component_type, physical_mapping, &component_name)
                    .await;
            }
        }

        // Add the augmented spec to the workflow.
        workflow.augmented_spec = Some(augmented_spec);

        // Add the newly-created workflow to the container of active ones.
        self.active_workflows.insert(wf_id.clone(), workflow);

        //
        // If all went OK, notify the client that the workflow
        // has been accepted.
        // On the other hand, if something went wrong, we must stop
        // all the functions and resources that have been started.
        //

        if res.is_err() {
            log::error!("Workflow start failed, stopping");
            self.stop_workflow(wf_id).await;
        }

        let reply = match res {
            Ok(_) => Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(
                edgeless_api::workflow_instance::WorkflowInstance {
                    workflow_id: wf_id.clone(),
                    domain_mapping: self.active_workflows.get(wf_id).unwrap().domain_mapping(),
                },
            )),
            Err(err) => Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Workflow creation failed".to_string(),
                    detail: Some(err),
                },
            )),
        };

        reply
    }

    async fn stop_workflow(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
    ) -> Option<edgeless_api::workflow_instance::SpawnWorkflowRequest> {
        let workflow = match self.active_workflows.get(wf_id) {
            None => {
                log::error!("trying to tear-down a workflow that does not exist: {}", wf_id.to_string());
                return None;
            }
            Some(val) => val,
        };

        // Stop all the functions/resources.
        for component in workflow.domain_mapping.values() {
            log::debug!("stopping function/resource of workflow {}: {}", wf_id.to_string(), &component);
            let orc_api = match self.orchestrators.get_mut(&component.domain_id) {
                None => {
                    log::warn!(
                        "Orchestration domain '{}' for workflow '{}' component '{}' disappeared",
                        &component.domain_id,
                        wf_id,
                        &component.name,
                    );
                    continue;
                }
                Some(val) => val,
            };
            let mut fn_client = orc_api.client.function_instance_api();
            let mut resource_client = orc_api.client.resource_configuration_api();
            match component.component_type {
                super::ComponentType::Function => {
                    if let Err(err) = fn_client.stop(component.lid).await {
                        log::error!("Unhandled error when stopping wf '{}' function '{}': {}", wf_id, component.name, err);
                    }
                }
                super::ComponentType::Resource => {
                    if let Err(err) = resource_client.stop(component.lid).await {
                        log::error!("Unhandled error when stopping wf '{}' resource '{}': {}", wf_id, component.name, err);
                    }
                }
            }
        }

        // Remove the workflow from the active set.
        let remove_res = self.active_workflows.remove(wf_id);
        assert!(remove_res.is_some());
        Some(remove_res.unwrap().desired_state)
    }

    fn list(&self) -> Vec<edgeless_api::workflow_instance::WorkflowId> {
        let mut ret: Vec<edgeless_api::workflow_instance::WorkflowId> = vec![];
        for wf_id in self.active_workflows.keys() {
            ret.push(wf_id.clone());
        }
        for wf_id in self.orphan_workflows.keys() {
            ret.push(wf_id.clone());
        }
        ret
    }

    fn inspect(&self, wf_id: edgeless_api::workflow_instance::WorkflowId) -> anyhow::Result<edgeless_api::workflow_instance::WorkflowInfo> {
        if let Some(workflow) = self.active_workflows.get(&wf_id) {
            Ok(edgeless_api::workflow_instance::WorkflowInfo {
                request: workflow.augmented_spec.clone().unwrap(),
                status: edgeless_api::workflow_instance::WorkflowInstance {
                    workflow_id: wf_id.clone(),
                    domain_mapping: workflow
                        .domain_mapping
                        .values()
                        .map(|elem| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                            name: elem.name.clone(),
                            function_id: elem.lid,
                            domain_id: elem.domain_id.clone(),
                        })
                        .collect(),
                },
            })
        } else if let Some(request) = self.orphan_workflows.get(&wf_id) {
            Ok(edgeless_api::workflow_instance::WorkflowInfo {
                request: request.clone(),
                status: edgeless_api::workflow_instance::WorkflowInstance {
                    workflow_id: wf_id.clone(),
                    domain_mapping: vec![],
                },
            })
        } else {
            anyhow::bail!("Unknown workflow identifier '{}", wf_id);
        }
    }

    fn domains(&self, domain_id: &str) -> anyhow::Result<std::collections::HashMap<String, edgeless_api::domain_registration::DomainCapabilities>> {
        let mut ret = std::collections::HashMap::new();

        for (id, desc) in &self.orchestrators {
            if domain_id.is_empty() || domain_id == id {
                ret.insert(id.clone(), desc.capabilities.clone());
            }
        }

        Ok(ret)
    }

    /// Update domain information.
    ///
    /// Also update portal domain.
    async fn update_domain(
        &mut self,
        update_domain_request: &edgeless_api::domain_registration::UpdateDomainRequest,
    ) -> anyhow::Result<edgeless_api::domain_registration::UpdateDomainResponse> {
        log::debug!("Update domain request received {:?}", update_domain_request);

        if update_domain_request.domain_id.is_empty() {
            return Ok(edgeless_api::domain_registration::UpdateDomainResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: String::from("Empty domain identifier"),
                    detail: None,
                },
            ));
        }

        let (ret, update_portal_domain) = match self.orchestrators.get_mut(&update_domain_request.domain_id) {
            None => {
                log::info!(
                    "New domain '{}' with {} nodes",
                    update_domain_request.domain_id,
                    update_domain_request.capabilities.num_nodes
                );
                self.orchestrators.insert(
                    update_domain_request.domain_id.clone(),
                    OrchestratorDesc {
                        client: Box::new(
                            edgeless_api::grpc_impl::outer::orc::OrchestratorAPIClient::new(&update_domain_request.orchestrator_url).await?,
                        ),
                        orchestrator_url: update_domain_request.orchestrator_url.clone(),
                        capabilities: update_domain_request.capabilities.clone(),
                        refresh_deadline: update_domain_request.refresh_deadline,
                        counter: update_domain_request.counter,
                        nonce: update_domain_request.nonce,
                    },
                );

                // It is a new orchestration domain. Therefore, we ask the
                // orchestrator to reset to a clean state.
                (Ok(edgeless_api::domain_registration::UpdateDomainResponse::Reset), true)
            }
            Some(desc) => {
                // If the nonce is different: this is a new instance of an
                // orchestration domain, which must be reset.
                // Otherwise, if the counter has not been incremented, then
                // update only the refresh deadline, all the other fields are
                // assumed to remain the same.

                let update_portal_domain;
                let response = if desc.nonce == update_domain_request.nonce && desc.counter == update_domain_request.counter {
                    update_portal_domain = false;
                    edgeless_api::domain_registration::UpdateDomainResponse::Accepted
                } else {
                    log::info!(
                        "Update domain '{}' with {} nodes",
                        update_domain_request.domain_id,
                        update_domain_request.capabilities.num_nodes
                    );

                    desc.capabilities = update_domain_request.capabilities.clone();
                    desc.counter = update_domain_request.counter;

                    // Re-create the client only if needed.
                    if desc.orchestrator_url != update_domain_request.orchestrator_url {
                        desc.orchestrator_url = update_domain_request.orchestrator_url.clone();
                        desc.client =
                            Box::new(edgeless_api::grpc_impl::outer::orc::OrchestratorAPIClient::new(&update_domain_request.orchestrator_url).await?);
                    }

                    update_portal_domain = true;
                    if desc.nonce == update_domain_request.nonce {
                        edgeless_api::domain_registration::UpdateDomainResponse::Accepted
                    } else {
                        desc.nonce = update_domain_request.nonce;
                        edgeless_api::domain_registration::UpdateDomainResponse::Reset
                    }
                };
                desc.refresh_deadline = update_domain_request.refresh_deadline;
                (Ok(response), update_portal_domain)
            }
        };

        if update_portal_domain {
            self.update_portal_domain().await;
        }

        ret
    }

    async fn update_portal_domain(&mut self) {
        self.portal_desc = None;
        for (domain_bal, desc) in &self.orchestrators {
            let domains = desc.capabilities.portal_domains();
            assert!(!domain_bal.is_empty());
            if !domains.is_empty() {
                // Remove those domains that do not advertise a portal resource.
                let mut confirmed_domains = std::collections::HashSet::new();
                for domain in &domains {
                    if let Some(desc) = self.orchestrators.get(domain) {
                        if desc.capabilities.resource_classes.contains("portal") {
                            confirmed_domains.insert(domain.clone());
                        }
                    }
                }

                // If there are no confirmed domains in the portal (or if there
                // is a single domain), skip it as a candidate.
                if confirmed_domains.len() <= 1 {
                    continue;
                }

                // If there are multiple portal candidates, the first one
                // found is considered and the others are ignored.
                if let Some(portal_desc) = &self.portal_desc {
                    log::warn!(
                        "found multiple candidate portal domains: ignoring {}, using {}",
                        domain_bal,
                        portal_desc.domain_bal
                    );
                    continue;
                }

                // Candidate found. We continue with the loop only to warn
                // about possible other candidate portal domains (ignored).
                self.portal_desc = Some(PortalDesc {
                    domain_bal: domain_bal.clone(),
                    domains: confirmed_domains,
                });
            }
        }

        if let Some(portal_desc) = &self.portal_desc {
            log::info!("portal domain {}, with access to {:?}", portal_desc.domain_bal, portal_desc.domains);
        } else {
            log::info!("no portal domain found: cross-domain workflows are not possible");
        }
    }

    /// Migrate a workflow, or a single component, to a target domain.
    ///
    /// If successful, return the new allocation.
    /// Return a response error if the workflow or target domains are not known,
    /// or if the target domain is not compatible with the workflow specs.
    async fn migrate_workflow(
        &mut self,
        request: &edgeless_api::workflow_instance::MigrateWorkflowRequest,
    ) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse> {
        let workflow = if let Some(active_workflow) = self.active_workflows.get(&request.workflow_id) {
            &active_workflow.desired_state
        } else if let Some(workflow) = self.orphan_workflows.get(&request.workflow_id) {
            workflow
        } else {
            return Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: String::from("Unknown workflow id"),
                    detail: Some(request.workflow_id.to_string()),
                },
            ));
        };

        let domain_assignments = if request.component.is_empty() {
            Self::fill_domains(workflow, &request.domain_id)
        } else if let Some(active_workflow) = self.active_workflows.get(&request.workflow_id) {
            let mut cur_assignments = active_workflow.domain_assignments();
            if let Some(cur_domain) = cur_assignments.get_mut(&request.component) {
                if *cur_domain == request.domain_id {
                    return Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: String::from("Ignoring request to migrate component to the same domain"),
                            detail: Some(request.domain_id.to_string()),
                        },
                    ));
                } else {
                    *cur_domain = request.domain_id.clone();
                    cur_assignments
                }
            } else {
                return Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                    edgeless_api::common::ResponseError {
                        summary: String::from("Invalid component name specified in migration request"),
                        detail: Some(request.component.clone()),
                    },
                ));
            }
        } else {
            return Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: String::from("Cannot migrate a single component of an orphan workflow"),
                    detail: Some(request.workflow_id.to_string()),
                },
            ));
        };

        if self.is_domain_assignment_feasible(workflow, &domain_assignments) {
            if let Some(spec) = self.stop_workflow(&request.workflow_id).await {
                match self.relocate_workflow(&request.workflow_id, spec, domain_assignments).await {
                    Ok(response) => {
                        if let edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(_) = response {
                            log::info!(
                                "workflow '{}' {}successfully migrated to domain '{}'",
                                request.workflow_id,
                                if request.component.is_empty() {
                                    String::default()
                                } else {
                                    format!("component '{}' ", request.component)
                                },
                                request.domain_id
                            );
                            Ok(response)
                        } else {
                            panic!(
                                "relocation of the workflow '{}' has triggered a non-implemented sequence",
                                request.workflow_id
                            );
                        }
                    }
                    Err(workflow_request) => {
                        self.orphan_workflows.insert(request.workflow_id.clone(), workflow_request);
                        Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                            edgeless_api::common::ResponseError {
                                summary: String::from("Error when migrating the workflow"),
                                detail: Some(request.workflow_id.to_string()),
                            },
                        ))
                    }
                }
            } else {
                Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                    edgeless_api::common::ResponseError {
                        summary: String::from("Error when terminating the workflow during migration"),
                        detail: Some(request.workflow_id.to_string()),
                    },
                ))
            }
        } else {
            Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: String::from("Migration request cannot be satified"),
                    detail: Some(request.workflow_id.to_string()),
                },
            ))
        }
    }

    async fn refresh(&mut self) {
        log::debug!("Checking domains");

        // Find all domains that are stale, i.e., which have not been
        // refreshed by their own indicated deadline.
        let mut stale_domains = vec![];
        for (domain_id, desc) in &self.orchestrators {
            if std::time::SystemTime::now() > desc.refresh_deadline {
                stale_domains.push(domain_id.clone());
            }
        }

        // Delete all stale domains, also invalidating all mapping of functions
        // and resources of active flows.
        let domains_removed = !stale_domains.is_empty();
        for stale_domain in stale_domains {
            log::info!("Removing domain '{}' because it is stale", stale_domain);
            self.orchestrators.remove(&stale_domain);

            for workflow in &mut self.active_workflows.values_mut() {
                for component in workflow.domain_mapping.values_mut() {
                    if component.domain_id == stale_domain {
                        component.domain_id.clear();
                    }
                }
            }
        }

        // If some domains were removed there might be new orphans, and the
        // portal domain status might have changed.
        if domains_removed {
            self.find_new_orphans().await;
            self.update_portal_domain().await;
        }

        // Try to fix orphans.
        self.try_fix_orphans().await;
    }

    /// Return true if the given orchestration domain is compatible with the
    /// workflow request, i.e., it can host all its functions and resources.
    fn is_workflow_compatible(desc: &OrchestratorDesc, workflow: &edgeless_api::workflow_instance::SpawnWorkflowRequest) -> bool {
        for function in &workflow.workflow_functions {
            if !Self::is_function_compatible(desc, function) {
                return false;
            }
        }
        for resource in &workflow.workflow_resources {
            if !Self::is_resource_compatible(desc, resource) {
                return false;
            }
        }
        true
    }

    /// Return true if the given function is compatible with a domain.
    fn is_function_compatible(desc: &OrchestratorDesc, function: &edgeless_api::workflow_instance::WorkflowFunction) -> bool {
        desc.capabilities
            .runtimes
            .contains(&function.function_class_specification.function_class_type)
    }

    /// Return true if the given resource is compatible with a domain.
    fn is_resource_compatible(desc: &OrchestratorDesc, resource: &edgeless_api::workflow_instance::WorkflowResource) -> bool {
        desc.capabilities.resource_classes.contains(&resource.class_type)
    }

    /// Return a candidate assignment of functions/resources to domains,
    /// including the possibility to use the portal (if any) for inter-domain
    /// workflows, or an empty map if a full mapping is not possible.
    ///
    /// Return immediately an empty map if there is no portal.
    ///
    /// The map returned the function/resource name as key and the domain
    /// selected as value.
    fn domain_assignments_portal(
        &mut self,
        workflow: &edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> std::collections::HashMap<String, String> {
        let mut ret = std::collections::HashMap::new();

        if let Some(portal_desc) = &self.portal_desc {
            assert!(
                portal_desc.domains.len() > 1,
                "too few domains for a portal: {}",
                portal_desc.domains.len()
            );

            for function in &workflow.workflow_functions {
                let compatible_domains = self
                    .orchestrators
                    .iter()
                    .filter(|(domain_id, desc)| portal_desc.domains.contains(*domain_id) && Self::is_function_compatible(desc, function))
                    .map(|(domain_id, _desc)| domain_id.clone())
                    .collect::<Vec<String>>();
                if let Some(domain) = compatible_domains.choose(&mut self.rng) {
                    ret.insert(function.name.clone(), domain.clone());
                } else {
                    return std::collections::HashMap::new();
                }
            }
            for resource in &workflow.workflow_resources {
                let compatible_domains = self
                    .orchestrators
                    .iter()
                    .filter(|(domain_id, desc)| portal_desc.domains.contains(*domain_id) && Self::is_resource_compatible(desc, resource))
                    .map(|(domain_id, _desc)| domain_id.clone())
                    .collect::<Vec<String>>();
                if let Some(domain) = compatible_domains.choose(&mut self.rng) {
                    ret.insert(resource.name.clone(), domain.clone());
                } else {
                    return std::collections::HashMap::new();
                }
            }
        }

        ret
    }

    /// Return true if a given mapping to domains is feasible.
    fn is_domain_assignment_feasible(
        &self,
        workflow: &edgeless_api::workflow_instance::SpawnWorkflowRequest,
        domain_assignments: &std::collections::HashMap<String, String>,
    ) -> bool {
        if domain_assignments.is_empty() || workflow.is_valid().is_err() {
            return false;
        }

        let domains = domain_assignments.values().collect::<std::collections::HashSet<&String>>();

        assert!(!domains.is_empty());
        if domains.len() == 1 {
            // Request to migrate the workflow to a single domain.
            let domain_name = *domains.iter().next().unwrap();
            if let Some(desc) = self.orchestrators.get(domain_name) {
                Self::is_workflow_compatible(desc, workflow)
            } else {
                false
            }
        } else {
            // Request to migrate the workflow to multiple domains.
            if let Some(portal_desc) = &self.portal_desc {
                for component in workflow.source_components() {
                    if let Some(target_domain) = domain_assignments.get(&component) {
                        if !portal_desc.domains.contains(target_domain) {
                            return false;
                        }
                        if let Some(desc) = self.orchestrators.get(target_domain) {
                            if let Some(function) = workflow.get_function(&component) {
                                if !Self::is_function_compatible(desc, function) {
                                    return false;
                                }
                            } else if let Some(resource) = workflow.get_resource(&component) {
                                if !Self::is_resource_compatible(desc, resource) {
                                    return false;
                                }
                            } else {
                                return false;
                            }
                        }
                    } else {
                        return false;
                    }
                }
                true
            } else {
                false
            }
        }
    }

    /// Return the list of orchestration domains that are compatible with the
    /// given workflow request.
    fn workflow_compatible_domains(
        orchestrators: &std::collections::HashMap<String, OrchestratorDesc>,
        workflow_request: &edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> Vec<String> {
        orchestrators
            .iter()
            .filter(|(_domain_id, desc)| Self::is_workflow_compatible(desc, workflow_request))
            .map(|(domain_id, _desc)| domain_id.clone())
            .collect()
    }

    /// Check all active workflows.
    /// If a workflow has at least one resource or function that is not assigned
    /// to a domain, then it is marked as orphan.
    async fn find_new_orphans(&mut self) {
        let mut new_orphans = vec![];
        for (wf_id, workflow) in &self.active_workflows {
            if workflow.is_orphan() {
                new_orphans.push(wf_id.clone());
            }
        }
        for wf_id in new_orphans {
            let active_workflow = self
                .active_workflows
                .remove(&wf_id)
                .expect("Could not find a workflow that must be there");
            let res = self.orphan_workflows.insert(wf_id, active_workflow.desired_state);
            assert!(res.is_none(), "Trying to mark as orphan a workflow that already so");
        }
    }

    /// Try to fix all orphan workflows by stopping it on their current domain
    /// and starting it again on another that compatible with it.
    async fn try_fix_orphans(&mut self) {
        struct WorkflowRequestFixable {
            wf_id: edgeless_api::workflow_instance::WorkflowId,
            workflow_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
            domain_assignments: std::collections::HashMap<String, String>,
        }

        // Find workflows that can be fixed, i.e., assigned to a compatible domain.
        let mut workflow_requests_fixable = vec![];
        let mut workflow_requests_unfixable = std::collections::BTreeMap::new();
        while let Some((wf_id, workflow_request)) = self.orphan_workflows.pop_first() {
            match Self::workflow_compatible_domains(&self.orchestrators, &workflow_request).choose(&mut self.rng) {
                None => {
                    // Try again with multiple domains attached to the portal, if any.
                    let domain_assignments = self.domain_assignments_portal(&workflow_request);
                    if domain_assignments.is_empty() {
                        // The workflow cannot be relocated.
                        workflow_requests_unfixable.insert(wf_id, workflow_request);
                    } else {
                        // The workflow can be relocated to multiple domains.
                        workflow_requests_fixable.push(WorkflowRequestFixable {
                            wf_id,
                            workflow_request,
                            domain_assignments,
                        })
                    }
                }
                Some(new_domain) => {
                    // The workflow can be assigned to a single domain.
                    let domain_assignments = Self::fill_domains(&workflow_request, new_domain);
                    workflow_requests_fixable.push(WorkflowRequestFixable {
                        wf_id,
                        workflow_request,
                        domain_assignments,
                    })
                }
            };
        }
        assert!(self.orphan_workflows.is_empty());

        std::mem::swap(&mut self.orphan_workflows, &mut workflow_requests_unfixable);

        // Try to deploy the orphan workflows to the assigned orchestration
        // domains. If this fails for some workflows, they go back to the
        // orphan list.
        for WorkflowRequestFixable {
            wf_id,
            workflow_request,
            domain_assignments,
        } in workflow_requests_fixable
        {
            match self.relocate_workflow(&wf_id, workflow_request, domain_assignments).await {
                Ok(response) => {
                    if let edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(_) = response {
                        log::info!("orphan workflow {} relocated ", wf_id);
                    }
                }
                Err(workflow_request) => {
                    self.orphan_workflows.insert(wf_id, workflow_request);
                }
            }
        }
    }

    async fn start_workflow_function_in_domain(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        workflow: &mut ActiveWorkflow,
        function: &edgeless_api::workflow_instance::WorkflowFunction,
        domain: &str,
    ) -> Result<(), String> {
        // [TODO] Issue#95
        // The state_specification configuration should be
        // read from the function annotations.
        log::debug!("state specifications currently forced to NodeLocal");
        let response = self
            .fn_client(domain)
            .ok_or(format!("No function client for domain: {}", domain))?
            .start(edgeless_api::function_instance::SpawnFunctionRequest {
                code: function.function_class_specification.clone(),
                annotations: function.annotations.clone(),
                state_specification: edgeless_api::function_instance::StateSpecification {
                    state_id: uuid::Uuid::new_v4(),
                    state_policy: edgeless_api::function_instance::StatePolicy::NodeLocal,
                },
                workflow_id: wf_id.workflow_id.to_string(),
            })
            .await;

        match response {
            Ok(response) => match response {
                edgeless_api::common::StartComponentResponse::ResponseError(error) => {
                    log::warn!("function instance creation rejected: {}", error);
                    Err(format!("function instance creation rejected: {} ", error))
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    log::info!("workflow {} function {} started with fid {}", wf_id.to_string(), function.name, &id);
                    // id.node_id is unused
                    workflow.domain_mapping.insert(
                        function.name.clone(),
                        super::deployment_state::ActiveComponent {
                            component_type: super::ComponentType::Function,
                            name: function.name.clone(),
                            domain_id: domain.to_string(),
                            lid: id,
                        },
                    );
                    Ok(())
                }
            },
            Err(err) => Err(format!("failed interaction when creating a function instance: {}", err)),
        }
    }

    async fn start_workflow_resource_in_domain(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        workflow: &mut ActiveWorkflow,
        resource: &edgeless_api::workflow_instance::WorkflowResource,
        domain: &str,
    ) -> Result<(), String> {
        let response = self
            .resource_client(domain)
            .ok_or(format!("No resource client for domain: {}", domain))?
            .start(edgeless_api::resource_configuration::ResourceInstanceSpecification {
                class_type: resource.class_type.clone(),
                configuration: resource.configurations.clone(),
                workflow_id: wf_id.workflow_id.to_string(),
            })
            .await;

        match response {
            Ok(response) => match response {
                edgeless_api::common::StartComponentResponse::ResponseError(error) => {
                    log::warn!("resource start rejected: {}", error);
                    Err(format!("resource start rejected: {} ", error))
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    log::info!("workflow {} resource {} started with fid {}", wf_id.to_string(), resource.name, &id);
                    // id.node_id is unused
                    workflow.domain_mapping.insert(
                        resource.name.clone(),
                        super::deployment_state::ActiveComponent {
                            component_type: super::ComponentType::Resource,
                            name: resource.name.clone(),
                            domain_id: domain.to_string(),
                            lid: id,
                        },
                    );
                    Ok(())
                }
            },
            Err(err) => Err(format!("failed interaction when starting a resource: {}", err)),
        }
    }

    async fn patch_outputs(
        &mut self,
        origin_domain: &str,
        origin_id: uuid::Uuid,
        origin_type: super::ComponentType,
        output_mapping: std::collections::HashMap<String, edgeless_api::function_instance::InstanceId>,
        name_in_workflow: &str,
    ) -> Result<(), String> {
        match origin_type {
            super::ComponentType::Function => {
                match self
                    .fn_client(origin_domain)
                    .ok_or(format!("No function client for domain: {}", origin_domain))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping,
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("failed interaction when patching component {}: {}", name_in_workflow, err)),
                }
            }
            super::ComponentType::Resource => {
                match self
                    .resource_client(origin_domain)
                    .ok_or(format!("No resource client for domain: {}", origin_domain))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping,
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("failed interaction when patching component {}: {}", name_in_workflow, err)),
                }
            }
        }
    }

    fn fn_client(&mut self, domain: &str) -> Option<Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<uuid::Uuid>>> {
        Some(self.orchestrators.get_mut(domain)?.client.function_instance_api())
    }

    fn resource_client(&mut self, domain: &str) -> Option<Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<uuid::Uuid>>> {
        Some(self.orchestrators.get_mut(domain)?.client.resource_configuration_api())
    }
}

#[cfg(test)]
mod tests {
    use edgeless_api::workflow_instance::SpawnWorkflowRequest;

    use super::*;

    #[tokio::test]
    async fn test_serialize_deserialize_controller_task_state() {
        let mut expected_state = PersistedState { workflows: vec![] };

        let serialized = serde_json::to_string(&expected_state).unwrap();
        let actual_state: PersistedState = serde_json::from_str(&serialized).unwrap();
        assert_eq!(expected_state, actual_state);

        for i in 0..10 {
            let workflow_functions = vec![edgeless_api::workflow_instance::WorkflowFunction {
                name: format!("f{}", i),
                function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                    function_class_id: "test".to_string(),
                    function_class_type: "RUST_WASM".to_string(),
                    function_class_version: "0.1".to_string(),
                    function_class_code: include_bytes!("../../../functions/system_test/system_test.wasm").to_vec(),
                    function_class_outputs: vec!["out1".to_string(), "out2".to_string(), "err".to_string(), "log".to_string()],
                },
                output_mapping: std::collections::HashMap::new(),
                annotations: std::collections::HashMap::new(),
            }];
            let workflow_resources = vec![edgeless_api::workflow_instance::WorkflowResource {
                name: "log".to_string(),
                class_type: "file-log".to_string(),
                output_mapping: std::collections::HashMap::new(),
                configurations: std::collections::HashMap::from([("filename".to_string(), "example.log".to_string())]),
            }];
            let annotations = std::collections::HashMap::from([("ann1".to_string(), "val1".to_string())]);
            let request = SpawnWorkflowRequest {
                workflow_functions,
                workflow_resources,
                annotations,
            };
            expected_state.workflows.push((uuid::Uuid::new_v4().to_string(), request));
        }

        let serialized = serde_json::to_string(&expected_state).unwrap();
        let actual_state: PersistedState = serde_json::from_str(&serialized).unwrap();
        assert_eq!(expected_state, actual_state);
    }
}
