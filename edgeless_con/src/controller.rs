use edgeless_api::{
    function_instance::{ComponentId, InstanceId, PatchRequest},
    workflow_instance::{WorkflowId, WorkflowInstance},
};
use futures::{Future, SinkExt, StreamExt};

#[cfg(test)]
pub mod test;

pub struct Controller {
    sender: futures::channel::mpsc::UnboundedSender<ControllerRequest>,
}

enum ControllerRequest {
    START(
        edgeless_api::workflow_instance::SpawnWorkflowRequest,
        // oneshot channel that basically represents the return address for the
        // SpawnWorkflowRequest
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>>,
    ),
    STOP(edgeless_api::workflow_instance::WorkflowId),
    LIST(
        edgeless_api::workflow_instance::WorkflowId,
        tokio::sync::oneshot::Sender<anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>>>,
    ),
}

#[derive(Clone)]
enum ComponentType {
    Function,
    Resource,
}

#[derive(Clone)]
struct ActiveComponent {
    // Function or resource.
    component_type: ComponentType,

    // Name of the function/resource within the workflow.
    name: String,

    // Name of the domain that manages the lifecycle of this function/resource.
    domain_id: String,

    // Identifier returned by the e-ORC.
    fid: ComponentId,
}

impl std::fmt::Display for ActiveComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.component_type {
            ComponentType::Function => write!(f, "function name {}, domain {}, fid {}", self.name, self.domain_id, self.fid),
            ComponentType::Resource => write!(f, "resource name {}, domain {}, fid {}", self.name, self.domain_id, self.fid),
        }
    }
}

#[derive(Clone)]
struct ActiveWorkflow {
    // Workflow as it was requested by the client.
    _desired_state: edgeless_api::workflow_instance::SpawnWorkflowRequest,

    // Mapping of each function/resource to a list of domains.
    domain_mapping: Vec<ActiveComponent>,
}

impl ActiveWorkflow {
    pub fn mapped_fids(&self, name: &str) -> Vec<ComponentId> {
        self.domain_mapping
            .iter()
            .filter(|x| x.name == name)
            .map(|x| x.fid)
            .collect::<Vec<ComponentId>>()
    }
}

impl Controller {
    pub async fn new_from_config(controller_settings: crate::EdgelessConSettings) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        // Connect to all orchestrators.
        let mut orc_clients = std::collections::HashMap::<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>::new();
        for orc in &controller_settings.orchestrators {
            match edgeless_api::grpc_impl::orc::OrchestratorAPIClient::new(&orc.orchestrator_url, Some(1)).await {
                Ok(val) => {
                    orc_clients.insert(orc.domain_id.to_string(), Box::new(val));
                }
                Err(err) => {
                    log::error!("Could not connect to e-ORC {}: {}", &orc.orchestrator_url, err);
                }
            }
        }

        Self::new(orc_clients)
    }

    fn new(
        orchestrators: std::collections::HashMap<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            Self::main_task(receiver, orchestrators).await;
        });

        (Controller { sender }, main_task)
    }

    async fn tear_down_workflow(
        orchestrators: &mut std::collections::HashMap<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>,
        active_workflows: &mut std::collections::HashMap<WorkflowId, ActiveWorkflow>,
        wf_id: &WorkflowId,
    ) {
        let workflow = match active_workflows.get(wf_id) {
            None => {
                log::error!("trying to tear-down a workflow that does not exist: {}", wf_id.to_string());
                return;
            }
            Some(val) => val,
        };

        // Stop all the functions/resources.
        for component in &workflow.domain_mapping {
            let orc_api = match orchestrators.get_mut(&component.domain_id) {
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

            log::debug!("stopping function/resource of workflow {}: {}", wf_id.to_string(), &component);
            match component.component_type {
                ComponentType::Function => match fn_client
                    .stop_function(InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: component.fid.clone(),
                    })
                    .await
                {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("Unhandled: {}", err);
                    }
                },
                ComponentType::Resource => match fn_client
                    .stop_resource(InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: component.fid.clone(),
                    })
                    .await
                {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("Unhandled: {}", err);
                    }
                },
            }
        }

        // Remove the workflow from the active set.
        let remove_res = active_workflows.remove(&wf_id);
        assert!(remove_res.is_some());
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<ControllerRequest>,
        mut orchestrators: std::collections::HashMap<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>,
    ) {
        let mut receiver = receiver;

        if orchestrators.is_empty() {
            log::error!("No orchestration domains configured for this controller");
            return;
        }

        // For now, use the first orchestration domain only and issue a warning
        // if there are more.
        let num_orchestrators = orchestrators.len();
        let orc_entry = orchestrators.iter_mut().next().unwrap();
        let orc_domain = orc_entry.0.clone();
        if num_orchestrators > 1 {
            log::warn!(
                "The controller is configured with {} orchestration domains, but it will use only: {}",
                num_orchestrators,
                orc_domain
            )
        }

        // Gets the FunctionInsatnceAPI object of the selected orchestrator,
        // which can then be used to start / stop / update functions on nodes in
        // its orchestration domain.
        let mut fn_client = orc_entry.1.function_instance_api();

        // This contains the set of active workflows.
        let mut active_workflows = std::collections::HashMap::new();

        // Main loop that reacts to messages on the receiver channel
        while let Some(req) = receiver.next().await {
            match req {
                ControllerRequest::START(spawn_workflow_request, reply_sender) => {
                    log::info!("Annotations ({}) are currently ignored", spawn_workflow_request.annotations.len());

                    // Assign a new identifier to the newly-created workflow.
                    let wf_id = edgeless_api::workflow_instance::WorkflowId {
                        workflow_id: uuid::Uuid::new_v4(),
                    };

                    active_workflows.insert(
                        wf_id.clone(),
                        ActiveWorkflow {
                            _desired_state: spawn_workflow_request.clone(),
                            domain_mapping: vec![],
                        },
                    );
                    let cur_workflow = active_workflows.get_mut(&wf_id).unwrap();

                    // Used to reply to the client.
                    let mut workflow_function_mapping = vec![];

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
                        // [TODO] The state_specification configuration should be
                        // read from the function annotations.
                        log::warn!("state specifications currently forced to NodeLocal");
                        let response = fn_client
                            .start_function(edgeless_api::function_instance::SpawnFunctionRequest {
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
                                    log::error!("function instance creation rejected: {}", error);
                                }
                                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                                    // id.node_id is unused
                                    workflow_function_mapping.push(edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                        name: function.name.clone(),
                                        domain_id: orc_domain.clone(),
                                    });
                                    cur_workflow.domain_mapping.push(ActiveComponent {
                                        component_type: ComponentType::Function,
                                        name: function.name.clone(),
                                        domain_id: orc_domain.clone(),
                                        fid: id.function_id.clone(),
                                    });
                                }
                            },
                            Err(err) => {
                                res = Err(format!("failed interaction when creating a function instance: {}", err.to_string()));
                            }
                        }
                    }

                    // Start the resources on the orchestration domain.
                    for resource in &spawn_workflow_request.workflow_resources {
                        if res.is_err() {
                            break;
                        }
                        let response = fn_client
                            .start_resource(edgeless_api::workflow_instance::WorkflowResource {
                                name: resource.name.clone(),
                                class_type: resource.class_type.clone(),
                                output_mapping: std::collections::HashMap::new(),
                                configurations: resource.configurations.clone(),
                            })
                            .await;

                        match response {
                            Ok(response) => match response {
                                edgeless_api::common::StartComponentResponse::ResponseError(error) => {
                                    log::error!("resource start rejected: {}", error);
                                }
                                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                                    // id.node_id is unused
                                    workflow_function_mapping.push(edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                        name: resource.name.clone(),
                                        domain_id: orc_domain.clone(),
                                    });
                                    cur_workflow.domain_mapping.push(ActiveComponent {
                                        component_type: ComponentType::Resource,
                                        name: resource.name.clone(),
                                        domain_id: orc_domain.clone(),
                                        fid: id.function_id.clone(),
                                    });
                                }
                            },
                            Err(err) => {
                                res = Err(format!("failed interaction when startinga a resource: {}", err.to_string()));
                            }
                        }
                    }

                    //
                    // Second pass: patch the workflow, if all the functions
                    // have been created successfully.
                    //

                    // Collect all the names+output_mapping from the
                    // functions and resources of this workflow.
                    let mut function_resources = std::collections::HashMap::new();
                    for function in &spawn_workflow_request.workflow_functions {
                        function_resources.insert(function.name.clone(), function.output_mapping.clone());
                    }
                    for resource in &spawn_workflow_request.workflow_resources {
                        function_resources.insert(resource.name.clone(), resource.output_mapping.clone());
                    }

                    // Loop on all the functions and resources of the workflow.
                    for (component_name, component_mapping) in function_resources {
                        if res.is_err() {
                            break;
                        }

                        // Loop on all the identifiers for this function/resource
                        // (once for each orchestration domain to which the
                        // function/resource was allocated).
                        for origin_fid in cur_workflow.mapped_fids(&component_name) {
                            // Loop on all the channels that needed to be
                            // mapped for this function/resource.
                            let mut output_mapping = std::collections::HashMap::new();
                            for (from_channel, to_name) in &component_mapping {
                                // Loop on all the identifiers for the
                                // target function/resource (once for each
                                // assigned orchestration domain).
                                for target_fid in cur_workflow.mapped_fids(&to_name) {
                                    // [TODO] The output_mapping structure
                                    // should be changed so that multiple
                                    // values are possible (with weights), and
                                    // this change must be applied to runners,
                                    // as well. For now, we just keep
                                    // overwriting the same entry.
                                    output_mapping.insert(
                                        from_channel.clone(),
                                        InstanceId {
                                            node_id: uuid::Uuid::nil(),
                                            function_id: target_fid,
                                        },
                                    );
                                }
                            }

                            if output_mapping.is_empty() {
                                continue;
                            }
                            match fn_client
                                .patch(PatchRequest {
                                    instance_id: Some(InstanceId {
                                        node_id: uuid::Uuid::nil(),
                                        function_id: origin_fid,
                                    }),
                                    output_mapping,
                                })
                                .await
                            {
                                Ok(_) => {}
                                Err(err) => {
                                    res = Err(format!(
                                        "failed interaction when patching component {}: {}",
                                        &component_name,
                                        err.to_string()
                                    ));
                                }
                            }
                        }
                    }

                    //
                    // If all went OK, notify the client that the workflow
                    // has been accepted.
                    // On the other hand, if something went wrong, we must stop
                    // all the functions and resources that have been started.
                    //

                    if res.is_err() {
                        Self::tear_down_workflow(&mut orchestrators, &mut active_workflows, &wf_id).await;
                    }

                    let reply = match res {
                        Ok(_) => Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(
                            edgeless_api::workflow_instance::WorkflowInstance {
                                workflow_id: wf_id,
                                domain_mapping: workflow_function_mapping,
                            },
                        )),
                        Err(err) => Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(
                            edgeless_api::common::ResponseError {
                                summary: "Workflow creation failed".to_string(),
                                detail: Some(err),
                            },
                        )),
                    };

                    match reply_sender.send(reply) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {:?}", err);
                        }
                    }
                }
                ControllerRequest::STOP(wf_id) => {
                    Self::tear_down_workflow(&mut orchestrators, &mut active_workflows, &wf_id).await;
                }
                ControllerRequest::LIST(workflow_id, reply_sender) => {
                    let mut ret: Vec<WorkflowInstance> = vec![];
                    if let Some(w_id) = workflow_id.is_valid() {
                        if let Some(wf) = active_workflows.get(&w_id) {
                            ret = vec![WorkflowInstance {
                                workflow_id: w_id.clone(),
                                domain_mapping: wf
                                    .domain_mapping
                                    .iter()
                                    .map(|component| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                        name: component.name.to_string(),
                                        domain_id: component.domain_id.clone(),
                                    })
                                    .collect(),
                            }];
                        }
                    } else {
                        ret = active_workflows
                            .iter()
                            .map(|(w_id, wf)| WorkflowInstance {
                                workflow_id: w_id.clone(),
                                domain_mapping: wf
                                    .domain_mapping
                                    .iter()
                                    .map(|component| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                        name: component.name.to_string(),
                                        domain_id: component.domain_id.clone(),
                                    })
                                    .collect(),
                            })
                            .collect();
                    }
                    match reply_sender.send(Ok(ret)) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {:?}", err);
                        }
                    }
                }
            }
        }

        // Main loop that reacts to messages on the receiver channel
        // while let Some(req) = receiver.next().await {
        //     match req {
        //         ControllerRequest::START(spawn_workflow_request, reply_sender) => {
        //             let mut current_workflow = ActiveWorkflow {
        //                 _desired_state: spawn_workflow_request.clone(),
        //                 function_instances: std::collections::HashMap::new(),
        //                 resource_instances: std::collections::HashMap::new(),
        //             };

        //             let mut to_upsert = std::collections::HashSet::<String>::new();
        //             to_upsert.extend(spawn_workflow_request.workflow_functions.iter().map(|wf| wf.name.to_string()));
        //             to_upsert.extend(spawn_workflow_request.workflow_resources.iter().map(|wr| wr.name.to_string()));

        //             let mut iteration_count = 100;

        //             //  This algorithm iterates over all functions/resources
        //             //  until either all output connections are linked or the
        //             //  iteration count (100) is reached. This is required, as
        //             //  we can only get the instance id by spawning the function
        //             //  and because there might be dependency loops. By doing
        //             //  this in multiple iterations (and updating the sets) we
        //             //  can create workflows that also contain loops from the
        //             //  name system (and we don't need to find the order in a
        //             //  loop-free graph). In case there is a loop, the iteration
        //             //  count of 100 will be reached and the workflow creation
        //             //  would fail.
        //             loop {
        //                 if iteration_count == 0 || to_upsert.len() == 0 {
        //                     break;
        //                 }
        //                 iteration_count = iteration_count - 1;

        //                 for fun in &spawn_workflow_request.workflow_functions {
        //                     if to_upsert.contains(&fun.name) {
        //                         let outputs: std::collections::HashMap<String, edgeless_api::function_instance::InstanceId> = fun
        //                             .output_mapping
        //                             .iter()
        //                             .filter_map(|(output_id, output_name)| {
        //                                 let instances = current_workflow.instances(&output_name);
        //                                 if instances.len() > 0 {
        //                                     Some((output_id.to_string(), instances[0].clone()))
        //                                 } else {
        //                                     None
        //                                 }
        //                             })
        //                             .collect();

        //                         let all_outputs_mapped = outputs.len() == fun.output_mapping.len();

        //                         let state_id = match fun.name.as_str() {
        //                             "pinger" => uuid::Uuid::from_str("86699b23-6c24-4ca2-a2a0-b843b7c5e193").unwrap(),
        //                             "ponger" => uuid::Uuid::from_str("7dd076cc-2606-40ae-b46b-97628e0094be").unwrap(),
        //                             _ => uuid::Uuid::new_v4(),
        //                         };

        //                         // Update an existing spawned instance of a
        //                         // function
        //                         if let Some(existing_instances) = current_workflow.function_instances.get(&fun.name) {
        //                             for instance in existing_instances {
        //                                 let res = fn_client
        //                                     .patch(edgeless_api::function_instance::PatchRequest {
        //                                         instance_id: Some(instance.clone()),
        //                                         output_mapping: outputs.clone(),
        //                                     })
        //                                     .await;
        //                                 match res {
        //                                     Ok(_) => {
        //                                         if all_outputs_mapped {
        //                                             to_upsert.remove(&fun.name);
        //                                         }
        //                                     }
        //                                     Err(err) => {
        //                                         log::error!("Unhandled exception during update: {:?}", err);
        //                                     }
        //                                 }
        //                             }
        //                         } else {
        //                             // An instance of this function does not
        //                             // exist yet, create a new one
        //                             let response = fn_client
        //                                 .start(edgeless_api::function_instance::SpawnFunctionRequest {
        //                                     // at this stage we don't specify an
        //                                     // instance_id yet - it will be
        //                                     // assigned by the node running the function
        //                                     instance_id: None,
        //                                     code: fun.function_class_specification.clone(),
        //                                     annotations: fun.annotations.clone(),
        //                                     output_mapping: outputs.clone(),
        //                                     state_specification: edgeless_api::function_instance::StateSpecification {
        //                                         state_id: state_id,
        //                                         state_policy: edgeless_api::function_instance::StatePolicy::NodeLocal,
        //                                     },
        //                                 })
        //                                 .await;

        //                             match response {
        //                                 Ok(response) => match response {
        //                                     edgeless_api::function_instance::StartComponentResponse::ResponseError(error) => {
        //                                         log::error!("function instance creation rejected: {}", error);
        //                                     }
        //                                     edgeless_api::function_instance::StartComponentResponse::InstanceId(id) => {
        //                                         current_workflow.function_instances.insert(fun.name.clone(), vec![id]);
        //                                         if all_outputs_mapped {
        //                                             to_upsert.remove(&fun.name);
        //                                         }
        //                                     }
        //                                 },
        //                                 Err(err) => {
        //                                     log::error!("failed interaction when creating a function instance: {}", err.to_string());
        //                                 }
        //                             }

        //                             // TODO(ccicconetti) handle failed function
        //                             // instance creation
        //                         }
        //                     }
        //                 }

        //                 for resource in &spawn_workflow_request.workflow_resources {
        //                     if to_upsert.contains(&resource.name) {
        //                         let output_mapping: std::collections::HashMap<String, edgeless_api::function_instance::InstanceId> = resource
        //                             .output_mapping
        //                             .iter()
        //                             .map(|(callback, name)| (callback.to_string(), current_workflow.function_instances.get(name).unwrap()[0].clone()))
        //                             .collect();

        //                         // Update resource instance
        //                         if let Some(_instances) = current_workflow.resource_instances.get(&resource.name) {
        //                             // resources currently don't have an update
        //                             // function.
        //                             todo!();
        //                         } else {
        //                             // Create new resource instance
        //                             if let Some((provider_id, handle)) =
        //                                 resources.iter_mut().find(|(_id, spec)| spec.resource_type == resource.class_type)
        //                             {
        //                                 match handle
        //                                     .config_api
        //                                     .start(edgeless_api::resource_configuration::ResourceInstanceSpecification {
        //                                         provider_id: provider_id.clone(),
        //                                         output_mapping: output_mapping.clone(),
        //                                         configuration: resource.configurations.clone(),
        //                                     })
        //                                     .await
        //                                 {
        //                                     Ok(response) => match response {
        //                                         edgeless_api::common::StartComponentResponse::InstanceId(instance_id) => {
        //                                             current_workflow
        //                                                 .resource_instances
        //                                                 .insert(resource.name.clone(), vec![(provider_id.clone(), instance_id)]);
        //                                             if output_mapping.len() == resource.output_mapping.len() {
        //                                                 to_upsert.remove(&resource.name);
        //                                             }
        //                                         }
        //                                         edgeless_api::common::StartComponentResponse::ResponseError(err) => {
        //                                             log::error!("resource creation rejected: {:?}", &err);
        //                                         }
        //                                     },
        //                                     Err(err) => {
        //                                         log::error!("failed interaction when creating a resource: {}", err.to_string());
        //                                     }
        //                                 }
        //                                 // TODO(ccicconetti) handle failed
        //                                 // resource creation
        //                             }
        //                         }
        //                     }
        //                 }
        //             }

        //             // Everything should be mapped now. Fails if there is
        //             // invalid mappings or large dependency loops.
        //             if to_upsert.len() > 0 {
        //                 reply_sender.send(Err(anyhow::anyhow!("Failed to resolve names."))).unwrap();
        //                 continue;
        //             }

        //             // Assign a new identifier to the newly-created workflow.
        //             let wf_id = edgeless_api::workflow_instance::WorkflowId {
        //                 workflow_id: uuid::Uuid::new_v4(),
        //             };

        //             active_workflows.insert(wf_id.clone(), current_workflow.clone());
        //             match reply_sender.send(Ok(edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(
        //                 edgeless_api::workflow_instance::WorkflowInstance {
        //                     workflow_id: wf_id,
        //                     functions: current_workflow
        //                         .function_instances
        //                         .iter()
        //                         .map(|(name, instances)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
        //                             name: name.to_string(),
        //                             instances: instances.clone(),
        //                         })
        //                         .collect(),
        //                 },
        //             ))) {
        //                 Ok(_) => {}
        //                 Err(err) => {
        //                     log::error!("Unhandled: {:?}", err);
        //                 }
        //             }
        //         }
        //         ControllerRequest::STOP(workflow_id) => {
        //             if let Some(workflow_to_remove) = active_workflows.remove(&workflow_id) {
        //                 // Send stop to all function instances associated with
        //                 // this workflow. For now only one orchestrator is
        //                 // supported.
        //                 for (_name, instances) in workflow_to_remove.function_instances {
        //                     for f_id in instances {
        //                         match fn_client.stop(f_id).await {
        //                             Ok(_) => {}
        //                             Err(err) => {
        //                                 log::error!("Unhandled: {}", err);
        //                             }
        //                         }
        //                     }
        //                 }
        //                 // Stop all of the resources using the
        //                 // ResourceConfigurationAPI
        //                 for (_name, instances) in workflow_to_remove.resource_instances {
        //                     for (provider, instance_id) in instances {
        //                         match resources.get_mut(&provider) {
        //                             Some(provider) => match provider.config_api.stop(instance_id).await {
        //                                 Ok(()) => {}
        //                                 Err(err) => {
        //                                     log::warn!("Stop resource failed: {:?}", err);
        //                                 }
        //                             },
        //                             None => {
        //                                 log::warn!("Provider for previously spawned resource does not exist (anymore).");
        //                             }
        //                         }
        //                     }
        //                 }
        //             } else {
        //                 log::warn!("cannot stop non-existing workflow: {:?}", workflow_id);
        //             }
        //         }
        //         ControllerRequest::LIST(workflow_id, reply_sender) => {
        //             let mut ret: Vec<WorkflowInstance> = vec![];
        //             if let Some(w_id) = workflow_id.is_valid() {
        //                 if let Some(wf) = active_workflows.get(&w_id) {
        //                     ret = vec![WorkflowInstance {
        //                         workflow_id: w_id.clone(),
        //                         functions: wf
        //                             .function_instances
        //                             .iter()
        //                             .map(|(name, instances)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
        //                                 name: name.to_string(),
        //                                 instances: instances.clone(),
        //                             })
        //                             .collect(),
        //                     }];
        //                 }
        //             } else {
        //                 ret = active_workflows
        //                     .iter()
        //                     .map(|(w_id, wf)| WorkflowInstance {
        //                         workflow_id: w_id.clone(),
        //                         functions: wf
        //                             .function_instances
        //                             .iter()
        //                             .map(|(name, instances)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
        //                                 name: name.to_string(),
        //                                 instances: instances.clone(),
        //                             })
        //                             .collect(),
        //                     })
        //                     .collect();
        //             }
        //             match reply_sender.send(Ok(ret)) {
        //                 Ok(_) => {}
        //                 Err(err) => {
        //                     log::error!("Unhandled: {:?}", err);
        //                 }
        //             }
        //         }
        //     }
        // }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::controller::ControllerAPI + Send> {
        Box::new(ControllerClient {
            workflow_instance_client: Box::new(ControllerWorkflowInstanceClient { sender: self.sender.clone() }),
        })
    }
}

pub struct ControllerClient {
    workflow_instance_client: Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>,
}

impl edgeless_api::controller::ControllerAPI for ControllerClient {
    fn workflow_instance_api(&mut self) -> Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI> {
        self.workflow_instance_client.clone()
    }
}

#[derive(Clone)]
pub struct ControllerWorkflowInstanceClient {
    sender: futures::channel::mpsc::UnboundedSender<ControllerRequest>,
}

#[async_trait::async_trait]
impl edgeless_api::workflow_instance::WorkflowInstanceAPI for ControllerWorkflowInstanceClient {
    async fn start(
        &mut self,
        request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse> {
        let request = request;
        let (reply_sender, reply_receiver) =
            tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>>();
        match self.sender.send(ControllerRequest::START(request.clone(), reply_sender)).await {
            Ok(_) => {}
            Err(_) => return Err(anyhow::anyhow!("Controller Channel Error")),
        }
        let reply = reply_receiver.await;
        match reply {
            Ok(ret) => ret,
            Err(_) => Err(anyhow::anyhow!("Controller Channel Error")),
        }
    }
    async fn stop(&mut self, id: edgeless_api::workflow_instance::WorkflowId) -> anyhow::Result<()> {
        match self.sender.send(ControllerRequest::STOP(id)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Controller Channel Error")),
        }
    }
    async fn list(&mut self, id: edgeless_api::workflow_instance::WorkflowId) -> anyhow::Result<Vec<WorkflowInstance>> {
        let (reply_sender, reply_receiver) =
            tokio::sync::oneshot::channel::<anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>>>();
        match self.sender.send(ControllerRequest::LIST(id.clone(), reply_sender)).await {
            Ok(_) => {}
            Err(_) => return Err(anyhow::anyhow!("Controller Channel Error")),
        }
        let reply = reply_receiver.await;
        match reply {
            Ok(ret) => ret,
            Err(_) => Err(anyhow::anyhow!("Controller Channel Error")),
        }
    }
}
