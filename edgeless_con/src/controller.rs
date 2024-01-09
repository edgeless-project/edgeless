use edgeless_api::{
    common::PatchRequest,
    function_instance::{ComponentId, InstanceId},
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
            let mut resource_client = orc_api.resource_configuration_api();

            log::debug!("stopping function/resource of workflow {}: {}", wf_id.to_string(), &component);
            match component.component_type {
                ComponentType::Function => match fn_client.stop(component.fid.clone()).await {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("Unhandled: {}", err);
                    }
                },
                ComponentType::Resource => match resource_client.stop(component.fid.clone()).await {
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

        let mut resource_client = orc_entry.1.resource_configuration_api();

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
                        // [TODO] Issue#95
                        // The state_specification configuration should be
                        // read from the function annotations.
                        log::warn!("state specifications currently forced to NodeLocal");
                        let response = fn_client
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
                                    res = Err(format!("function instance creation rejected: {} ", error));
                                }
                                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                                    log::info!("workflow {} function {} started with fid {}", wf_id.to_string(), function.name, &id);
                                    // id.node_id is unused
                                    workflow_function_mapping.push(edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                        name: function.name.clone(),
                                        domain_id: orc_domain.clone(),
                                    });
                                    cur_workflow.domain_mapping.push(ActiveComponent {
                                        component_type: ComponentType::Function,
                                        name: function.name.clone(),
                                        domain_id: orc_domain.clone(),
                                        fid: id.clone(),
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
                        let response = resource_client
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
                                    res = Err(format!("resource start rejected: {} ", error));
                                }
                                edgeless_api::common::StartComponentResponse::InstanceId(id) => {
                                    log::info!("workflow {} resource {} started with fid {}", wf_id.to_string(), resource.name, &id);
                                    // id.node_id is unused
                                    workflow_function_mapping.push(edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                        name: resource.name.clone(),
                                        domain_id: orc_domain.clone(),
                                    });
                                    cur_workflow.domain_mapping.push(ActiveComponent {
                                        component_type: ComponentType::Resource,
                                        name: resource.name.clone(),
                                        domain_id: orc_domain.clone(),
                                        fid: id.clone(),
                                    });
                                }
                            },
                            Err(err) => {
                                res = Err(format!("failed interaction when starting a resource: {}", err.to_string()));
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
                                    // [TODO] Issue#96 The output_mapping
                                    // structure should be changed so that
                                    // multiple values are possible (with
                                    // weights), and this change must be applied
                                    // to runners, as well.
                                    // For now, we just keep
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
                                    function_id: origin_fid,
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
