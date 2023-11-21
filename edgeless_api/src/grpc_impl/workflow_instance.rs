use super::common::CommonConverters;

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
            name: api_function.name.clone(),
            function_class_specification: crate::grpc_impl::function_instance::FunctonInstanceConverters::parse_function_class_specification(
                match &api_function.class_spec.as_ref() {
                    Some(val) => val,
                    None => return Err(anyhow::anyhow!("Missing Workflow FunctionClass")),
                },
            )?,
            output_mapping: api_function.output_mapping.clone(),
            annotations: api_function.annotations.clone(),
        })
    }

    pub fn parse_workflow_resource(
        api_workflow: &crate::grpc_impl::api::WorkflowResource,
    ) -> anyhow::Result<crate::workflow_instance::WorkflowResource> {
        Ok(crate::workflow_instance::WorkflowResource {
            name: api_workflow.name.clone(),
            class_type: api_workflow.class_type.clone(),
            output_mapping: api_workflow.output_mapping.clone(),
            configurations: api_workflow.configurations.clone(),
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
            workflow_resources: api_request
                .workflow_resources
                .iter()
                .filter_map(|f| match WorkflowInstanceConverters::parse_workflow_resource(f) {
                    Ok(val) => Some(val),
                    Err(_) => None,
                })
                .collect(),
            annotations: api_request.annotations.clone(),
        })
    }

    pub fn parse_workflow_function_mapping(
        api_mapping: &crate::grpc_impl::api::WorkflowFunctionMapping,
    ) -> anyhow::Result<crate::workflow_instance::WorkflowFunctionMapping> {
        Ok(crate::workflow_instance::WorkflowFunctionMapping {
            name: api_mapping.name.to_string(),
            instances: api_mapping
                .instances
                .iter()
                .filter_map(|fun| match CommonConverters::parse_instance_id(fun) {
                    Ok(val) => Some(val),
                    Err(_) => None,
                })
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

    pub fn parse_workflow_spawn_response(
        api_instance: &crate::grpc_impl::api::SpawnWorkflowResponse,
    ) -> anyhow::Result<crate::workflow_instance::SpawnWorkflowResponse> {
        match api_instance.workflow_status.as_ref() {
            Some(val) => match WorkflowInstanceConverters::parse_workflow_instance(val) {
                Ok(val) => Ok(crate::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(val)),
                Err(err) => Err(anyhow::anyhow!(err.to_string())),
            },
            None => match api_instance.response_error.as_ref() {
                Some(val) => match CommonConverters::parse_response_error(val) {
                    Ok(val) => Ok(crate::workflow_instance::SpawnWorkflowResponse::ResponseError(val)),
                    Err(err) => Err(anyhow::anyhow!(err.to_string())),
                },
                None => Err(anyhow::anyhow!(
                    "Ill-formed SpawnWorkflowResponse message: both ResponseError and WorkflowInstance are empty"
                )),
            },
        }
    }

    pub fn parse_workflow_instance_list(
        api_instance: &crate::grpc_impl::api::WorkflowInstanceList,
    ) -> anyhow::Result<Vec<crate::workflow_instance::WorkflowInstance>> {
        let ret: Vec<crate::workflow_instance::WorkflowInstance> = api_instance
            .workflow_statuses
            .iter()
            .map(|x| WorkflowInstanceConverters::parse_workflow_instance(x).unwrap())
            .collect();
        Ok(ret)
    }

    pub fn serialize_workflow_id(crate_id: &crate::workflow_instance::WorkflowId) -> crate::grpc_impl::api::WorkflowId {
        crate::grpc_impl::api::WorkflowId {
            workflow_id: crate_id.workflow_id.to_string(),
        }
    }

    pub fn serialize_workflow_function(crate_function: &crate::workflow_instance::WorkflowFunction) -> crate::grpc_impl::api::WorkflowFunction {
        crate::grpc_impl::api::WorkflowFunction {
            name: crate_function.name.clone(),
            annotations: crate_function.annotations.clone(),
            class_spec: Some(
                crate::grpc_impl::function_instance::FunctonInstanceConverters::serialize_function_class_specification(
                    &crate_function.function_class_specification,
                ),
            ),
            output_mapping: crate_function.output_mapping.clone(),
        }
    }

    pub fn serialize_workflow_resource(crate_resource: &crate::workflow_instance::WorkflowResource) -> crate::grpc_impl::api::WorkflowResource {
        crate::grpc_impl::api::WorkflowResource {
            name: crate_resource.name.clone(),
            class_type: crate_resource.class_type.clone(),
            output_mapping: crate_resource.output_mapping.clone(),
            configurations: crate_resource.configurations.clone(),
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
            workflow_resources: crate_request
                .workflow_resources
                .iter()
                .map(|res| Self::serialize_workflow_resource(res))
                .collect(),
            annotations: crate_request.annotations.clone(),
        }
    }

    pub fn serialize_workflow_spawn_response(
        crate_request: &crate::workflow_instance::SpawnWorkflowResponse,
    ) -> crate::grpc_impl::api::SpawnWorkflowResponse {
        match crate_request {
            crate::workflow_instance::SpawnWorkflowResponse::ResponseError(err) => crate::grpc_impl::api::SpawnWorkflowResponse {
                response_error: Some(CommonConverters::serialize_response_error(&err)),
                workflow_status: None,
            },
            crate::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(instance) => crate::grpc_impl::api::SpawnWorkflowResponse {
                response_error: None,
                workflow_status: Some(Self::serialize_workflow_instance(&instance)),
            },
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

    pub fn serialize_workflow_instance_list(
        instances: &Vec<crate::workflow_instance::WorkflowInstance>,
    ) -> crate::grpc_impl::api::WorkflowInstanceList {
        crate::grpc_impl::api::WorkflowInstanceList {
            workflow_statuses: instances.iter().map(|res| Self::serialize_workflow_instance(res)).collect(),
        }
    }

    pub fn serialize_workflow_function_mapping(
        crate_mapping: &crate::workflow_instance::WorkflowFunctionMapping,
    ) -> crate::grpc_impl::api::WorkflowFunctionMapping {
        crate::grpc_impl::api::WorkflowFunctionMapping {
            name: crate_mapping.name.to_string(),
            instances: crate_mapping
                .instances
                .iter()
                .map(|instance| CommonConverters::serialize_instance_id(instance))
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
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Self { client };
                }
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::workflow_instance::WorkflowInstanceAPI for WorkflowInstanceAPIClient {
    async fn start(
        &mut self,
        request: crate::workflow_instance::SpawnWorkflowRequest,
    ) -> anyhow::Result<crate::workflow_instance::SpawnWorkflowResponse> {
        let ret = self
            .client
            .start(tonic::Request::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_spawn_request(&request),
            ))
            .await;
        match ret {
            Ok(ret) => return crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_spawn_response(&ret.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Communication error while starting a workflow: {}", err.to_string())),
        }
    }
    async fn stop(&mut self, id: crate::workflow_instance::WorkflowId) -> anyhow::Result<()> {
        let ret = self
            .client
            .stop(tonic::Request::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_id(&id),
            ))
            .await;
        match ret {
            Ok(_) => return Ok(()),
            Err(err) => Err(anyhow::anyhow!("Communication error while stopping a workflow: {}", err.to_string())),
        }
    }
    async fn list(&mut self, id: crate::workflow_instance::WorkflowId) -> anyhow::Result<Vec<crate::workflow_instance::WorkflowInstance>> {
        let ret = self
            .client
            .list(tonic::Request::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_id(&id),
            ))
            .await;
        match ret {
            Ok(ret) => return crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_instance_list(&ret.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Communication error while listing workflows: {}", err.to_string())),
        }
    }
}

pub struct WorkflowInstanceAPIServer {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::workflow_instance::WorkflowInstanceAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::workflow_instance_server::WorkflowInstance for WorkflowInstanceAPIServer {
    async fn start(
        &self,
        request: tonic::Request<crate::grpc_impl::api::SpawnWorkflowRequest>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::SpawnWorkflowResponse>, tonic::Status> {
        let req = match crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_spawn_request(&request.into_inner()) {
            Ok(val) => val,
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::SpawnWorkflowResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Invalid request".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    workflow_status: None,
                }))
            }
        };
        let ret = self.root_api.lock().await.start(req).await;
        match ret {
            Ok(response) => Ok(tonic::Response::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_spawn_response(&response),
            )),
            Err(err) => Ok(tonic::Response::new(crate::grpc_impl::api::SpawnWorkflowResponse {
                response_error: Some(crate::grpc_impl::api::ResponseError {
                    summary: "Request rejected".to_string(),
                    detail: Some(err.to_string()),
                }),
                workflow_status: None,
            })),
        }
    }

    async fn stop(&self, request_id: tonic::Request<crate::grpc_impl::api::WorkflowId>) -> Result<tonic::Response<()>, tonic::Status> {
        let req = match crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_id(&request_id.into_inner()) {
            Ok(val) => val,
            Err(err) => {
                return Err(tonic::Status::internal(format!(
                    "Internal error when stopping a workflow: {}",
                    err.to_string()
                )))
            }
        };
        let ret = self.root_api.lock().await.stop(req).await;
        match ret {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!(
                "Internal error when stopping a workflow: {}",
                err.to_string()
            ))),
        }
    }

    async fn list(
        &self,
        request_id: tonic::Request<crate::grpc_impl::api::WorkflowId>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::WorkflowInstanceList>, tonic::Status> {
        let req = match crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_id(&request_id.into_inner()) {
            Ok(val) => val,
            Err(err) => {
                return Err(tonic::Status::internal(format!(
                    "Internal error when listing workflows: {}",
                    err.to_string()
                )))
            }
        };
        let ret = self.root_api.lock().await.list(req).await;
        match ret {
            Ok(instances) => Ok(tonic::Response::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_instance_list(&instances),
            )),
            Err(err) => Err(tonic::Status::internal(format!(
                "Internal error when listing workflows: {}",
                err.to_string()
            ))),
        }
    }
}
