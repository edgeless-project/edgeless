use super::common::CommonConverters;

pub struct FunctonInstanceConverters {}

impl FunctonInstanceConverters {
    pub fn parse_instance_id(api_id: &crate::grpc_impl::api::InstanceId) -> anyhow::Result<crate::function_instance::InstanceId> {
        Ok(crate::function_instance::InstanceId {
            node_id: uuid::Uuid::parse_str(&api_id.node_id)?,
            function_id: uuid::Uuid::parse_str(&api_id.function_id)?,
        })
    }

    pub fn parse_function_class_specification(
        api_spec: &crate::grpc_impl::api::FunctionClassSpecification,
    ) -> anyhow::Result<crate::function_instance::FunctionClassSpecification> {
        Ok(crate::function_instance::FunctionClassSpecification {
            function_class_id: api_spec.function_class_id.clone(),
            function_class_type: api_spec.function_class_type.clone(),
            function_class_version: api_spec.function_class_version.clone(),
            function_class_inlude_code: api_spec.function_class_inline_code().to_vec(),
            output_callback_declarations: api_spec.output_callback_declarations.clone(),
        })
    }

    pub fn parse_api_request(
        api_request: &crate::grpc_impl::api::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::function_instance::SpawnFunctionRequest> {
        Ok(crate::function_instance::SpawnFunctionRequest {
            instance_id: match api_request.instance_id.as_ref() {
                Some(id) => Some(Self::parse_instance_id(id)?),
                None => None,
            },
            code: Self::parse_function_class_specification(match api_request.code.as_ref() {
                Some(val) => val,
                None => {
                    return Err(anyhow::anyhow!("Request does not contain actor class."));
                }
            })?,
            output_callback_definitions: api_request
                .output_callback_definitions
                .iter()
                .filter_map(|(key, value)| {
                    return {
                        match Self::parse_instance_id(&value) {
                            Ok(val) => Some((key.clone(), val)),
                            Err(_) => None,
                        }
                    };
                })
                .collect(),
            annotations: api_request.annotations.clone(),
            state_specification: Self::parse_api_state_specification(match &api_request.state_specification {
                Some(val) => val,
                None => {
                    return Err(anyhow::anyhow!("Request does not contain state_spec."));
                }
            })?,
        })
    }

    pub fn parse_spawn_function_response(
        api_instance: &crate::grpc_impl::api::SpawnFunctionResponse,
    ) -> anyhow::Result<crate::function_instance::SpawnFunctionResponse> {
        Ok(crate::function_instance::SpawnFunctionResponse {
            response_error: match api_instance.response_error.as_ref() {
                Some(val) => Some(match CommonConverters::parse_response_error(val) {
                    Ok(val) => val,
                    Err(err) => return Err(anyhow::anyhow!(err.to_string())),
                }),
                None => None,
            },
            instance_id: match api_instance.instance_id.as_ref() {
                Some(val) => Some(match FunctonInstanceConverters::parse_instance_id(val) {
                    Ok(val) => val,
                    Err(err) => return Err(anyhow::anyhow!(err.to_string())),
                }),
                None => None,
            },
        })
    }

    pub fn parse_api_function_link_update(
        api_update: &crate::grpc_impl::api::UpdateFunctionLinksRequest,
    ) -> anyhow::Result<crate::function_instance::UpdateFunctionLinksRequest> {
        Ok(crate::function_instance::UpdateFunctionLinksRequest {
            instance_id: match api_update.instance_id.as_ref() {
                Some(id) => Some(Self::parse_instance_id(id)?),
                None => None,
            },
            output_callback_definitions: api_update
                .output_callback_definitions
                .iter()
                .filter_map(|(key, value)| {
                    return {
                        match Self::parse_instance_id(&value) {
                            Ok(val) => Some((key.clone(), val)),
                            Err(_) => None,
                        }
                    };
                })
                .collect(),
        })
    }

    pub fn parse_api_state_specification(
        api_spec: &crate::grpc_impl::api::StateSpecification,
    ) -> anyhow::Result<crate::function_instance::StateSpecification> {
        Ok(crate::function_instance::StateSpecification {
            state_id: uuid::Uuid::parse_str(&api_spec.state_id)?,
            state_policy: match api_spec.policy {
                1 => crate::function_instance::StatePolicy::NodeLocal,
                2 => crate::function_instance::StatePolicy::Global,
                _ => crate::function_instance::StatePolicy::Transient,
            },
        })
    }

    pub fn serialize_instance_id(instance_id: &crate::function_instance::InstanceId) -> crate::grpc_impl::api::InstanceId {
        crate::grpc_impl::api::InstanceId {
            node_id: instance_id.node_id.to_string(),
            function_id: instance_id.function_id.to_string(),
        }
    }

    pub fn serialize_function_class_specification(
        spec: &crate::function_instance::FunctionClassSpecification,
    ) -> crate::grpc_impl::api::FunctionClassSpecification {
        crate::grpc_impl::api::FunctionClassSpecification {
            function_class_id: spec.function_class_id.clone(),
            function_class_type: spec.function_class_type.clone(),
            function_class_version: spec.function_class_version.clone(),
            function_class_inline_code: Some(spec.function_class_inlude_code.clone()),
            output_callback_declarations: spec.output_callback_declarations.clone(),
        }
    }

    pub fn serialize_spawn_function_request(req: &crate::function_instance::SpawnFunctionRequest) -> crate::grpc_impl::api::SpawnFunctionRequest {
        crate::grpc_impl::api::SpawnFunctionRequest {
            instance_id: req
                .instance_id
                .as_ref()
                .and_then(|instance_id| Some(Self::serialize_instance_id(instance_id))),
            code: Some(Self::serialize_function_class_specification(&req.code)),
            output_callback_definitions: req
                .output_callback_definitions
                .iter()
                .map(|(key, value)| (key.clone(), Self::serialize_instance_id(&value)))
                .collect(),
            annotations: req.annotations.clone(),
            state_specification: Some(Self::serialize_state_specification(&req.state_specification)),
        }
    }

    pub fn serialize_spawn_function_response(req: &crate::function_instance::SpawnFunctionResponse) -> crate::grpc_impl::api::SpawnFunctionResponse {
        crate::grpc_impl::api::SpawnFunctionResponse {
            response_error: match &req.response_error {
                Some(val) => Some(CommonConverters::serialize_response_error(&val)),
                None => None,
            },
            instance_id: match &req.instance_id {
                Some(val) => Some(Self::serialize_instance_id(&val)),
                None => None,
            },
        }
    }

    pub fn serialize_update_function_links_request(
        crate_update: &crate::function_instance::UpdateFunctionLinksRequest,
    ) -> crate::grpc_impl::api::UpdateFunctionLinksRequest {
        crate::grpc_impl::api::UpdateFunctionLinksRequest {
            instance_id: crate_update
                .instance_id
                .as_ref()
                .and_then(|instance_id| Some(Self::serialize_instance_id(instance_id))),
            output_callback_definitions: crate_update
                .output_callback_definitions
                .iter()
                .map(|(key, value)| (key.clone(), Self::serialize_instance_id(&value)))
                .collect(),
        }
    }

    pub fn serialize_state_specification(crate_spec: &crate::function_instance::StateSpecification) -> crate::grpc_impl::api::StateSpecification {
        crate::grpc_impl::api::StateSpecification {
            state_id: crate_spec.state_id.to_string(),
            policy: match crate_spec.state_policy {
                crate::function_instance::StatePolicy::Transient => crate::grpc_impl::api::StatePolicy::Transient as i32,
                crate::function_instance::StatePolicy::Global => crate::grpc_impl::api::StatePolicy::Global as i32,
                crate::function_instance::StatePolicy::NodeLocal => crate::grpc_impl::api::StatePolicy::NodeLocal as i32,
            },
        }
    }
}

#[derive(Clone)]
pub struct FunctionInstanceAPIClient {
    client: crate::grpc_impl::api::function_instance_client::FunctionInstanceClient<tonic::transport::Channel>,
}

impl FunctionInstanceAPIClient {
    pub async fn new(server_addr: &str) -> Self {
        loop {
            match crate::grpc_impl::api::function_instance_client::FunctionInstanceClient::connect(server_addr.to_string()).await {
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
impl crate::function_instance::FunctionInstanceAPI for FunctionInstanceAPIClient {
    async fn start(
        &mut self,
        request: crate::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::function_instance::SpawnFunctionResponse> {
        let serialized_request = FunctonInstanceConverters::serialize_spawn_function_request(&request);

        let res = self.client.start(tonic::Request::new(serialized_request)).await;
        match res {
            Ok(ret) => FunctonInstanceConverters::parse_spawn_function_response(&ret.into_inner()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while starting a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn stop(&mut self, id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        let serialized_id = FunctonInstanceConverters::serialize_instance_id(&id);
        let res = self.client.stop(tonic::Request::new(serialized_id)).await;
        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn update_links(&mut self, update: crate::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        let serialized_update = FunctonInstanceConverters::serialize_update_function_links_request(&update);

        let res = self.client.update_links(tonic::Request::new(serialized_update)).await;
        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}

pub struct FunctionInstanceAPIServer {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::function_instance::FunctionInstanceAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::function_instance_server::FunctionInstance for FunctionInstanceAPIServer {
    async fn start(
        &self,
        request: tonic::Request<crate::grpc_impl::api::SpawnFunctionRequest>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::SpawnFunctionResponse>, tonic::Status> {
        let inner_request = request.into_inner();
        let parsed_request = match FunctonInstanceConverters::parse_api_request(&inner_request) {
            Ok(val) => val,
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::SpawnFunctionResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Invalid request".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    instance_id: None,
                }))
            }
        };
        let res = self.root_api.lock().await.start(parsed_request).await;
        match res {
            Ok(response) => Ok(tonic::Response::new(FunctonInstanceConverters::serialize_spawn_function_response(
                &response,
            ))),
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::SpawnFunctionResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Request rejected".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    instance_id: None,
                }))
            }
        }
    }

    async fn stop(&self, request: tonic::Request<crate::grpc_impl::api::InstanceId>) -> Result<tonic::Response<()>, tonic::Status> {
        let stop_function_id = match FunctonInstanceConverters::parse_instance_id(&request.into_inner()) {
            Ok(parsed_update) => parsed_update,
            Err(err) => {
                log::error!("Parse Update Failed: {}", err);
                return Err(tonic::Status::invalid_argument("Bad Request"));
            }
        };
        let res = self.root_api.lock().await.stop(stop_function_id).await;
        match res {
            Ok(_fid) => Ok(tonic::Response::new(())),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }

    async fn update_links(
        &self,
        update: tonic::Request<crate::grpc_impl::api::UpdateFunctionLinksRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_update = match FunctonInstanceConverters::parse_api_function_link_update(&update.into_inner()) {
            Ok(parsed_update) => parsed_update,
            Err(err) => {
                log::error!("Parse Update Failed: {}", err);
                return Err(tonic::Status::invalid_argument("Bad Request"));
            }
        };
        let res = self.root_api.lock().await.update_links(parsed_update).await;
        match res {
            Ok(_fid) => Ok(tonic::Response::new(())),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }
}
