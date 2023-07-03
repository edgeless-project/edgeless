pub struct WorkflowInstanceConverters {}

impl WorkflowInstanceConverters {
    pub fn parse_workflow_id(api_id: &crate::grpc_impl::api::WorkflowId) -> anyhow::Result<crate::workflow_instance::WorkflowId> {
        Ok(crate::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::parse_str(&api_id.workflow_id)?,
        })
    }
    pub fn parse_workflow_function(
        api_function: &crate::grpc_impl::api::WorkflowFunction,
    ) -> anyhow::Result<crate::workflow_instance::WorkflowFunction> {
        Ok(crate::workflow_instance::WorkflowFunction {
            function_alias: api_function.function_alias.clone(),
            function_class_specification: crate::grpc_impl::function_instance::FunctonInstanceConverters::parse_function_class_specification(
                match &api_function.function_class.as_ref() {
                    Some(val) => val,
                    None => return Err(anyhow::anyhow!("Missing Workflow FunctionClass")),
                },
            )?,
            output_callback_definitions: api_function.output_callback_definitions.clone(),
            return_continuation: api_function.return_continuation.clone(),
            function_annotations: api_function.function_annotations.clone(),
        })
    }
    pub fn parse_workflow_spawn_request(
        api_request: &crate::grpc_impl::api::SpawnWorkflowRequest,
    ) -> anyhow::Result<crate::workflow_instance::SpawnWorkflowRequest> {
        Ok(crate::workflow_instance::SpawnWorkflowRequest {
            workflow_id: WorkflowInstanceConverters::parse_workflow_id(match api_request.workflow_id.as_ref() {
                Some(val) => val,
                None => {
                    return Err(anyhow::anyhow!("Missing Workflow Id"));
                }
            })?,
            workflow_functions: api_request
                .workflow_functions
                .iter()
                .map(|fun| WorkflowInstanceConverters::parse_workflow_function(fun))
                .filter_map(|f| match f {
                    Ok(val) => Some(val),
                    Err(_) => None,
                })
                .collect(),
            workflow_annotations: api_request.workflow_annotations.clone(),
        })
    }

    pub fn parse_workflow_function_mapping(
        api_mapping: &crate::grpc_impl::api::WorkflowFunctionMapping,
    ) -> anyhow::Result<crate::workflow_instance::WorkflowFunctionMapping> {
        Ok(crate::workflow_instance::WorkflowFunctionMapping {
            function_alias: api_mapping.function_alias.to_string(),
            instances: api_mapping
                .instances
                .iter()
                .filter_map(
                    |fun| match crate::grpc_impl::function_instance::FunctonInstanceConverters::parse_function_id(fun) {
                        Ok(val) => Some(val),
                        Err(_) => None,
                    },
                )
                .collect(),
        })
    }

    pub fn parse_workflow_instance(
        api_instance: &crate::grpc_impl::api::WorkflowInstanceStatus,
    ) -> anyhow::Result<crate::workflow_instance::WorkflowInstance> {
        Ok(crate::workflow_instance::WorkflowInstance {
            workflow_id: WorkflowInstanceConverters::parse_workflow_id(match api_instance.workflow_id.as_ref() {
                Some(val) => val,
                None => {
                    return Err(anyhow::anyhow!("WorkflowId Missing"));
                }
            })?,
            functions: api_instance
                .functions
                .iter()
                .map(|mapping| WorkflowInstanceConverters::parse_workflow_function_mapping(mapping))
                .filter_map(|x| match x {
                    Ok(val) => Some(val),
                    Err(_) => None,
                })
                .collect(),
        })
    }

    pub fn serialize_workflow_id(crate_id: &crate::workflow_instance::WorkflowId) -> crate::grpc_impl::api::WorkflowId {
        crate::grpc_impl::api::WorkflowId {
            workflow_id: crate_id.workflow_id.to_string(),
        }
    }
    pub fn serialize_workflow_function(crate_function: &crate::workflow_instance::WorkflowFunction) -> crate::grpc_impl::api::WorkflowFunction {
        crate::grpc_impl::api::WorkflowFunction {
            function_alias: crate_function.function_alias.clone(),
            function_annotations: crate_function.function_annotations.clone(),
            function_class: Some(
                crate::grpc_impl::function_instance::FunctonInstanceConverters::serialize_function_class_specification(
                    &crate_function.function_class_specification,
                ),
            ),
            output_callback_definitions: crate_function.output_callback_definitions.clone(),
            return_continuation: crate_function.return_continuation.clone(),
        }
    }
    pub fn serialize_workflow_spawn_request(
        crate_request: &crate::workflow_instance::SpawnWorkflowRequest,
    ) -> crate::grpc_impl::api::SpawnWorkflowRequest {
        crate::grpc_impl::api::SpawnWorkflowRequest {
            workflow_id: Some(Self::serialize_workflow_id(&crate_request.workflow_id)),
            workflow_functions: crate_request
                .workflow_functions
                .iter()
                .map(|fun| Self::serialize_workflow_function(fun))
                .collect(),
            workflow_annotations: crate_request.workflow_annotations.clone(),
        }
    }

    pub fn serialize_workflow_instance(crate_instance: &crate::workflow_instance::WorkflowInstance) -> crate::grpc_impl::api::WorkflowInstanceStatus {
        crate::grpc_impl::api::WorkflowInstanceStatus {
            workflow_id: Some(Self::serialize_workflow_id(&crate_instance.workflow_id)),
            functions: crate_instance
                .functions
                .iter()
                .map(|fun_mapping| Self::serialize_workflow_function_mapping(fun_mapping))
                .collect(),
        }
    }

    pub fn serialize_workflow_function_mapping(
        crate_mapping: &crate::workflow_instance::WorkflowFunctionMapping,
    ) -> crate::grpc_impl::api::WorkflowFunctionMapping {
        crate::grpc_impl::api::WorkflowFunctionMapping {
            function_alias: crate_mapping.function_alias.to_string(),
            instances: crate_mapping
                .instances
                .iter()
                .map(|instance| crate::grpc_impl::function_instance::FunctonInstanceConverters::serialize_function_id(instance))
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct WorkflowInstanceAPIClient {
    client: crate::grpc_impl::api::workflow_instance_client::WorkflowInstanceClient<tonic::transport::Channel>,
}

impl WorkflowInstanceAPIClient {
    pub async fn new(server_addr: &str) -> Self {
        loop {
            match crate::grpc_impl::api::workflow_instance_client::WorkflowInstanceClient::connect(server_addr.to_string()).await {
                Ok(client) => return Self { client },
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::workflow_instance::WorkflowInstanceAPI for WorkflowInstanceAPIClient {
    async fn start_workflow_instance(
        &mut self,
        request: crate::workflow_instance::SpawnWorkflowRequest,
    ) -> anyhow::Result<crate::workflow_instance::WorkflowInstance> {
        let ret = self
            .client
            .start_workflow_instance(tonic::Request::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_spawn_request(&request),
            ))
            .await;
        match ret {
            Ok(ret) => return crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_instance(&ret.into_inner()),
            Err(_) => Err(anyhow::anyhow!("Workflow instance server returned error.")),
        }
    }
    async fn stop_workflow_instance(&mut self, id: crate::workflow_instance::WorkflowId) -> anyhow::Result<()> {
        let ret = self
            .client
            .stop_workflow_instance(tonic::Request::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_id(&id),
            ))
            .await;
        match ret {
            Ok(_) => return Ok(()),
            Err(_) => Err(anyhow::anyhow!("Workflow instance server returned error.")),
        }
    }
}

pub struct WorkflowInstanceAPIServer {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::workflow_instance::WorkflowInstanceAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::workflow_instance_server::WorkflowInstance for WorkflowInstanceAPIServer {
    async fn start_workflow_instance(
        &self,
        request: tonic::Request<crate::grpc_impl::api::SpawnWorkflowRequest>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::WorkflowInstanceStatus>, tonic::Status> {
        let req = match crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_spawn_request(&request.into_inner()) {
            Ok(val) => val,
            Err(_) => return Err(tonic::Status::internal("Server Error")),
        };
        let ret = self.root_api.lock().await.start_workflow_instance(req).await;
        match ret {
            Ok(workflow) => Ok(tonic::Response::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_instance(&workflow),
            )),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }

    async fn stop_workflow_instance(
        &self,
        request_id: tonic::Request<crate::grpc_impl::api::WorkflowId>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let req = match crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_id(&request_id.into_inner()) {
            Ok(val) => val,
            Err(_) => return Err(tonic::Status::internal("Server Error")),
        };
        let ret = self.root_api.lock().await.stop_workflow_instance(req).await;
        match ret {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }
}
