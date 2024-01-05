use super::common::CommonConverters;

pub struct FunctonInstanceConverters {}

impl FunctonInstanceConverters {
    pub fn parse_function_class_specification(
        api_spec: &crate::grpc_impl::api::FunctionClassSpecification,
    ) -> anyhow::Result<crate::function_instance::FunctionClassSpecification> {
        Ok(crate::function_instance::FunctionClassSpecification {
            function_class_id: api_spec.function_class_id.clone(),
            function_class_type: api_spec.function_class_type.clone(),
            function_class_version: api_spec.function_class_version.clone(),
            function_class_inlude_code: api_spec.function_class_inline_code().to_vec(),
            outputs: api_spec.outputs.clone(),
        })
    }

    pub fn parse_spawn_function_request(
        api_request: &crate::grpc_impl::api::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::function_instance::SpawnFunctionRequest> {
        Ok(crate::function_instance::SpawnFunctionRequest {
            instance_id: match api_request.instance_id.as_ref() {
                Some(id) => Some(CommonConverters::parse_instance_id(id)?),
                None => None,
            },
            code: Self::parse_function_class_specification(match api_request.code.as_ref() {
                Some(val) => val,
                None => {
                    return Err(anyhow::anyhow!("Request does not contain actor class."));
                }
            })?,
            annotations: api_request.annotations.clone(),
            state_specification: Self::parse_state_specification(match &api_request.state_specification {
                Some(val) => val,
                None => {
                    return Err(anyhow::anyhow!("Request does not contain state_spec."));
                }
            })?,
        })
    }

    pub fn parse_start_resource_request(
        api_workflow: &crate::grpc_impl::api::StartResourceRequest,
    ) -> anyhow::Result<crate::function_instance::StartResourceRequest> {
        Ok(crate::function_instance::StartResourceRequest {
            class_type: api_workflow.class_type.clone(),
            configurations: api_workflow.configurations.clone(),
        })
    }

    pub fn parse_state_specification(
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

    pub fn serialize_function_class_specification(
        spec: &crate::function_instance::FunctionClassSpecification,
    ) -> crate::grpc_impl::api::FunctionClassSpecification {
        crate::grpc_impl::api::FunctionClassSpecification {
            function_class_id: spec.function_class_id.clone(),
            function_class_type: spec.function_class_type.clone(),
            function_class_version: spec.function_class_version.clone(),
            function_class_inline_code: Some(spec.function_class_inlude_code.clone()),
            outputs: spec.outputs.clone(),
        }
    }

    pub fn serialize_spawn_function_request(req: &crate::function_instance::SpawnFunctionRequest) -> crate::grpc_impl::api::SpawnFunctionRequest {
        crate::grpc_impl::api::SpawnFunctionRequest {
            instance_id: req
                .instance_id
                .as_ref()
                .and_then(|instance_id| Some(CommonConverters::serialize_instance_id(instance_id))),
            code: Some(Self::serialize_function_class_specification(&req.code)),
            annotations: req.annotations.clone(),
            state_specification: Some(Self::serialize_state_specification(&req.state_specification)),
        }
    }

    pub fn serialize_start_resource_request(
        crate_resource: &crate::function_instance::StartResourceRequest,
    ) -> crate::grpc_impl::api::StartResourceRequest {
        crate::grpc_impl::api::StartResourceRequest {
            class_type: crate_resource.class_type.clone(),
            configurations: crate_resource.configurations.clone(),
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

//
// orc
//

#[derive(Clone)]
pub struct FunctionInstanceOrcAPIClient {
    client: crate::grpc_impl::api::function_instance_orc_client::FunctionInstanceOrcClient<tonic::transport::Channel>,
}

impl FunctionInstanceOrcAPIClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::function_instance_orc_client::FunctionInstanceOrcClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Ok(Self { client });
                }
                Err(err) => match retry_interval {
                    Some(val) => tokio::time::sleep(tokio::time::Duration::from_secs(val)).await,
                    None => {
                        return Err(anyhow::anyhow!("Error when connecting to {}: {}", server_addr, err));
                    }
                },
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::function_instance::FunctionInstanceOrcAPI for FunctionInstanceOrcAPIClient {
    async fn start_function(
        &mut self,
        request: crate::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::common::StartComponentResponse> {
        match self
            .client
            .start_function(tonic::Request::new(FunctonInstanceConverters::serialize_spawn_function_request(&request)))
            .await
        {
            Ok(res) => CommonConverters::parse_start_component_response(&res.into_inner()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while starting a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn stop_function(&mut self, id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        match self
            .client
            .stop_function(tonic::Request::new(CommonConverters::serialize_instance_id(&id)))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn start_resource(
        &mut self,
        instance_specification: crate::function_instance::StartResourceRequest,
    ) -> anyhow::Result<crate::common::StartComponentResponse> {
        let serialized_request =
            crate::grpc_impl::function_instance::FunctonInstanceConverters::serialize_start_resource_request(&instance_specification);
        match self.client.start_resource(tonic::Request::new(serialized_request)).await {
            Ok(ret) => crate::grpc_impl::common::CommonConverters::parse_start_component_response(&ret.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Resource configuration request failed: {}", err)),
        }
    }

    async fn stop_resource(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        let encoded_id = CommonConverters::serialize_instance_id(&resource_id);
        match self.client.stop_resource(encoded_id).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Resource configuration request failed: {}", err)),
        }
    }

    async fn patch(&mut self, update: crate::common::PatchRequest) -> anyhow::Result<()> {
        match self
            .client
            .patch(tonic::Request::new(CommonConverters::serialize_patch_request(&update)))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}

pub struct FunctionInstanceOrcAPIServer {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::function_instance::FunctionInstanceOrcAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::function_instance_orc_server::FunctionInstanceOrc for FunctionInstanceOrcAPIServer {
    async fn start_function(
        &self,
        request: tonic::Request<crate::grpc_impl::api::SpawnFunctionRequest>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::StartComponentResponse>, tonic::Status> {
        let inner_request = request.into_inner();
        let parsed_request = match FunctonInstanceConverters::parse_spawn_function_request(&inner_request) {
            Ok(val) => val,
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::StartComponentResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Invalid function instance creation request".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    instance_id: None,
                }))
            }
        };
        match self.root_api.lock().await.start_function(parsed_request).await {
            Ok(response) => Ok(tonic::Response::new(
                crate::grpc_impl::common::CommonConverters::serialize_start_component_response(&response),
            )),
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::StartComponentResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Function instance creation request rejected".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    instance_id: None,
                }))
            }
        }
    }

    async fn stop_function(&self, request: tonic::Request<crate::grpc_impl::api::InstanceId>) -> Result<tonic::Response<()>, tonic::Status> {
        let stop_function_id = match CommonConverters::parse_instance_id(&request.into_inner()) {
            Ok(parsed_update) => parsed_update,
            Err(err) => {
                log::error!("Error when stopping a function instance: {}", err);
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when stopping a function instance: {}",
                    err
                )));
            }
        };
        match self.root_api.lock().await.stop_function(stop_function_id).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Function instance stopping error: {}", err))),
        }
    }

    async fn start_resource(
        &self,
        request: tonic::Request<crate::grpc_impl::api::StartResourceRequest>,
    ) -> tonic::Result<tonic::Response<crate::grpc_impl::api::StartComponentResponse>> {
        let inner = request.into_inner();
        let parsed_spec = match crate::grpc_impl::function_instance::FunctonInstanceConverters::parse_start_resource_request(&inner) {
            Ok(val) => val,
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::StartComponentResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Invalid resource specification".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    instance_id: None,
                }))
            }
        };
        match self.root_api.lock().await.start_resource(parsed_spec).await {
            Ok(response) => Ok(tonic::Response::new(
                crate::grpc_impl::common::CommonConverters::serialize_start_component_response(&response),
            )),
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::StartComponentResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Resource creation rejected".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    instance_id: None,
                }))
            }
        }
    }

    async fn stop_resource(&self, request: tonic::Request<crate::grpc_impl::api::InstanceId>) -> tonic::Result<tonic::Response<()>> {
        let inner = request.into_inner();
        let parsed_id = match CommonConverters::parse_instance_id(&inner) {
            Ok(val) => val,
            Err(err) => {
                return Err(tonic::Status::invalid_argument(format!("Error when deleting a resource: {}", err)));
            }
        };
        match self.root_api.lock().await.stop_resource(parsed_id).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when deleting a resource: {}", err))),
        }
    }

    async fn patch(&self, update: tonic::Request<crate::grpc_impl::api::PatchRequest>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_update = match CommonConverters::parse_patch_request(&update.into_inner()) {
            Ok(parsed_update) => parsed_update,
            Err(err) => {
                log::error!("Parse UpdateFunctionLinks Failed: {}", err);
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when updating the links of a function instance: {}",
                    err
                )));
            }
        };
        match self.root_api.lock().await.patch(parsed_update).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!(
                "Error when updating the links of a function instance: {}",
                err
            ))),
        }
    }
}

//
// node
//

#[derive(Clone)]
pub struct FunctionInstanceNodeAPIClient {
    client: crate::grpc_impl::api::function_instance_node_client::FunctionInstanceNodeClient<tonic::transport::Channel>,
}

impl FunctionInstanceNodeAPIClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::function_instance_node_client::FunctionInstanceNodeClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Ok(Self { client });
                }
                Err(err) => match retry_interval {
                    Some(val) => tokio::time::sleep(tokio::time::Duration::from_secs(val)).await,
                    None => {
                        return Err(anyhow::anyhow!("Error when connecting to {}: {}", server_addr, err));
                    }
                },
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::function_instance::FunctionInstanceNodeAPI for FunctionInstanceNodeAPIClient {
    async fn start(&mut self, request: crate::function_instance::SpawnFunctionRequest) -> anyhow::Result<crate::common::StartComponentResponse> {
        match self
            .client
            .start(tonic::Request::new(FunctonInstanceConverters::serialize_spawn_function_request(&request)))
            .await
        {
            Ok(res) => CommonConverters::parse_start_component_response(&res.into_inner()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while starting a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn stop(&mut self, id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        match self.client.stop(tonic::Request::new(CommonConverters::serialize_instance_id(&id))).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while stopping a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn patch(&mut self, update: crate::common::PatchRequest) -> anyhow::Result<()> {
        match self
            .client
            .patch(tonic::Request::new(CommonConverters::serialize_patch_request(&update)))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while updating the links of a function instance: {}",
                err.to_string()
            )),
        }
    }
}
pub struct FunctionInstanceNodeAPIServer {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::function_instance::FunctionInstanceNodeAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::function_instance_node_server::FunctionInstanceNode for FunctionInstanceNodeAPIServer {
    async fn start(
        &self,
        request: tonic::Request<crate::grpc_impl::api::SpawnFunctionRequest>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::StartComponentResponse>, tonic::Status> {
        let inner_request = request.into_inner();
        let parsed_request = match FunctonInstanceConverters::parse_spawn_function_request(&inner_request) {
            Ok(val) => val,
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::StartComponentResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Invalid function instance creation request".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    instance_id: None,
                }))
            }
        };
        match self.root_api.lock().await.start(parsed_request).await {
            Ok(response) => Ok(tonic::Response::new(CommonConverters::serialize_start_component_response(&response))),
            Err(err) => {
                return Ok(tonic::Response::new(crate::grpc_impl::api::StartComponentResponse {
                    response_error: Some(crate::grpc_impl::api::ResponseError {
                        summary: "Function instance creation request rejected".to_string(),
                        detail: Some(err.to_string()),
                    }),
                    instance_id: None,
                }))
            }
        }
    }

    async fn stop(&self, request: tonic::Request<crate::grpc_impl::api::InstanceId>) -> Result<tonic::Response<()>, tonic::Status> {
        let stop_function_id = match CommonConverters::parse_instance_id(&request.into_inner()) {
            Ok(parsed_update) => parsed_update,
            Err(err) => {
                log::error!("Error when stopping a function instance: {}", err);
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when stopping a function instance: {}",
                    err
                )));
            }
        };
        match self.root_api.lock().await.stop(stop_function_id).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Function instance stopping error: {}", err))),
        }
    }

    async fn patch(&self, update: tonic::Request<crate::grpc_impl::api::PatchRequest>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_update = match CommonConverters::parse_patch_request(&update.into_inner()) {
            Ok(parsed_update) => parsed_update,
            Err(err) => {
                log::error!("Parse UpdateFunctionLinks Failed: {}", err);
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when updating the links of a function instance: {}",
                    err
                )));
            }
        };
        match self.root_api.lock().await.patch(parsed_update).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!(
                "Error when updating the links of a function instance: {}",
                err
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::StartComponentResponse;
    use crate::function_instance::FunctionClassSpecification;
    use crate::function_instance::SpawnFunctionRequest;
    use crate::function_instance::StartResourceRequest;
    use crate::function_instance::StatePolicy;
    use crate::function_instance::StateSpecification;
    use edgeless_api_core::instance_id::InstanceId;

    #[test]
    fn serialize_deserialize_spawn_function_request() {
        let messages = vec![SpawnFunctionRequest {
            instance_id: Some(InstanceId {
                node_id: uuid::Uuid::new_v4(),
                function_id: uuid::Uuid::new_v4(),
            }),
            code: FunctionClassSpecification {
                function_class_id: "my-func-id".to_string(),
                function_class_type: "WASM".to_string(),
                function_class_version: "1.0.0".to_string(),
                function_class_inlude_code: "binary-code".as_bytes().to_vec(),
                outputs: vec!["out".to_string(), "err".to_string()],
            },
            annotations: std::collections::HashMap::from([("key1".to_string(), "value1".to_string())]),
            state_specification: StateSpecification {
                state_id: uuid::Uuid::new_v4(),
                state_policy: StatePolicy::NodeLocal,
            },
        }];
        for msg in messages {
            match FunctonInstanceConverters::parse_spawn_function_request(&FunctonInstanceConverters::serialize_spawn_function_request(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_start_resource_request() {
        let messages = vec![
            StartResourceRequest {
                class_type: "provider-1".to_string(),
                configurations: std::collections::HashMap::new(),
            },
            StartResourceRequest {
                class_type: "provider-1".to_string(),
                configurations: std::collections::HashMap::from([
                    ("conf-1".to_string(), "value-1".to_string()),
                    ("conf-2".to_string(), "value-2".to_string()),
                ]),
            },
        ];
        for msg in messages {
            match FunctonInstanceConverters::parse_start_resource_request(&&FunctonInstanceConverters::serialize_start_resource_request(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_start_component_response() {
        let messages = vec![
            StartComponentResponse::ResponseError(crate::common::ResponseError {
                summary: "error summary".to_string(),
                detail: Some("error details".to_string()),
            }),
            StartComponentResponse::InstanceId(InstanceId {
                node_id: uuid::Uuid::new_v4(),
                function_id: uuid::Uuid::new_v4(),
            }),
        ];
        for msg in messages {
            match CommonConverters::parse_start_component_response(&CommonConverters::serialize_start_component_response(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }
}
