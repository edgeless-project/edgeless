use std::str::FromStr;

use futures::{Future, SinkExt, StreamExt};

pub struct Controller {
    sender: futures::channel::mpsc::UnboundedSender<ControllerRequest>,
    // controller_settings: crate::EdgelessConSettings,
}

enum ControllerRequest {
    START(
        edgeless_api::workflow_instance::SpawnWorkflowRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::workflow_instance::WorkflowInstance>>,
    ),
    STOP(edgeless_api::workflow_instance::WorkflowId),
}

impl Controller {
    pub fn new(controller_settings: crate::EdgelessConSettings) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            Self::main_task(receiver, controller_settings).await;
        });

        (Controller { sender }, main_task)
    }

    async fn main_task(receiver: futures::channel::mpsc::UnboundedReceiver<ControllerRequest>, settings: crate::EdgelessConSettings) {
        let mut orc_clients = std::collections::HashMap::<String, Box<dyn edgeless_api::orc::OrchestratorAPI + Send>>::new();
        for orc in &settings.orchestrators {
            orc_clients.insert(
                orc.domain_id.to_string(),
                Box::new(edgeless_api::grpc_impl::orc::OrchestratorAPIClient::new(&orc.api_addr).await),
            );
        }
        let mut receiver = receiver;
        let mut client = match orc_clients.into_values().next() {
            Some(c) => c,
            None => {
                return;
            }
        };
        let mut fn_client = client.function_instance_api();
        let mut active_workflows = std::collections::HashMap::<String, Vec<edgeless_api::workflow_instance::WorkflowFunctionMapping>>::new();
        while let Some(req) = receiver.next().await {
            match req {
                ControllerRequest::START(spawn_workflow_request, reply_sender) => {
                    let mut f_ids = std::collections::HashMap::<String, edgeless_api::workflow_instance::WorkflowFunctionMapping>::new();
                    let mut to_patch = Vec::<String>::new();
                    for fun in spawn_workflow_request.workflow_functions.clone() {
                        let outputs: std::collections::HashMap<String, edgeless_api::function_instance::FunctionId> = fun
                            .output_callback_definitions
                            .iter()
                            .filter_map(|(output_id, output_alias)| match f_ids.get(output_alias) {
                                Some(mapping) => Some((output_id.to_string(), mapping.instances[0].clone())),
                                None => None,
                            })
                            .collect();
                        if outputs.len() != fun.output_callback_definitions.len() {
                            to_patch.push(fun.function_alias.clone());
                        }

                        let state_id = match fun.function_alias.as_str() {
                            "pinger" => uuid::Uuid::from_str("86699b23-6c24-4ca2-a2a0-b843b7c5e193").unwrap(),
                            "ponger" => uuid::Uuid::from_str("7dd076cc-2606-40ae-b46b-97628e0094be").unwrap(),
                            _ => uuid::Uuid::new_v4(),
                        };

                        let f_id = fn_client
                            .start_function_instance(edgeless_api::function_instance::SpawnFunctionRequest {
                                function_id: None,
                                code: fun.function_class_specification,
                                annotations: fun.function_annotations,
                                output_callback_definitions: outputs,
                                return_continuation: edgeless_api::function_instance::FunctionId::new(uuid::Uuid::new_v4()),
                                state_specification: edgeless_api::function_instance::StateSpecification {
                                    state_id: state_id,
                                    state_policy: edgeless_api::function_instance::StatePolicy::NodeLocal,
                                },
                            })
                            .await;
                        if let Ok(id) = f_id {
                            f_ids.insert(
                                fun.function_alias.clone(),
                                edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                    function_alias: fun.function_alias.clone(),
                                    instances: vec![id],
                                },
                            );
                        }
                    }
                    for workflow_fid_alias in to_patch {
                        if let Some(mapping) = f_ids.get(&workflow_fid_alias) {
                            if let Some(config) = spawn_workflow_request
                                .workflow_functions
                                .iter()
                                .filter(|fun| fun.function_alias == workflow_fid_alias)
                                .next()
                            {
                                for instance in &mapping.instances {
                                    let res = fn_client
                                        .update_function_instance_links(edgeless_api::function_instance::UpdateFunctionLinksRequest {
                                            function_id: Some(instance.clone()),
                                            output_callback_definitions: config
                                                .output_callback_definitions
                                                .iter()
                                                .filter_map(|(output_id, output_alias)| match f_ids.get(output_alias) {
                                                    Some(peer_function) => Some((output_id.to_string(), peer_function.instances[0].clone())),
                                                    None => None,
                                                })
                                                .collect(),
                                            return_continuation: edgeless_api::function_instance::FunctionId::new(uuid::Uuid::new_v4()),
                                        })
                                        .await;
                                    match res {
                                        Ok(_) => {}
                                        Err(err) => {
                                            log::error!("Unhandled: {:?}", err);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    active_workflows.insert(
                        spawn_workflow_request.workflow_id.workflow_id.to_string(),
                        f_ids.clone().into_values().collect(),
                    );
                    match reply_sender.send(Ok(edgeless_api::workflow_instance::WorkflowInstance {
                        workflow_id: spawn_workflow_request.workflow_id,
                        functions: f_ids.into_values().collect(),
                    })) {
                        Ok(_) => {}
                        Err(err) => {
                            log::error!("Unhandled: {:?}", err);
                        }
                    }
                }
                ControllerRequest::STOP(workflow_id) => {
                    if let Some(workflow_functions) = active_workflows.remove(&workflow_id.workflow_id.to_string()) {
                        for mapping in workflow_functions {
                            for f_id in mapping.instances {
                                match fn_client.stop_function_instance(f_id).await {
                                    Ok(_) => {}
                                    Err(err) => {
                                        log::error!("Unhandled: {}", err);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::con::ControllerAPI + Send> {
        Box::new(ControllerClient {
            workflow_instance_client: Box::new(ControllerWorkflowInstanceClient { sender: self.sender.clone() }),
        })
    }
}

pub struct ControllerClient {
    workflow_instance_client: Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>,
}

impl edgeless_api::con::ControllerAPI for ControllerClient {
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
    async fn start_workflow_instance(
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
    async fn stop_workflow_instance(&mut self, id: edgeless_api::workflow_instance::WorkflowId) -> anyhow::Result<()> {
        match self.sender.send(ControllerRequest::STOP(id)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Controller Channel Error")),
        }
    }
}
