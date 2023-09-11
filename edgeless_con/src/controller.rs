use std::str::FromStr;

use edgeless_api::workflow_instance::WorkflowInstance;
use futures::{Future, SinkExt, StreamExt};

#[cfg(test)]
pub mod test;

pub struct Controller {
    sender: futures::channel::mpsc::UnboundedSender<ControllerRequest>,
}

enum ControllerRequest {
    START(
        edgeless_api::workflow_instance::SpawnWorkflowRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::workflow_instance::WorkflowInstance>>,
    ),
    STOP(edgeless_api::workflow_instance::WorkflowId),
    LIST(
        edgeless_api::workflow_instance::WorkflowId,
        tokio::sync::oneshot::Sender<anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>>>,
    ),
}

struct ResourceHandle {
    resource_type: String,
    _output_callback_declarations: Vec<String>,
    config_api: Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI + Send>,
}

#[derive(Clone)]
struct ActiveWorkflow {
    _desired_state: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    function_instances: std::collections::HashMap<String, Vec<edgeless_api::function_instance::InstanceId>>,
    resource_instances: std::collections::HashMap<String, Vec<(String, edgeless_api::function_instance::InstanceId)>>,
}

impl ActiveWorkflow {
    fn instances(&self, alias: &str) -> Vec<edgeless_api::function_instance::InstanceId> {
        let mut all_instances = Vec::new();
        if let Some(function_instances) = self.function_instances.get(alias) {
            all_instances.append(&mut function_instances.clone());
        }
        if let Some(resource_instances) = self.resource_instances.get(alias) {
            all_instances.extend(&mut resource_instances.iter().map(|(_provider, id)| id.clone()));
        }
        return all_instances;
    }
}

impl Controller {
    pub async fn new_from_config(controller_settings: crate::EdgelessConSettings) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let mut orc_clients = std::collections::HashMap::<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>::new();
        let mut resources = std::collections::HashMap::<String, ResourceHandle>::new();

        for orc in &controller_settings.orchestrators {
            orc_clients.insert(
                orc.domain_id.to_string(),
                Box::new(edgeless_api::grpc_impl::orc::OrchestratorAPIClient::new(&orc.orchestrator_url).await),
            );
        }

        for resource in &controller_settings.resources {
            resources.insert(
                resource.resource_provider_id.clone(),
                ResourceHandle {
                    resource_type: resource.resource_class_type.clone(),
                    _output_callback_declarations: resource.output_callback_declarations.clone(),
                    config_api: Box::new(
                        edgeless_api::grpc_impl::resource_configuration::ResourceConfigurationClient::new(&resource.resource_configuration_url).await,
                    ),
                },
            );
        }

        Self::new(orc_clients, resources)
    }

    fn new(
        orchestrators: std::collections::HashMap<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>,
        resources: std::collections::HashMap<String, ResourceHandle>,
    ) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            Self::main_task(receiver, orchestrators, resources).await;
        });

        (Controller { sender }, main_task)
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<ControllerRequest>,
        orchestrators: std::collections::HashMap<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>,
        resources: std::collections::HashMap<String, ResourceHandle>,
    ) {
        let mut resources = resources;

        let mut receiver = receiver;
        let mut client = match orchestrators.into_values().next() {
            Some(c) => c,
            None => {
                return;
            }
        };

        let mut fn_client = client.function_instance_api();
        let mut active_workflows = std::collections::HashMap::<edgeless_api::workflow_instance::WorkflowId, ActiveWorkflow>::new();

        while let Some(req) = receiver.next().await {
            match req {
                ControllerRequest::START(spawn_workflow_request, reply_sender) => {
                    let mut wf = ActiveWorkflow {
                        _desired_state: spawn_workflow_request.clone(),
                        function_instances: std::collections::HashMap::new(),
                        resource_instances: std::collections::HashMap::new(),
                    };

                    let mut to_upsert = std::collections::HashSet::<String>::new();
                    to_upsert.extend(spawn_workflow_request.workflow_functions.iter().map(|f| f.function_alias.to_string()));
                    to_upsert.extend(spawn_workflow_request.workflow_resources.iter().map(|w| w.alias.to_string()));

                    let mut iteration_count = 100;

                    loop {
                        if iteration_count == 0 || to_upsert.len() == 0 {
                            break;
                        }
                        iteration_count = iteration_count - 1;

                        for fun in &spawn_workflow_request.workflow_functions {
                            if to_upsert.contains(&fun.function_alias) {
                                let outputs: std::collections::HashMap<String, edgeless_api::function_instance::InstanceId> = fun
                                    .output_callback_definitions
                                    .iter()
                                    .filter_map(|(output_id, output_alias)| {
                                        let instances = wf.instances(&output_alias);
                                        if instances.len() > 0 {
                                            Some((output_id.to_string(), instances[0].clone()))
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();

                                let all_outputs_mapped = outputs.len() == fun.output_callback_definitions.len();

                                let state_id = match fun.function_alias.as_str() {
                                    "pinger" => uuid::Uuid::from_str("86699b23-6c24-4ca2-a2a0-b843b7c5e193").unwrap(),
                                    "ponger" => uuid::Uuid::from_str("7dd076cc-2606-40ae-b46b-97628e0094be").unwrap(),
                                    _ => uuid::Uuid::new_v4(),
                                };

                                // Update spawned instance
                                if let Some(existing_instances) = wf.function_instances.get(&fun.function_alias) {
                                    for instance in existing_instances {
                                        let res = fn_client
                                            .update_links(edgeless_api::function_instance::UpdateFunctionLinksRequest {
                                                instance_id: Some(instance.clone()),
                                                output_callback_definitions: outputs.clone(),
                                            })
                                            .await;
                                        match res {
                                            Ok(_) => {
                                                if all_outputs_mapped {
                                                    to_upsert.remove(&fun.function_alias);
                                                }
                                            }
                                            Err(err) => {
                                                log::error!("Unhandled exception during update: {:?}", err);
                                            }
                                        }
                                    }
                                } else {
                                    // Create new instance
                                    let f_id = fn_client
                                        .start(edgeless_api::function_instance::SpawnFunctionRequest {
                                            instance_id: None,
                                            code: fun.function_class_specification.clone(),
                                            annotations: fun.function_annotations.clone(),
                                            output_callback_definitions: outputs.clone(),
                                            state_specification: edgeless_api::function_instance::StateSpecification {
                                                state_id: state_id,
                                                state_policy: edgeless_api::function_instance::StatePolicy::NodeLocal,
                                            },
                                        })
                                        .await;

                                    if let Ok(id) = f_id {
                                        wf.function_instances.insert(fun.function_alias.clone(), vec![id]);
                                        if all_outputs_mapped {
                                            to_upsert.remove(&fun.function_alias);
                                        }
                                    }
                                }
                            }
                        }

                        for resource in &spawn_workflow_request.workflow_resources {
                            if to_upsert.contains(&resource.alias) {
                                let output_mapping: std::collections::HashMap<String, edgeless_api::function_instance::InstanceId> = resource
                                    .output_callback_definitions
                                    .iter()
                                    .map(|(callback, alias)| (callback.to_string(), wf.function_instances.get(alias).unwrap()[0].clone()))
                                    .collect();

                                // Update resource instance
                                if let Some(_instances) = wf.resource_instances.get(&resource.alias) {
                                    // resources currently don't have an update function.
                                    todo!();
                                } else {
                                    // Create new resource instance
                                    if let Some((provider_id, handle)) = resources
                                        .iter_mut()
                                        .find(|(_id, spec)| spec.resource_type == resource.resource_class_type)
                                    {
                                        let wf_id = handle
                                            .config_api
                                            .start(edgeless_api::resource_configuration::ResourceInstanceSpecification {
                                                provider_id: provider_id.clone(),
                                                output_callback_definitions: output_mapping.clone(),
                                                configuration: resource.configurations.clone(),
                                            })
                                            .await;

                                        if let Ok(id) = wf_id {
                                            wf.resource_instances.insert(resource.alias.clone(), vec![(provider_id.clone(), id)]);
                                            if output_mapping.len() == resource.output_callback_definitions.len() {
                                                to_upsert.remove(&resource.alias);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Everything should be mapped now.
                    // Fails if there is invalid mappings or large dependency loops.
                    if to_upsert.len() > 0 {
                        reply_sender.send(Err(anyhow::anyhow!("Failed to resolve alias-links."))).unwrap();
                        continue;
                    }

                    active_workflows.insert(spawn_workflow_request.workflow_id.clone(), wf.clone());
                    match reply_sender.send(Ok(edgeless_api::workflow_instance::WorkflowInstance {
                        workflow_id: spawn_workflow_request.workflow_id,
                        functions: wf
                            .function_instances
                            .iter()
                            .map(|(alias, instances)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                function_alias: alias.to_string(),
                                instances: instances.clone(),
                            })
                            .collect(),
                    })) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {:?}", err);
                        }
                    }
                }
                ControllerRequest::STOP(workflow_id) => {
                    if let Some(workflow_to_remove) = active_workflows.remove(&workflow_id) {
                        for (_alias, instances) in workflow_to_remove.function_instances {
                            for f_id in instances {
                                match fn_client.stop(f_id).await {
                                    Ok(_) => {}
                                    Err(err) => {
                                        log::error!("Unhandled: {}", err);
                                    }
                                }
                            }
                        }
                        for (_alias, instances) in workflow_to_remove.resource_instances {
                            for (provider, instance_id) in instances {
                                match resources.get_mut(&provider) {
                                    Some(provider) => match provider.config_api.stop(instance_id).await {
                                        Ok(()) => {}
                                        Err(err) => {
                                            log::warn!("Stop resource failed: {:?}", err);
                                        }
                                    },
                                    None => {
                                        log::warn!("Provider for previously spawned resource does not exist (anymore).");
                                    }
                                }
                            }
                        }
                    } else {
                        log::warn!("cannot stop non-existing workflow: {:?}", workflow_id);
                    }
                }
                ControllerRequest::LIST(workflow_id, reply_sender) => {
                    let mut ret: Vec<WorkflowInstance> = vec![];
                    if let Some(w_id) = workflow_id.is_valid() {
                        if let Some(wf) = active_workflows.get(&w_id) {
                            ret = vec![WorkflowInstance {
                                workflow_id: w_id.clone(),
                                functions: wf
                                    .function_instances
                                    .iter()
                                    .map(|(alias, instances)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                        function_alias: alias.to_string(),
                                        instances: instances.clone(),
                                    })
                                    .collect(),
                            }];
                        }
                    } else {
                        ret = active_workflows
                            .iter()
                            .map(|(w_id, wf)| WorkflowInstance {
                                workflow_id: w_id.clone(),
                                functions: wf
                                    .function_instances
                                    .iter()
                                    .map(|(alias, instances)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                        function_alias: alias.to_string(),
                                        instances: instances.clone(),
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
    ) -> anyhow::Result<edgeless_api::workflow_instance::WorkflowInstance> {
        let request = request;
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::workflow_instance::WorkflowInstance>>();
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
