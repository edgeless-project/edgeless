// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use futures::StreamExt;

pub struct ControllerTask {
    request_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
    orchestrators: std::collections::HashMap<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>,
    active_workflows: std::collections::HashMap<edgeless_api::workflow_instance::WorkflowId, super::deployment_state::ActiveWorkflow>,
}

impl ControllerTask {
    pub fn new(
        request_receiver: futures::channel::mpsc::UnboundedReceiver<super::ControllerRequest>,
        orchestrators: std::collections::HashMap<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>,
    ) -> Self {
        Self {
            request_receiver,
            orchestrators,
            active_workflows: std::collections::HashMap::new(),
        }
    }

    pub async fn run(&mut self) {
        if self.orchestrators.is_empty() {
            log::error!("No orchestration domains configured for this controller");
            return;
        }

        // For now, use the first orchestration domain only and issue a warning
        // if there are more.
        let num_orchestrators = self.orchestrators.len();
        let orc_entry = self.orchestrators.iter_mut().next().unwrap();
        let orc_domain = orc_entry.0.clone();
        if num_orchestrators > 1 {
            log::warn!(
                "The controller is configured with {} orchestration domains, but it will use only: {}",
                num_orchestrators,
                orc_domain
            )
        }

        self.main_loop().await;
    }

    async fn main_loop(&mut self) {
        while let Some(req) = self.request_receiver.next().await {
            match req {
                super::ControllerRequest::Start(spawn_workflow_request, reply_sender) => {
                    let reply = self.start_workflow(spawn_workflow_request).await;
                    match reply_sender.send(reply) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {:?}", err);
                        }
                    }
                }
                super::ControllerRequest::Stop(wf_id) => {
                    self.stop_workflow(&wf_id).await;
                }
                super::ControllerRequest::List(workflow_id, reply_sender) => {
                    let reply = self.list_workflows(&workflow_id).await;
                    match reply_sender.send(reply) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {:?}", err);
                        }
                    }
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

        self.active_workflows.insert(
            wf_id.clone(),
            super::deployment_state::ActiveWorkflow {
                desired_state: spawn_workflow_request.clone(),
                domain_mapping: std::collections::HashMap::new(),
            },
        );

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

            let function_domain = self.orchestrators.iter_mut().next().unwrap().0.clone();

            res = self.start_workflow_function_in_domain(&wf_id, function, &function_domain).await;
        }

        // Start the resources on the orchestration domain.
        for resource in &spawn_workflow_request.workflow_resources {
            if res.is_err() {
                break;
            }

            let resource_domain = self.orchestrators.iter_mut().next().unwrap().0.clone();

            res = self.start_workflow_resource_in_domain(&wf_id, resource, &resource_domain).await;
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
            for origin_fid in self.active_workflows.get_mut(&wf_id).unwrap().mapped_fids(component_name).unwrap() {
                let origin_domain = self.orchestrators.iter_mut().next().unwrap().0.clone();

                let output_mapping = self.output_mapping_for(&wf_id, component_name).await;

                if output_mapping.is_empty() {
                    continue;
                }

                let component_type = self.active_workflows.get_mut(&wf_id).unwrap().component_type(component_name).unwrap();
                res = self
                    .patch_outputs(&origin_domain, origin_fid, component_type, output_mapping, component_name)
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
        for component in workflow.domain_mapping.values() {
            let orc_api = match self.orchestrators.get_mut(&component.domain_id) {
                None => {
                    log::warn!(
                        "orchestration domain for workflow {} function {} disappeared: {}",
                        wf_id.to_string(),
                        &component.name,
                        &component.domain_id
                    );
                    continue;
                }
                Some(val) => val,
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
                        .values()
                        .map(|component| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                            name: component.name.to_string(),
                            function_id: component.fid,
                            domain_id: component.domain_id.clone(),
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
                        .values()
                        .map(|component| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                            name: component.name.to_string(),
                            function_id: component.fid,
                            domain_id: component.domain_id.clone(),
                        })
                        .collect(),
                })
                .collect();
        }
        Ok(ret)
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
                            fid: id,
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
                            fid: id,
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
        Some(self.orchestrators.get_mut(domain)?.function_instance_api())
    }

    fn resource_client(&mut self, domain: &str) -> Option<Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<uuid::Uuid>>> {
        Some(self.orchestrators.get_mut(domain)?.resource_configuration_api())
    }
}
