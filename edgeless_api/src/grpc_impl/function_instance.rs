pub struct FunctonInstanceConverters {}

impl FunctonInstanceConverters {
    pub fn parse_function_id(api_id: &crate::grpc_impl::api::FunctionId) -> anyhow::Result<crate::function_instance::FunctionId> {
        Ok(crate::function_instance::FunctionId {
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
            function_id: match api_request.function_id.as_ref() {
                Some(id) => Some(Self::parse_function_id(id)?),
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
                        match Self::parse_function_id(&value) {
                            Ok(val) => Some((key.clone(), val)),
                            Err(_) => None,
                        }
                    };
                })
                .collect(),
            return_continuation: Self::parse_function_id(match &api_request.return_continuation.as_ref() {
                Some(val) => val,
                None => {
                    return Err(anyhow::anyhow!("Request does not contain continuation."));
                }
            })?,
            annotations: api_request.annotations.clone(),
        })
    }

    pub fn parse_api_function_link_update(
        api_update: &crate::grpc_impl::api::UpdateFunctionLinksRequest,
    ) -> anyhow::Result<crate::function_instance::UpdateFunctionLinksRequest> {
        Ok(crate::function_instance::UpdateFunctionLinksRequest {
            function_id: match api_update.function_id.as_ref() {
                Some(id) => Some(Self::parse_function_id(id)?),
                None => None,
            },
            output_callback_definitions: api_update
                .output_callback_definitions
                .iter()
                .filter_map(|(key, value)| {
                    return {
                        match Self::parse_function_id(&value) {
                            Ok(val) => Some((key.clone(), val)),
                            Err(_) => None,
                        }
                    };
                })
                .collect(),
            return_continuation: Self::parse_function_id(match &api_update.return_continuation.as_ref() {
                Some(val) => val,
                None => {
                    return Err(anyhow::anyhow!("Update does not contain continuation."));
                }
            })?,
        })
    }

    pub fn serialize_function_id(function_id: &crate::function_instance::FunctionId) -> crate::grpc_impl::api::FunctionId {
        crate::grpc_impl::api::FunctionId {
            node_id: function_id.node_id.to_string(),
            function_id: function_id.function_id.to_string(),
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
            function_id: req.function_id.as_ref().and_then(|fid| Some(Self::serialize_function_id(fid))),
            code: Some(Self::serialize_function_class_specification(&req.code)),
            output_callback_definitions: req
                .output_callback_definitions
                .iter()
                .map(|(key, value)| (key.clone(), Self::serialize_function_id(&value)))
                .collect(),
            return_continuation: Some(Self::serialize_function_id(&req.return_continuation)),
            annotations: req.annotations.clone(),
        }
    }

    pub fn serialize_update_function_links_request(
        crate_update: &crate::function_instance::UpdateFunctionLinksRequest,
    ) -> crate::grpc_impl::api::UpdateFunctionLinksRequest {
        crate::grpc_impl::api::UpdateFunctionLinksRequest {
            function_id: crate_update.function_id.as_ref().and_then(|fid| Some(Self::serialize_function_id(fid))),
            output_callback_definitions: crate_update
                .output_callback_definitions
                .iter()
                .map(|(key, value)| (key.clone(), Self::serialize_function_id(&value)))
                .collect(),
            return_continuation: Some(Self::serialize_function_id(&crate_update.return_continuation)),
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
                Ok(client) => return Self { client },
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::function_instance::FunctionInstanceAPI for FunctionInstanceAPIClient {
    async fn start_function_instance(
        &mut self,
        request: crate::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::function_instance::FunctionId> {
        let serialized_request = FunctonInstanceConverters::serialize_spawn_function_request(&request);

        let res = self.client.start_function_instance(tonic::Request::new(serialized_request)).await;
        match res {
            Ok(function_id) => FunctonInstanceConverters::parse_function_id(&function_id.into_inner()),
            Err(_) => Err(anyhow::anyhow!("Start Request Failed")),
        }
    }

    async fn stop_function_instance(&mut self, id: crate::function_instance::FunctionId) -> anyhow::Result<()> {
        let serialized_id = FunctonInstanceConverters::serialize_function_id(&id);
        let res = self.client.stop_function_instance(tonic::Request::new(serialized_id)).await;
        match res {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Stop Request Failed")),
        }
    }

    async fn update_function_instance_links(&mut self, update: crate::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        let serialized_update = FunctonInstanceConverters::serialize_update_function_links_request(&update);

        let res = self.client.update_function_instance_links(tonic::Request::new(serialized_update)).await;
        match res {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Start Request Failed")),
        }
    }
}

pub struct FunctionInstanceAPIServer {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::function_instance::FunctionInstanceAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::function_instance_server::FunctionInstance for FunctionInstanceAPIServer {
    async fn start_function_instance(
        &self,
        request: tonic::Request<crate::grpc_impl::api::SpawnFunctionRequest>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::FunctionId>, tonic::Status> {
        let inner_request = request.into_inner();
        let parsed_request = match FunctonInstanceConverters::parse_api_request(&inner_request) {
            Ok(val) => val,
            Err(err) => {
                log::error!("Parse Request Failed: {}", err);
                return Err(tonic::Status::invalid_argument("Bad Request"));
            }
        };
        let res = self.root_api.lock().await.start_function_instance(parsed_request).await;
        match res {
            Ok(fid) => Ok(tonic::Response::new(FunctonInstanceConverters::serialize_function_id(&fid))),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }

    async fn stop_function_instance(&self, request: tonic::Request<crate::grpc_impl::api::FunctionId>) -> Result<tonic::Response<()>, tonic::Status> {
        let stop_function_id = match FunctonInstanceConverters::parse_function_id(&request.into_inner()) {
            Ok(parsed_update) => parsed_update,
            Err(err) => {
                log::error!("Parse Update Failed: {}", err);
                return Err(tonic::Status::invalid_argument("Bad Request"));
            }
        };
        let res = self.root_api.lock().await.stop_function_instance(stop_function_id).await;
        match res {
            Ok(_fid) => Ok(tonic::Response::new(())),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }

    async fn update_function_instance_links(
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
        let res = self.root_api.lock().await.update_function_instance_links(parsed_update).await;
        match res {
            Ok(_fid) => Ok(tonic::Response::new(())),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }
}
