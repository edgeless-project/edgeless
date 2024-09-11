// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

// This contains code originally developed in edgeless_orc (also in some of the related files).
// Refer to the orchestrator's history for the history/authorship of those snippets.

use edgeless_api::common::ResponseError;
use futures::StreamExt;

pub struct ControllerTask {
    request_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
    nodes: std::collections::HashMap<edgeless_api::function_instance::NodeId, WorkerNode>,
    active_workflows: std::collections::HashMap<edgeless_api::workflow_instance::WorkflowId, super::deployment_state::ActiveWorkflow>,
    orchestration_logic: crate::orchestration_logic::OrchestrationLogic,
}
pub struct WorkerNode {
    pub agent_url: String,
    pub invocation_url: String,
    pub api: Box<dyn edgeless_api::agent::AgentAPI + Send>,
    pub resource_providers: std::collections::HashMap<String, ResourceProvider>,
    pub capabilities: edgeless_api::node_registration::NodeCapabilities,
    pub health_status: edgeless_api::node_management::HealthStatus,
    pub weight: f32,
}

#[derive(serde::Serialize)]
pub struct ResourceProvider {
    pub class_type: String,
    pub outputs: Vec<String>,
}

impl ControllerTask {
    pub fn new(
        request_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
        orchestrators: std::collections::HashMap<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>,
    ) -> Self {
        Self {
            request_receiver,
            nodes: std::collections::HashMap::new(),
            active_workflows: std::collections::HashMap::new(),
            orchestration_logic: crate::orchestration_logic::OrchestrationLogic::new(crate::orchestration_utils::OrchestrationStrategy::Random),
        }
    }

    pub async fn run(&mut self) {
        self.main_loop().await;
    }

    async fn main_loop(&mut self) {
        let mut check_interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        loop {
            tokio::select! {
                req = self.request_receiver.next() => {
                    if let Some(req) = req {
                        match req {
                            super::ControllerRequest::START(spawn_workflow_request, reply_sender) => {
                                let reply = self.start_workflow(spawn_workflow_request).await;
                                match reply_sender.send(reply) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        log::error!("Unhandled: {:?}", err);
                                    }
                                }
                            }
                            super::ControllerRequest::STOP(wf_id) => {
                                self.stop_workflow(&wf_id).await;
                            }
                            super::ControllerRequest::LIST(workflow_id, reply_sender) => {
                                let reply = self.list_workflows(&workflow_id).await;
                                match reply_sender.send(reply) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        log::error!("Unhandled: {:?}", err);
                                    }
                                }
                            }
                            super::ControllerRequest::UPDATENODE(update, reply_sender) => {
                                let reply = match update {
                                    edgeless_api::node_registration::UpdateNodeRequest::Registration(node_id, agent_url, invocation_url, resource_providers, capabilities) => self.process_node_registration(node_id, agent_url, invocation_url, resource_providers, capabilities).await,
                                    edgeless_api::node_registration::UpdateNodeRequest::Deregistration(node_id) => self.process_node_del(node_id).await,
                                };
                                match reply_sender.send(reply) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        log::error!("Unhandled: {:?}", err);
                                    }
                                }
                            },
                        }
                    }
                },
                _ = check_interval.tick() => {
                    self.periodic_health_check().await;
                }

            }
        }
    }

    async fn start_workflow(
        &mut self,
        spawn_workflow_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse> {
        if !spawn_workflow_request.annotations.is_empty() {
            log::warn!(
                "Workflow annotations ({}) are currently ignored",
                spawn_workflow_request.annotations.len()
            );
        }

        // Assign a new identifier to the newly-created workflow.
        let wf_id = edgeless_api::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::new_v4(),
        };

        let mut wf = super::deployment_state::ActiveWorkflow {
            desired_state: spawn_workflow_request.clone(),
            domain_mapping: std::collections::HashMap::new(),
        };

        wf.optimize_logical();

        self.active_workflows.insert(wf_id.clone(), wf);

        let active_workflow = self.active_workflows.get(&wf_id).unwrap().clone();

        // Keep the last error.
        let mut res: Result<(), String> = Ok(());

        //
        // First pass: create instances for all the functions and resources.
        //

        // Start the functions on the orchestration domain.
        for function in &spawn_workflow_request.workflow_functions {
            if res.is_err() {
                break;
            }

            // let function_domain = self.orchestrators.iter_mut().next().unwrap().0.clone();

            // select node
            let function_node = self.select_node(function).unwrap();

            res = self
                .start_workflow_function_on_node(
                    &wf_id,
                    function,
                    &function_node,
                    active_workflow.active_inputs(&function.name),
                    active_workflow.active_outputs(&function.name),
                )
                .await;
        }

        // Start the resources on the orchestration domain.
        for resource in &spawn_workflow_request.workflow_resources {
            if res.is_err() {
                break;
            }

            let resource_node = self.select_node_for_resource(&resource);

            if let Some(resource_node) = resource_node {
                res = self.start_workflow_resource_on_node(&wf_id, resource, &resource_node).await;
            }
        }

        //
        // Second pass: patch the workflow, if all the functions
        // have been created successfully.
        //

        // Loop on all the functions and resources of the workflow.
        for component_name in &active_workflow.components() {
            if res.is_err() {
                break;
            }

            // Loop on all the identifiers for this function/resource
            // (once for each orchestration domain to which the
            // function/resource was allocated).
            for origin_fid in self.active_workflows.get_mut(&wf_id).unwrap().mapped_fids(&component_name).unwrap() {
                // let origin_domain = self.orchestrators.iter_mut().next().unwrap().0.clone();

                let output_mapping = self.output_mapping_for(&wf_id, &component_name).await;

                if output_mapping.is_empty() {
                    continue;
                }

                let component_type = self.active_workflows.get_mut(&wf_id).unwrap().component_type(&component_name).unwrap();
                res = self
                    .patch_outputs(
                        origin_fid,
                        component_type,
                        output_mapping,
                        std::collections::HashMap::new(),
                        &component_name,
                    )
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
            self.stop_workflow(&wf_id).await;
        }

        let reply = match res {
            Ok(_) => Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(
                edgeless_api::workflow_instance::WorkflowInstance {
                    workflow_id: wf_id.clone(),
                    domain_mapping: self.active_workflows.get(&wf_id).unwrap().domain_mapping(),
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

    async fn stop_workflow(&mut self, wf_id: &edgeless_api::workflow_instance::WorkflowId) {
        let workflow = match self.active_workflows.get(wf_id) {
            None => {
                log::error!("trying to tear-down a workflow that does not exist: {}", wf_id.to_string());
                return;
            }
            Some(val) => val,
        };

        // Stop all the functions/resources.
        for (_, component) in &workflow.domain_mapping {
            let orc_api = match self.nodes.get_mut(&component.fid.node_id) {
                None => {
                    log::warn!(
                        "node for workflow {} function {} disappeared: {}",
                        wf_id.to_string(),
                        &component.name,
                        &component.fid.node_id
                    );
                    continue;
                }
                Some(val) => &mut val.api,
            };
            let mut fn_client = orc_api.function_instance_api();
            let mut resource_client = orc_api.resource_configuration_api();

            log::debug!("stopping function/resource of workflow {}: {}", wf_id.to_string(), &component);
            match component.component_type {
                super::ComponentType::Function => match fn_client.stop(component.fid).await {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("Unhandled: {}", err);
                    }
                },
                super::ComponentType::Resource => match resource_client.stop(component.fid).await {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("Unhandled: {}", err);
                    }
                },
            }
        }

        // Remove the workflow from the active set.
        let remove_res = self.active_workflows.remove(wf_id);
        assert!(remove_res.is_some());
    }

    async fn list_workflows(
        &mut self,
        workflow_id: &edgeless_api::workflow_instance::WorkflowId,
    ) -> anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>> {
        let mut ret: Vec<edgeless_api::workflow_instance::WorkflowInstance> = vec![];
        if let Some(w_id) = workflow_id.is_valid() {
            if let Some(wf) = self.active_workflows.get(w_id) {
                ret = vec![edgeless_api::workflow_instance::WorkflowInstance {
                    workflow_id: w_id.clone(),
                    domain_mapping: wf
                        .domain_mapping
                        .iter()
                        .map(|(_name, component)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                            name: component.name.to_string(),
                            domain_id: component.fid.node_id.to_string(),
                        })
                        .collect(),
                }];
            }
        } else {
            ret = self
                .active_workflows
                .iter()
                .map(|(w_id, wf)| edgeless_api::workflow_instance::WorkflowInstance {
                    workflow_id: w_id.clone(),
                    domain_mapping: wf
                        .domain_mapping
                        .iter()
                        .map(|(_name, component)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                            name: component.name.to_string(),
                            domain_id: component.fid.node_id.to_string(),
                        })
                        .collect(),
                })
                .collect();
        }
        Ok(ret)
    }

    async fn process_node_registration(
        &mut self,
        node_id: uuid::Uuid,
        agent_url: String,
        invocation_url: String,
        resource_providers: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
        capabilities: edgeless_api::node_registration::NodeCapabilities,
    ) -> anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse> {
        
        if let Some(node) = self.nodes.get(&node_id) {
            if node.agent_url == agent_url && node.invocation_url == invocation_url {
                return Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted)
            } else {
                return Ok(edgeless_api::node_registration::UpdateNodeResponse::ResponseError(ResponseError {
                    summary: "Duplicate NodeId with different URL(s).".to_string(),
                    detail: None,
                }))
            }
        }

        let api = Self::get_api_for_url(&agent_url).await;

        let mut node_weight = (std::cmp::max(capabilities.num_cores, capabilities.num_cpus) as f32) * capabilities.clock_freq_cpu;
        if node_weight == 0.0 {
            // Force a vanishing weight to an arbitrary value.
            node_weight = 1.0;
        };

        self.nodes.insert(
            node_id.clone(),
            WorkerNode {
                agent_url,
                invocation_url: invocation_url.clone(),
                api,
                resource_providers: resource_providers
                    .into_iter()
                    .map(|r| {
                        (
                            r.provider_id,
                            ResourceProvider {
                                class_type: r.class_type,
                                outputs: r.outputs,
                            },
                        )
                    })
                    .collect(),
                capabilities,
                health_status: edgeless_api::node_management::HealthStatus::empty(),
                weight: node_weight,
            },
        );

        self.send_peer_updates(vec![edgeless_api::node_management::UpdatePeersRequest::Add(
            node_id.clone(),
            invocation_url,
        )])
        .await;

        // Send information about all nodes to the new node.
        let updates: Vec<_> = self
            .nodes
            .iter()
            .filter_map(|(n_id, n_spec)| {
                if n_id != &node_id {
                    Some(edgeless_api::node_management::UpdatePeersRequest::Add(
                        n_id.clone(),
                        n_spec.invocation_url.clone(),
                    ))
                } else {
                    None
                }
            })
            .collect();
        {
            let n = self.nodes.get_mut(&node_id).unwrap();
            for update in updates {
                n.api.node_management_api().update_peers(update).await.unwrap();
            }
        }

        Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted)
    }

    async fn process_node_del(
        &mut self,
        node_id: edgeless_api::function_instance::NodeId,
    ) -> anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse> {
        if let Some(_) = self.nodes.remove(&node_id) {
            self.send_peer_updates(vec![edgeless_api::node_management::UpdatePeersRequest::Del(node_id)])
                .await;
            Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted)
        } else {
            Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted)
        }
    }

    async fn start_workflow_function_on_node(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        function: &edgeless_api::workflow_instance::WorkflowFunction,
        node_id: &edgeless_api::function_instance::NodeId,
        enabled_inputs: Vec<edgeless_api::function_instance::PortId>,
        enabled_outputs: Vec<edgeless_api::function_instance::PortId>,
    ) -> Result<(), String> {
        let mut function = function.clone();

        let mut enabled_features: Vec<String> = Vec::new();
        for input in enabled_inputs {
            enabled_features.push(format!("input_{}", input.0))
        }
        for output in enabled_outputs {
            enabled_features.push(format!("output_{}", output.0))
        }

        if function.function_class_specification.function_class_type == "RUST" {
            let rust_dir = edgeless_build::unpack_rust_package(&function.function_class_specification.function_class_code).unwrap();
            let wasm_file = edgeless_build::rust_to_wasm(rust_dir, enabled_features, true, false).unwrap();
            let wasm_code = std::fs::read(wasm_file).unwrap();
            function.function_class_specification.function_class_code = wasm_code;
            function.function_class_specification.function_class_type = "RUST_WASM".to_string();
        }

        // [TODO] Issue#95
        // The state_specification configuration should be
        // read from the function annotations.
        log::debug!("state specifications currently forced to NodeLocal");
        let response = self
            .fn_client(node_id)
            .ok_or(format!("No function client for node: {}", node_id))?
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
                    return Err(format!("function instance creation rejected: {} ", error));
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    log::info!("workflow {} function {} started with fid {}", wf_id.to_string(), function.name, &id);
                    // id.node_id is unused
                    self.active_workflows.get_mut(&wf_id).unwrap().domain_mapping.insert(
                        function.name.clone(),
                        super::deployment_state::ActiveComponent {
                            component_type: super::ComponentType::Function,
                            name: function.name.clone(),
                            // domain_id: domain.to_string(),
                            fid: id,
                        },
                    );
                    return Ok(());
                }
            },
            Err(err) => {
                return Err(format!("failed interaction when creating a function instance: {}", err));
            }
        }
    }

    async fn start_workflow_resource_on_node(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        resource: &edgeless_api::workflow_instance::WorkflowResource,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Result<(), String> {
        let response = self
            .resource_client(node_id)
            .ok_or(format!("No resource client for node: {}", node_id))?
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
                    return Err(format!("resource start rejected: {} ", error));
                }
                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                    log::info!("workflow {} resource {} started with fid {}", wf_id.to_string(), resource.name, &id);
                    // id.node_id is unused
                    self.active_workflows.get_mut(&wf_id).unwrap().domain_mapping.insert(
                        resource.name.clone(),
                        super::deployment_state::ActiveComponent {
                            component_type: super::ComponentType::Resource,
                            name: resource.name.clone(),
                            // domain_id: domain.to_string(),
                            fid: id,
                        },
                    );
                    return Ok(());
                }
            },
            Err(err) => {
                return Err(format!("failed interaction when starting a resource: {}", err));
            }
        }
    }

    async fn periodic_health_check(&mut self) {
        // First check if there are nodes that must be disconnected
        // because they failed to reply to a keep-alive.
        let to_be_disconnected = self.find_dead_nodes().await;

        // Second, remove all those nodes from the map of clients.
        for node_id in to_be_disconnected.iter() {
            log::info!("disconnected node not replying to keep-alive: {}", &node_id);
            let val = self.nodes.remove(&node_id);
            assert!(val.is_some());
        }

        // Update the peers of (still alive) nodes by
        // deleting the missing-in-action peers.
        for removed_node_id in &to_be_disconnected {
            for (_, client_desc) in self.nodes.iter_mut() {
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

        self.handle_node_removal(&to_be_disconnected).await;
    }

    async fn find_dead_nodes(&mut self) -> std::collections::HashSet<edgeless_api::function_instance::NodeId> {
        let mut dead_nodes = std::collections::HashSet::new();
        for (node_id, client_desc) in &mut self.nodes {
            match client_desc.api.node_management_api().keep_alive().await {
                Ok(health_status) => {
                    client_desc.health_status = health_status;
                }
                Err(_) => {
                    dead_nodes.insert(node_id.clone());
                }
            };
        }
        dead_nodes
    }

    async fn handle_node_removal(&mut self, removed_nodes: &std::collections::HashSet<edgeless_api::function_instance::NodeId>) {
        for wf_id in self
            .active_workflows
            .keys()
            .map(|k| k.clone())
            .collect::<Vec<edgeless_api::workflow_instance::WorkflowId>>()
        {
            self.handle_node_removal_for_workflow(removed_nodes, wf_id.clone()).await;
        }
    }

    async fn handle_node_removal_for_workflow(
        &mut self,
        removed_nodes: &std::collections::HashSet<edgeless_api::function_instance::NodeId>,
        wf_id: edgeless_api::workflow_instance::WorkflowId,
    ) {
        let old_state = self.active_workflows.get(&wf_id).unwrap().clone();

        let mut lost_instances = Vec::<(String, crate::controller::ComponentType)>::new();
        let mut damaged_instances = std::collections::HashSet::<String>::new();

        self.active_workflows.get_mut(&wf_id).unwrap().domain_mapping.retain(|cname, instance| {
            if removed_nodes.contains(&instance.fid.node_id) {
                lost_instances.push((cname.clone(), instance.component_type.clone()));
                false
            } else {
                true
            }
        });

        for (component_name, component_type) in lost_instances {
            damaged_instances.extend(old_state.inputs_for(&component_name));

            match component_type {
                super::ComponentType::Function => {
                    let function = old_state
                        .desired_state
                        .workflow_functions
                        .iter()
                        .find(|f| f.name == component_name)
                        .unwrap()
                        .clone();

                    let function_node = self.select_node(&function).unwrap();

                    let res = self
                        .start_workflow_function_on_node(
                            &wf_id,
                            &function,
                            &function_node,
                            old_state.active_inputs(&function.name),
                            old_state.active_outputs(&function.name),
                        )
                        .await;

                    if let Err(e) = res {
                        log::error!("Error Spawning Replacement: {}!", e);
                    }
                }
                super::ComponentType::Resource => {
                    let resource = old_state
                        .desired_state
                        .workflow_resources
                        .iter()
                        .find(|r| r.name == component_name)
                        .unwrap()
                        .clone();

                    let resource_node = self.select_node_for_resource(&resource);

                    if let Some(resource_node) = resource_node {
                        let res = self.start_workflow_resource_on_node(&wf_id, &resource, &resource_node).await;
                        if let Err(e) = res {
                            log::error!("Error Spawning Replacement: {}!", e);
                        }
                    }
                }
            }
        }

        for component_name in &damaged_instances {
            let compoent_id = self
                .active_workflows
                .get_mut(&wf_id)
                .unwrap()
                .domain_mapping
                .get(component_name)
                .unwrap()
                .fid;
            let component_type = old_state.component_type(component_name).unwrap();
            let output_mapping = self.output_mapping_for(&wf_id, component_name).await;

            self.patch_outputs(
                compoent_id,
                component_type,
                output_mapping,
                std::collections::HashMap::new(),
                &component_name,
            )
            .await
            .unwrap();
        }
    }

    async fn output_mapping_for(
        &mut self,
        wf_id: &edgeless_api::workflow_instance::WorkflowId,
        component_name: &str,
    ) -> std::collections::HashMap<String, edgeless_api::common::Output> {
        let workflow_mapping: std::collections::HashMap<String, super::deployment_state::LogicalOutput> =
            self.active_workflows.get(wf_id).unwrap().component_output_mapping(&component_name);

        let mut output_mapping = std::collections::HashMap::new();

        // Loop on all the channels that needed to be
        // mapped for this function/resource.
        for (from_channel, logical_output) in workflow_mapping {
            // Loop on all the identifiers for the
            // target function/resource (once for each
            // assigned orchestration domain).
            match logical_output {
                super::deployment_state::LogicalOutput::Single((component_name, port_id)) => {
                    let fids = self.active_workflows.get(&wf_id).unwrap().mapped_fids(&component_name).unwrap();

                    assert!(fids.len() == 1);

                    output_mapping.insert(from_channel.clone(), edgeless_api::common::Output::Single(fids[0], port_id));
                }
                super::deployment_state::LogicalOutput::Any(ids) => {
                    let mut all_fids = Vec::new();
                    for (component_name, port_id) in ids {
                        let mut fids = self
                            .active_workflows
                            .get(&wf_id)
                            .unwrap()
                            .mapped_fids(&component_name)
                            .unwrap()
                            .iter()
                            .map(|x| (x.clone(), port_id.clone()))
                            .collect();
                        all_fids.append(&mut fids);
                    }
                    output_mapping.insert(
                        from_channel.clone(),
                        edgeless_api::common::Output::Any(all_fids.iter().map(|(fid, port)| (fid.clone(), port.clone())).collect()),
                    );
                }
                super::deployment_state::LogicalOutput::All(ids) => {
                    let mut all_fids = Vec::new();
                    for (component_name, port_id) in ids {
                        let mut fids = self
                            .active_workflows
                            .get(&wf_id)
                            .unwrap()
                            .mapped_fids(&component_name)
                            .unwrap()
                            .iter()
                            .map(|x| (x.clone(), port_id.clone()))
                            .collect();
                        all_fids.append(&mut fids);
                    }
                    output_mapping.insert(
                        from_channel.clone(),
                        edgeless_api::common::Output::All(all_fids.iter().map(|(fid, port)| (fid.clone(), port.clone())).collect()),
                    );
                }
            }
        }

        output_mapping
    }

    async fn patch_outputs(
        &mut self,
        origin_id: edgeless_api::function_instance::InstanceId,
        origin_type: super::ComponentType,
        output_mapping: std::collections::HashMap<String, edgeless_api::common::Output>,
        input_mapping: std::collections::HashMap<String, edgeless_api::common::Input>,
        name_in_workflow: &str,
    ) -> Result<(), String> {
        match origin_type {
            super::ComponentType::Function => {
                match self
                    .fn_client(&origin_id.node_id)
                    .ok_or(format!("No function client for node: {}", origin_id.node_id))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping,
                        input_mapping,
                    })
                    .await
                {
                    Ok(_) => return Ok(()),
                    Err(err) => {
                        return Err(format!("failed interaction when patching component {}: {}", name_in_workflow, err));
                    }
                }
            }
            super::ComponentType::Resource => {
                match self
                    .resource_client((&origin_id.node_id))
                    .ok_or(format!("No resource client for node: {}", origin_id.node_id))?
                    .patch(edgeless_api::common::PatchRequest {
                        function_id: origin_id,
                        output_mapping,
                        input_mapping,
                    })
                    .await
                {
                    Ok(_) => return Ok(()),
                    Err(err) => {
                        return Err(format!("failed interaction when patching component {}: {}", name_in_workflow, err));
                    }
                }
            }
        }
    }

    fn fn_client(
        &mut self,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>>> {
        Some(self.nodes.get_mut(node_id)?.api.function_instance_api())
    }

    fn resource_client(
        &mut self,
        node_id: &edgeless_api::function_instance::NodeId,
    ) -> Option<Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>> {
        Some(self.nodes.get_mut(node_id)?.api.resource_configuration_api())
    }

    fn select_node(
        &mut self,
        spawn_req: &edgeless_api::workflow_instance::WorkflowFunction,
    ) -> anyhow::Result<edgeless_api::function_instance::NodeId> {
        match self.orchestration_logic.next(&self.nodes, spawn_req) {
            Some(node_id) => Ok(node_id),
            None => Err(anyhow::anyhow!("no valid node found")),
        }
    }

    fn select_node_for_resource(
        &self,
        resource: &edgeless_api::workflow_instance::WorkflowResource,
    ) -> Option<edgeless_api::function_instance::NodeId> {
        if let Some((id, _)) = self
            .nodes
            .iter()
            .find(|(_, n)| n.resource_providers.iter().find(|(_, r)| r.class_type == resource.class_type).is_some())
        {
            Some(id.clone())
        } else {
            None
        }
    }

    async fn send_peer_updates(&mut self, updates: Vec<edgeless_api::node_management::UpdatePeersRequest>) {
        for (n_id, n_spec) in &mut self.nodes {
            for update in &updates {
                let is_self = match update {
                    edgeless_api::node_management::UpdatePeersRequest::Add(id, _) => id == n_id,
                    edgeless_api::node_management::UpdatePeersRequest::Del(id) => id == n_id,
                    edgeless_api::node_management::UpdatePeersRequest::Clear => false,
                };

                if !is_self {
                    n_spec.api.node_management_api().update_peers(update.clone()).await.unwrap();
                }
            }
        }
    }

    async fn get_api_for_url(agent_url: &str) -> Box<dyn edgeless_api::agent::AgentAPI + Send> {
        let (proto, host, port) = edgeless_api::util::parse_http_host(&agent_url).unwrap();
        match proto {
            edgeless_api::util::Proto::COAP => {
                let addr = std::net::SocketAddrV4::new(host.parse().unwrap(), port);
                Box::new(edgeless_api::coap_impl::CoapClient::new(addr).await)
            }
            _ => Box::new(edgeless_api::grpc_impl::agent::AgentAPIClient::new(&agent_url).await),
        }
    }
}
