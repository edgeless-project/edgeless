// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use futures::StreamExt;
use rand::{seq::SliceRandom, SeedableRng};

pub struct OrchestratorDesc {
    pub client: Box<dyn edgeless_api::outer::orc::OrchestratorAPI>,
    pub orchestrator_url: String,
    pub capabilities: edgeless_api::domain_registration::DomainCapabilities,
    pub refresh_deadline: std::time::SystemTime,
    pub counter: u64,
    pub nonce: u64,
}

pub struct ControllerTask {
    workflow_instance_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
    domain_registration_receiver: futures::channel::mpsc::UnboundedReceiver<super::DomainRegisterRequest>,
    internal_receiver: futures::channel::mpsc::UnboundedReceiver<super::InternalRequest>,
    orchestrators: std::collections::HashMap<String, OrchestratorDesc>,
    active_workflows: std::collections::HashMap<edgeless_api::workflow_instance::WorkflowId, super::deployment_state::ActiveWorkflow>,
    orphan_workflows: std::collections::BTreeMap<edgeless_api::workflow_instance::WorkflowId, edgeless_api::workflow_instance::SpawnWorkflowRequest>,
    rng: rand::rngs::StdRng,
}

impl ControllerTask {
    pub fn new(
        workflow_instance_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
        domain_registration_receiver: futures::channel::mpsc::UnboundedReceiver<super::DomainRegisterRequest>,
        internal_receiver: futures::channel::mpsc::UnboundedReceiver<super::InternalRequest>,
    ) -> Self {
        Self {
            workflow_instance_receiver,
            domain_registration_receiver,
            internal_receiver,
            orchestrators: std::collections::HashMap::new(),
            active_workflows: std::collections::HashMap::new(),
            orphan_workflows: std::collections::BTreeMap::new(),
            rng: rand::rngs::StdRng::from_entropy(),
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
            workflow_instance_receiver,
            domain_registration_receiver,
            internal_receiver,
            orchestrators,
            active_workflows: std::collections::HashMap::new(),
            orphan_workflows: std::collections::BTreeMap::new(),
            rng: rand::rngs::StdRng::from_entropy(),
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
                                Err(_) => Err(anyhow::anyhow!(""))
                            };
                            if let Err(err) = reply_sender.send(reply) {
                                log::error!("Unhandled: {:?}", err);
                            }
                        }
                        super::ControllerRequest::Stop(wf_id) => {
                            self.stop_workflow(&wf_id).await;
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

        // Find a domain that can host all the workflow's functions and
        // resources.
        //
        // [TODO] It is also possible to split a workflow across multiple
        // domains, but this requires an inter-domain dataplane, which is not
        // yet supported as of today (Nov 2024).
        //

        let candidate_domains = Self::compatible_domains(&self.orchestrators, &spawn_workflow_request);
        let target_domain = match candidate_domains.choose(&mut self.rng) {
            Some(val) => val,
            None => {
                return Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                    edgeless_api::common::ResponseError {
                        summary: "Workflow creation failed".to_string(),
                        detail: Some("No single domain supporting all the functions/resources found".to_string()),
                    },
                ));
            }
        };

        // Assign a new identifier to the newly-created workflow.
        let wf_id = edgeless_api::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::new_v4(),
        };

        self.relocate_workflow(&wf_id, spawn_workflow_request, target_domain).await
    }

    async fn relocate_workflow(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        spawn_workflow_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
        target_domain: &str,
    ) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse, edgeless_api::workflow_instance::SpawnWorkflowRequest> {
        self.active_workflows.insert(
            wf_id.clone(),
            super::deployment_state::ActiveWorkflow {
                desired_state: spawn_workflow_request.clone(),
                domain_mapping: std::collections::HashMap::new(),
            },
        );

        let active_workflow = self.active_workflows.get(wf_id).unwrap().clone();

        // Keep the last error.
        let mut res: Result<(), String> = Ok(());

        //
        // First pass: create instances for all the functions and resources.
        //

        // Start the functions on the orchestration domain.
        for function in &spawn_workflow_request.workflow_functions {
            if res.is_err() {
                log::error!("Could not start a function {}", res.clone().unwrap_err());
                break;
            }

            res = self.start_workflow_function_in_domain(wf_id, function, target_domain).await;
        }

        // Start the resources on the orchestration domain.
        for resource in &spawn_workflow_request.workflow_resources {
            if res.is_err() {
                log::error!("Could not start a resource {}", res.clone().unwrap_err());
                break;
            }

            res = self.start_workflow_resource_in_domain(wf_id, resource, target_domain).await;
        }

        //
        // Second pass: patch the workflow, if all the functions
        // have been created successfully.
        //

        // Loop on all the functions and resources of the workflow.
        for component_name in &active_workflow.components() {
            if res.is_err() {
                log::error!("Could not patch the component {}, reason: {}", component_name, res.clone().unwrap_err());
                break;
            }

            // Loop on all the identifiers for this function/resource
            // (once for each orchestration domain to which the
            // function/resource was allocated).
            for origin_fid in self.active_workflows.get_mut(wf_id).unwrap().mapped_fids(component_name).unwrap() {
                let output_mapping = self.output_mapping_for(wf_id, component_name).await;

                if output_mapping.is_empty() {
                    continue;
                }

                let component_type = self.active_workflows.get_mut(wf_id).unwrap().component_type(component_name).unwrap();
                res = self
                    .patch_outputs(target_domain, origin_fid, component_type, output_mapping, component_name)
                    .await;
            }
        }

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
                request: workflow.desired_state.clone(),
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

        match self.orchestrators.get_mut(&update_domain_request.domain_id) {
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
                Ok(edgeless_api::domain_registration::UpdateDomainResponse::Reset)
            }
            Some(desc) => {
                // If the nonce is differen: this is a new instance of an
                // orchestration domain, which must be reset.
                // Otherwise, if the counter has not been incremented, then
                // update only the refresh deadline, all the other fields are
                // assumed to remain the same.

                let response = if desc.nonce == update_domain_request.nonce && desc.counter == update_domain_request.counter {
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

                    if desc.nonce == update_domain_request.nonce {
                        edgeless_api::domain_registration::UpdateDomainResponse::Accepted
                    } else {
                        desc.nonce = update_domain_request.nonce;
                        edgeless_api::domain_registration::UpdateDomainResponse::Reset
                    }
                };
                desc.refresh_deadline = update_domain_request.refresh_deadline;
                Ok(response)
            }
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

        // If some domains were removed there might be new orphans.
        if domains_removed {
            self.find_new_orphans().await;
        }

        // Try to fix orphans.
        self.try_fix_orphans().await;
    }

    /// Return true if the given orchestration domain is compatible with the
    /// workflow request, i.e., it can host all its functions and resources.
    fn is_compatible(desc: &OrchestratorDesc, workflow: &edgeless_api::workflow_instance::SpawnWorkflowRequest) -> bool {
        for function in &workflow.workflow_functions {
            if !desc
                .capabilities
                .runtimes
                .contains(&function.function_class_specification.function_class_type)
            {
                return false;
            }
        }
        for resource in &workflow.workflow_resources {
            if !desc.capabilities.resource_classes.contains(&resource.class_type) {
                return false;
            }
        }
        true
    }

    /// Return the list of orchestration domains that are compatible with the
    /// given workflow request.
    fn compatible_domains(
        orchestrators: &std::collections::HashMap<String, OrchestratorDesc>,
        workflow_request: &edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> Vec<String> {
        let mut ret = vec![];
        for (domain_id, desc) in orchestrators {
            if Self::is_compatible(desc, workflow_request) {
                ret.push(domain_id.clone());
            }
        }
        ret
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
        // Find workflows that can be fixed, i.e., assigned to a compatible domain.
        let mut workflow_requests_fixable = vec![];
        let mut workflow_requests_unfixable = std::collections::BTreeMap::new();
        while let Some((wf_id, workflow_request)) = self.orphan_workflows.pop_first() {
            match Self::compatible_domains(&self.orchestrators, &workflow_request).choose(&mut self.rng) {
                None => {
                    workflow_requests_unfixable.insert(wf_id, workflow_request);
                }
                Some(new_domain) => workflow_requests_fixable.push((new_domain.clone(), wf_id, workflow_request)),
            };
        }
        assert!(self.orphan_workflows.is_empty());

        std::mem::swap(&mut self.orphan_workflows, &mut workflow_requests_unfixable);

        // Try to deploy the orphan workflows to the assigned orchestration
        // domains. If this fails for some workflows, they go back to the
        // orphan list.
        for (new_domain, wf_id, workflow_request) in workflow_requests_fixable {
            assert!(!new_domain.is_empty());
            match self.relocate_workflow(&wf_id, workflow_request, &new_domain).await {
                Ok(response) => {
                    if let edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(_) = response {
                        log::info!("orphan workflow assigned to domain '{}'", new_domain);
                        continue;
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
                instance_id: None,
                code: function.function_class_specification.clone(),
                annotations: function.annotations.clone(),
                state_specification: edgeless_api::function_instance::StateSpecification {
                    state_id: uuid::Uuid::new_v4(),
                    state_policy: edgeless_api::function_instance::StatePolicy::NodeLocal,
                },
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
                    self.active_workflows.get_mut(wf_id).unwrap().domain_mapping.insert(
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
        resource: &edgeless_api::workflow_instance::WorkflowResource,
        domain: &str,
    ) -> Result<(), String> {
        let response = self
            .resource_client(domain)
            .ok_or(format!("No resource client for domain: {}", domain))?
            .start(edgeless_api::resource_configuration::ResourceInstanceSpecification {
                class_type: resource.class_type.clone(),
                configuration: resource.configurations.clone(),
                output_mapping: std::collections::HashMap::new(),
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
                    self.active_workflows.get_mut(wf_id).unwrap().domain_mapping.insert(
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

    async fn output_mapping_for(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        component_name: &str,
    ) -> std::collections::HashMap<String, edgeless_api::function_instance::InstanceId> {
        let workflow_mapping: std::collections::HashMap<String, String> =
            self.active_workflows.get(wf_id).unwrap().component_output_mapping(component_name);

        let mut output_mapping = std::collections::HashMap::new();

        // Loop on all the channels that needed to be
        // mapped for this function/resource.
        for (from_channel, to_name) in workflow_mapping {
            // Loop on all the identifiers for the
            // target function/resource (once for each
            // assigned orchestration domain).
            for target_fid in self.active_workflows.get(wf_id).unwrap().mapped_fids(&to_name).unwrap() {
                // [TODO] Issue#96 The output_mapping
                // structure should be changed so that
                // multiple values are possible (with
                // weights), and this change must be applied
                // to runners, as well.
                // For now, we just keep
                // overwriting the same entry.
                output_mapping.insert(
                    from_channel.clone(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: target_fid,
                    },
                );
            }
        }

        output_mapping
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
