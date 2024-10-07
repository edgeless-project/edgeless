// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
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
            function_class_code: api_spec.function_class_code().to_vec(),
            function_class_outputs: api_spec
                .function_class_outputs
                .iter()
                .map(|(key, raw_port)| (crate::function_instance::PortId(key.clone()), Self::parse_port(raw_port).unwrap()))
                .collect(),
            function_class_inputs: api_spec
                .function_class_inputs
                .iter()
                .map(|(key, raw_port)| (crate::function_instance::PortId(key.clone()), Self::parse_port(raw_port).unwrap()))
                .collect(),
            function_class_inner_structure: api_spec
                .function_class_inner_structure
                .iter()
                .map(|mapping| {
                    (
                        // crate::function_instance::PortId(source.clone()),
                        match &mapping.source.as_ref().unwrap().mapping_node_type.as_ref().unwrap() {
                            super::api::mapping_node_variant::MappingNodeType::Port(port_id) => {
                                crate::function_instance::MappingNode::Port(crate::function_instance::PortId(port_id.clone()))
                            }
                            super::api::mapping_node_variant::MappingNodeType::SideEffect(_) => crate::function_instance::MappingNode::SideEffect,
                        },
                        mapping
                            .dest
                            .iter()
                            .map(|id| match &id.mapping_node_type.as_ref().unwrap() {
                                super::api::mapping_node_variant::MappingNodeType::Port(port_id) => {
                                    crate::function_instance::MappingNode::Port(crate::function_instance::PortId(port_id.clone()))
                                }
                                super::api::mapping_node_variant::MappingNodeType::SideEffect(_) => crate::function_instance::MappingNode::SideEffect,
                            })
                            .collect(),
                    )
                })
                .collect(),
        })
    }

    pub fn parse_spawn_function_request(
        api_request: &crate::grpc_impl::api::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::function_instance::SpawnFunctionRequest> {
        Ok(crate::function_instance::SpawnFunctionRequest {
            instance_id: CommonConverters::parse_instance_id(api_request.instance_id.as_ref().unwrap())?,
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
            input_mapping: api_request
                .input_mapping
                .iter()
                .filter_map(|(id, spec)| {
                    if let Ok(input) = CommonConverters::parse_input(spec) {
                        Some((crate::function_instance::PortId(id.clone()), input))
                    } else {
                        None
                    }
                })
                .collect(),
            output_mapping: api_request
                .output_mapping
                .iter()
                .filter_map(|(id, spec)| {
                    if let Ok(input) = CommonConverters::parse_output(spec) {
                        Some((crate::function_instance::PortId(id.clone()), input))
                    } else {
                        None
                    }
                })
                .collect(),
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

    pub fn parse_port(api_port: &crate::grpc_impl::api::Port) -> anyhow::Result<crate::function_instance::Port> {
        Ok(crate::function_instance::Port {
            id: crate::function_instance::PortId(api_port.port_id.clone()),
            method: match api_port.method {
                0 => crate::function_instance::PortMethod::Cast,
                1 => crate::function_instance::PortMethod::Call,
                _ => {
                    return Err(anyhow::anyhow!("Unknown Port Method"));
                }
            },
            data_type: crate::function_instance::PortDataType(api_port.data_type.clone()),
            return_data_type: api_port
                .return_data_type
                .clone()
                .and_then(|dt| Some(crate::function_instance::PortDataType(dt))),
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
            function_class_outputs: spec
                .function_class_outputs
                .iter()
                .map(|(key, port)| (key.0.clone(), Self::serialize_port(port)))
                .collect(),
            function_class_inputs: spec
                .function_class_inputs
                .iter()
                .map(|(key, port)| (key.0.clone(), Self::serialize_port(port)))
                .collect(),
            function_class_inner_structure: spec
                .function_class_inner_structure
                .iter()
                .map(|(input, outputs)| super::api::Mapping {
                    source: Some(match input {
                        crate::function_instance::MappingNode::Port(port_id) => super::api::MappingNodeVariant {
                            mapping_node_type: Some(super::api::mapping_node_variant::MappingNodeType::Port(port_id.0.clone())),
                        },
                        crate::function_instance::MappingNode::SideEffect => super::api::MappingNodeVariant {
                            mapping_node_type: Some(super::api::mapping_node_variant::MappingNodeType::SideEffect(
                                super::api::SideEffectMapping {},
                            )),
                        },
                    }),
                    dest: outputs
                        .iter()
                        .map(|item| match item {
                            crate::function_instance::MappingNode::Port(port_id) => super::api::MappingNodeVariant {
                                mapping_node_type: Some(super::api::mapping_node_variant::MappingNodeType::Port(port_id.0.clone())),
                            },
                            crate::function_instance::MappingNode::SideEffect => super::api::MappingNodeVariant {
                                mapping_node_type: Some(super::api::mapping_node_variant::MappingNodeType::SideEffect(
                                    super::api::SideEffectMapping {},
                                )),
                            },
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    pub fn serialize_spawn_function_request(req: &crate::function_instance::SpawnFunctionRequest) -> crate::grpc_impl::api::SpawnFunctionRequest {
        crate::grpc_impl::api::SpawnFunctionRequest {
            instance_id: Some(CommonConverters::serialize_instance_id(&req.instance_id)),
            code: Some(Self::serialize_function_class_specification(&req.code)),
            annotations: req.annotations.clone(),
            state_specification: Some(Self::serialize_state_specification(&req.state_specification)),
            input_mapping: req
                .input_mapping
                .iter()
                .map(|(id, spec)| (id.0.clone(), CommonConverters::serialize_input(spec)))
                .collect(),
            output_mapping: req
                .output_mapping
                .iter()
                .map(|(id, spec)| (id.0.clone(), CommonConverters::serialize_output(spec)))
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

    pub fn serialize_port(crate_port: &crate::function_instance::Port) -> crate::grpc_impl::api::Port {
        crate::grpc_impl::api::Port {
            port_id: crate_port.id.0.clone(),
            method: match crate_port.method {
                crate::function_instance::PortMethod::Cast => 0,
                crate::function_instance::PortMethod::Call => 1,
            },
            data_type: crate_port.data_type.0.clone(),
            return_data_type: crate_port.return_data_type.as_ref().and_then(|dt| Some(dt.0.clone())),
        }
    }
}

#[derive(Clone)]
pub struct FunctionInstanceAPIClient<FunctionIdType> {
    client: crate::grpc_impl::api::function_instance_client::FunctionInstanceClient<tonic::transport::Channel>,
    _phantom: std::marker::PhantomData<FunctionIdType>,
}

impl<FunctionIdType: crate::grpc_impl::common::SerializeableId + Clone + Send + Sync + 'static> FunctionInstanceAPIClient<FunctionIdType> {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::function_instance_client::FunctionInstanceClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Ok(Self {
                        client,
                        _phantom: std::marker::PhantomData,
                    });
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
impl<FunctionIdType: super::common::SerializeableId + Clone + Send + Sync + 'static> crate::function_instance::FunctionInstanceAPI<FunctionIdType>
    for FunctionInstanceAPIClient<FunctionIdType>
where
    super::api::InstanceIdVariant: super::common::ParseableId<FunctionIdType>,
{
    async fn start(
        &mut self,
        request: crate::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<crate::common::StartComponentResponse<FunctionIdType>> {
        match self
            .client
            .start(tonic::Request::new(FunctonInstanceConverters::serialize_spawn_function_request(&request)))
            .await
        {
            Ok(res) => CommonConverters::parse_start_component_response::<FunctionIdType>(&res.into_inner()),
            Err(err) => Err(anyhow::anyhow!(
                "Communication error while starting a function instance: {}",
                err.to_string()
            )),
        }
    }

    async fn stop(&mut self, id: FunctionIdType) -> anyhow::Result<()> {
        match self
            .client
            .stop(tonic::Request::new(super::common::SerializeableId::serialize(&id)))
            .await
        {
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

    async fn stop(&self, request: tonic::Request<super::api::InstanceIdVariant>) -> Result<tonic::Response<()>, tonic::Status> {
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
        let out_prt_spec_1 = crate::function_instance::Port {
            id: crate::function_instance::PortId("out".to_string()),
            method: crate::function_instance::PortMethod::Cast,
            data_type: crate::function_instance::PortDataType("d1".to_string()),
            return_data_type: None,
        };

        let mut out_port_spec_err = out_prt_spec_1.clone();
        out_port_spec_err.id = crate::function_instance::PortId("err".to_string());

        let mut in_port_spec_1 = out_prt_spec_1.clone();
        in_port_spec_1.id = crate::function_instance::PortId("in".to_string());

        let messages = vec![SpawnFunctionRequest {
            instance_id: Some(InstanceId {
                node_id: uuid::Uuid::new_v4(),
                function_id: uuid::Uuid::new_v4(),
            }),
            code: FunctionClassSpecification {
                function_class_id: "my-func-id".to_string(),
                function_class_type: "WASM".to_string(),
                function_class_version: "1.0.0".to_string(),
                function_class_code: "binary-code".as_bytes().to_vec(),
                function_class_outputs: std::collections::HashMap::from([
                    (crate::function_instance::PortId("out".to_string()), out_prt_spec_1.clone()),
                    (crate::function_instance::PortId("err".to_string()), out_port_spec_err.clone()),
                ]),
                function_class_inputs: std::collections::HashMap::from([(
                    crate::function_instance::PortId("in1".to_string()),
                    in_port_spec_1.clone(),
                )]),
                function_class_inner_structure: std::collections::HashMap::from([(
                    crate::function_instance::MappingNode::Port(crate::function_instance::PortId("in".to_string())),
                    vec![
                        crate::function_instance::MappingNode::Port(crate::function_instance::PortId("out1".to_string())),
                        crate::function_instance::MappingNode::Port(crate::function_instance::PortId("err".to_string())),
                    ],
                )]),
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
