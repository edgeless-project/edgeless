// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use crate::grpc_impl::api as grpc_stubs;
use crate::grpc_impl::common::CommonConverters;

#[derive(Clone)]
pub struct FunctionInstanceAPIClient<FunctionIdType> {
    client: Option<crate::grpc_impl::api::function_instance_client::FunctionInstanceClient<tonic::transport::Channel>>,
    server_addr: String,
    _phantom: std::marker::PhantomData<FunctionIdType>,
}

impl<FunctionIdType: crate::grpc_impl::common::SerializeableId + Clone + Send + Sync + 'static> FunctionInstanceAPIClient<FunctionIdType> {
    pub fn new(server_addr: String) -> Self {
        Self {
            client: None,
            server_addr,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Try connecting, if not already connected.
    ///
    /// If an error is returned, then the client is set to None (disconnected).
    /// Otherwise, the client is set to some value (connected).
    async fn try_connect(&mut self) -> anyhow::Result<()> {
        if self.client.is_none() {
            self.client = match crate::grpc_impl::api::function_instance_client::FunctionInstanceClient::connect(self.server_addr.clone()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    Some(client)
                }
                Err(err) => anyhow::bail!(err),
            }
        }
        Ok(())
    }

    /// Disconnect the client.
    fn disconnect(&mut self) {
        self.client = None;
    }
}

#[async_trait::async_trait]
impl<FunctionIdType: crate::grpc_impl::common::SerializeableId + Clone + Send + Sync + 'static>
    crate::function_instance::FunctionInstanceAPI<FunctionIdType> for FunctionInstanceAPIClient<FunctionIdType>
where
    // TODO: refactor
    grpc_stubs::InstanceIdVariant: crate::grpc_impl::common::ParseableId<FunctionIdType>,
{
    async fn start(
        &mut self,
        request: crate::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::common::StartComponentResponse<FunctionIdType>> {
        match self.try_connect().await {
            Ok(_) => {
                if let Some(client) = &mut self.client {
                    match client.start(tonic::Request::new(serialize_spawn_function_request(&request))).await {
                        Ok(res) => CommonConverters::parse_start_component_response::<FunctionIdType>(&res.into_inner()),
                        Err(err) => {
                            self.disconnect();
                            Err(anyhow::anyhow!(
                                "Error when starting a function at {}: {}",
                                self.server_addr,
                                err.to_string()
                            ))
                        }
                    }
                } else {
                    panic!("The impossible happened");
                }
            }
            Err(err) => {
                anyhow::bail!("Error when connecting to {}: {}", self.server_addr, err);
            }
        }
    }

    async fn stop(&mut self, id: FunctionIdType) -> anyhow::Result<()> {
        match self.try_connect().await {
            Ok(_) => {
                if let Some(client) = &mut self.client {
                    match client
                        .stop(tonic::Request::new(crate::grpc_impl::common::SerializeableId::serialize(&id)))
                        .await
                    {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            self.disconnect();
                            Err(anyhow::anyhow!(
                                "Error when stopping a function at {}: {}",
                                self.server_addr,
                                err.to_string()
                            ))
                        }
                    }
                } else {
                    panic!("The impossible happened");
                }
            }
            Err(err) => {
                anyhow::bail!("Error when connecting to {}: {}", self.server_addr, err);
            }
        }
    }

    async fn patch(&mut self, update: crate::common::PatchRequest) -> anyhow::Result<()> {
        match self.try_connect().await {
            Ok(_) => {
                if let Some(client) = &mut self.client {
                    match client
                        .patch(tonic::Request::new(CommonConverters::serialize_patch_request(&update)))
                        .await
                    {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            self.disconnect();
                            Err(anyhow::anyhow!(
                                "Error when patching a function at {}: {}",
                                self.server_addr,
                                err.to_string()
                            ))
                        }
                    }
                } else {
                    panic!("The impossible happened");
                }
            }
            Err(err) => {
                anyhow::bail!("Error when connecting to {}: {}", self.server_addr, err);
            }
        }
    }
}
pub struct FunctionInstanceAPIServer<FunctionIdType> {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::function_instance::FunctionInstanceAPI<FunctionIdType>>>,
}

#[async_trait::async_trait]
impl<FunctionIdType: crate::grpc_impl::common::SerializeableId + Clone + Send + 'static>
    crate::grpc_impl::api::function_instance_server::FunctionInstance for FunctionInstanceAPIServer<FunctionIdType>
where
    crate::grpc_impl::api::InstanceIdVariant: crate::grpc_impl::common::ParseableId<FunctionIdType>,
{
    async fn start(
        &self,
        request: tonic::Request<crate::grpc_impl::api::SpawnFunctionRequest>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::StartComponentResponse>, tonic::Status> {
        let inner_request = request.into_inner();
        let parsed_request = match parse_spawn_function_request(&inner_request) {
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

    async fn stop(&self, request: tonic::Request<grpc_stubs::InstanceIdVariant>) -> Result<tonic::Response<()>, tonic::Status> {
        let stop_function_id = match crate::grpc_impl::common::ParseableId::<FunctionIdType>::parse(&request.into_inner()) {
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

pub fn parse_function_class_specification(
    api_spec: &grpc_stubs::FunctionClassSpecification,
) -> anyhow::Result<crate::function_instance::FunctionClassSpecification> {
    Ok(crate::function_instance::FunctionClassSpecification {
        function_class_id: api_spec.function_class_id.clone(),
        function_class_type: api_spec.function_class_type.clone(),
        function_class_version: api_spec.function_class_version.clone(),
        function_class_code: api_spec.function_class_code().to_vec(),
        function_class_outputs: api_spec.function_class_outputs.clone(),
    })
}

pub fn parse_spawn_function_request(
    api_request: &crate::grpc_impl::api::SpawnFunctionRequest,
) -> anyhow::Result<crate::function_instance::SpawnFunctionRequest> {
    Ok(crate::function_instance::SpawnFunctionRequest {
        code: parse_function_class_specification(match api_request.code.as_ref() {
            Some(val) => val,
            None => {
                return Err(anyhow::anyhow!("Request does not contain actor class."));
            }
        })?,
        annotations: api_request.annotations.clone(),
        state_specification: parse_state_specification(match &api_request.state_specification {
            Some(val) => val,
            None => {
                return Err(anyhow::anyhow!("Request does not contain state_spec."));
            }
        })?,
        workflow_id: api_request.workflow_id.clone(),
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
        function_class_code: Some(spec.function_class_code.clone()),
        function_class_outputs: spec.function_class_outputs.clone(),
    }
}

pub fn serialize_spawn_function_request(req: &crate::function_instance::SpawnFunctionRequest) -> crate::grpc_impl::api::SpawnFunctionRequest {
    crate::grpc_impl::api::SpawnFunctionRequest {
        code: Some(serialize_function_class_specification(&req.code)),
        annotations: req.annotations.clone(),
        state_specification: Some(serialize_state_specification(&req.state_specification)),
        workflow_id: req.workflow_id.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::StartComponentResponse;
    use crate::function_instance::FunctionClassSpecification;
    use crate::function_instance::SpawnFunctionRequest;
    use crate::function_instance::StatePolicy;
    use crate::function_instance::StateSpecification;
    use edgeless_api_core::instance_id::InstanceId;

    #[test]
    fn serialize_deserialize_spawn_function_request() {
        let messages = vec![SpawnFunctionRequest {
            code: FunctionClassSpecification {
                function_class_id: "my-func-id".to_string(),
                function_class_type: "WASM".to_string(),
                function_class_version: "1.0.0".to_string(),
                function_class_code: "binary-code".as_bytes().to_vec(),
                function_class_outputs: vec!["out".to_string(), "err".to_string()],
            },
            annotations: std::collections::HashMap::from([("key1".to_string(), "value1".to_string())]),
            state_specification: StateSpecification {
                state_id: uuid::Uuid::new_v4(),
                state_policy: StatePolicy::NodeLocal,
            },
            workflow_id: "workflow_1".to_string(),
        }];
        for msg in messages {
            match parse_spawn_function_request(&serialize_spawn_function_request(&msg)) {
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
