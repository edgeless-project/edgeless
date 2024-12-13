// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use std::str::FromStr;
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
            workflow_functions: api_request
                .workflow_functions
                .iter()
                .map(WorkflowInstanceConverters::parse_workflow_function)
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
            function_id: uuid::Uuid::from_str(api_mapping.function_id.as_str())?,
            domain_id: api_mapping.domain_id.to_string(),
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
            domain_mapping: api_instance
                .domain_mapping
                .iter()
                .map(WorkflowInstanceConverters::parse_workflow_function_mapping)
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
                Some(val) => match crate::grpc_impl::common::CommonConverters::parse_response_error(val) {
                    Ok(val) => Ok(crate::workflow_instance::SpawnWorkflowResponse::ResponseError(val)),
                    Err(err) => Err(anyhow::anyhow!(err.to_string())),
                },
                None => Err(anyhow::anyhow!(
                    "Ill-formed SpawnWorkflowResponse message: both ResponseError and WorkflowInstance are empty"
                )),
            },
        }
    }

    pub fn parse_domain_capabilities_list(
        api_instance: &crate::grpc_impl::api::DomainCapabilitiesList,
    ) -> anyhow::Result<std::collections::HashMap<String, crate::domain_registration::DomainCapabilities>> {
        let mut ret = std::collections::HashMap::new();
        for entry in &api_instance.domain_capabilities {
            if let Some(domain_capabilities) = &entry.domain_capabilities {
                ret.insert(
                    entry.domain_id.clone(),
                    crate::grpc_impl::domain_registration::parse_domain_capabilities(domain_capabilities),
                );
            }
        }
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
            workflow_functions: crate_request.workflow_functions.iter().map(Self::serialize_workflow_function).collect(),
            workflow_resources: crate_request.workflow_resources.iter().map(Self::serialize_workflow_resource).collect(),
            annotations: crate_request.annotations.clone(),
        }
    }

    pub fn serialize_workflow_spawn_response(
        crate_request: &crate::workflow_instance::SpawnWorkflowResponse,
    ) -> crate::grpc_impl::api::SpawnWorkflowResponse {
        match crate_request {
            crate::workflow_instance::SpawnWorkflowResponse::ResponseError(err) => crate::grpc_impl::api::SpawnWorkflowResponse {
                response_error: Some(crate::grpc_impl::common::CommonConverters::serialize_response_error(err)),
                workflow_status: None,
            },
            crate::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(instance) => crate::grpc_impl::api::SpawnWorkflowResponse {
                response_error: None,
                workflow_status: Some(Self::serialize_workflow_instance(instance)),
            },
        }
    }

    pub fn serialize_workflow_instance(crate_instance: &crate::workflow_instance::WorkflowInstance) -> crate::grpc_impl::api::WorkflowInstanceStatus {
        crate::grpc_impl::api::WorkflowInstanceStatus {
            workflow_id: Some(Self::serialize_workflow_id(&crate_instance.workflow_id)),
            domain_mapping: crate_instance
                .domain_mapping
                .iter()
                .map(Self::serialize_workflow_function_mapping)
                .collect(),
        }
    }

    pub fn serialize_domain_capabilities_list(
        domains: &std::collections::HashMap<String, crate::domain_registration::DomainCapabilities>,
    ) -> crate::grpc_impl::api::DomainCapabilitiesList {
        crate::grpc_impl::api::DomainCapabilitiesList {
            domain_capabilities: domains
                .iter()
                .map(|(domain_id, caps)| crate::grpc_impl::api::DomainCapabilitiesEntry {
                    domain_id: domain_id.clone(),
                    domain_capabilities: Some(crate::grpc_impl::domain_registration::serialize_domain_capabilities(caps)),
                })
                .collect(),
        }
    }

    pub fn serialize_workflow_function_mapping(
        crate_mapping: &crate::workflow_instance::WorkflowFunctionMapping,
    ) -> crate::grpc_impl::api::WorkflowFunctionMapping {
        crate::grpc_impl::api::WorkflowFunctionMapping {
            name: crate_mapping.name.to_string(),
            function_id: crate_mapping.function_id.to_string(),
            domain_id: crate_mapping.domain_id.to_string(),
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
    async fn list(&mut self) -> anyhow::Result<Vec<crate::workflow_instance::WorkflowId>> {
        let ret = self.client.list(tonic::Request::new(())).await;
        match ret {
            Ok(ret) => {
                return Ok(ret
                    .into_inner()
                    .identifiers
                    .iter()
                    .map(|val| crate::workflow_instance::WorkflowId {
                        workflow_id: uuid::Uuid::parse_str(val).unwrap_or_default(),
                    })
                    .collect());
            }
            Err(err) => Err(anyhow::anyhow!("Communication error while listing workflows: {}", err.to_string())),
        }
    }
    async fn inspect(&mut self, id: crate::workflow_instance::WorkflowId) -> anyhow::Result<crate::workflow_instance::WorkflowInfo> {
        let ret = self
            .client
            .inspect(tonic::Request::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_id(&id),
            ))
            .await;
        match ret {
            Ok(ret) => {
                let ret = ret.into_inner();
                let request = match &ret.request {
                    Some(request) => crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_spawn_request(request)?,
                    None => anyhow::bail!("Workflow request not present"),
                };
                let status = match &ret.status {
                    Some(status) => crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_instance(status)?,
                    None => anyhow::bail!("Workflow status not present"),
                };

                return Ok(crate::workflow_instance::WorkflowInfo { request, status });
            }
            Err(err) => Err(anyhow::anyhow!("Communication error while listing workflows: {}", err.to_string())),
        }
    }
    async fn domains(
        &mut self,
        domain_id: String,
    ) -> anyhow::Result<std::collections::HashMap<String, crate::domain_registration::DomainCapabilities>> {
        let ret = self
            .client
            .domains(tonic::Request::new(crate::grpc_impl::api::DomainId { domain_id }))
            .await;
        match ret {
            Ok(ret) => return crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_domain_capabilities_list(&ret.into_inner()),
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
            Err(err) => return Err(tonic::Status::internal(format!("Internal error when stopping a workflow: {}", err))),
        };
        let ret = self.root_api.lock().await.stop(req).await;
        match ret {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Internal error when stopping a workflow: {}", err))),
        }
    }

    async fn list(&self, _request: tonic::Request<()>) -> Result<tonic::Response<crate::grpc_impl::api::WorkflowIdList>, tonic::Status> {
        let ret = self.root_api.lock().await.list().await;
        match ret {
            Ok(identifiers) => Ok(tonic::Response::new(crate::grpc_impl::api::WorkflowIdList {
                identifiers: identifiers.iter().map(|x| x.to_string()).collect(),
            })),
            Err(err) => Err(tonic::Status::internal(format!("Internal error when listing workflows: {}", err))),
        }
    }

    async fn inspect(
        &self,
        request_id: tonic::Request<crate::grpc_impl::api::WorkflowId>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::WorkflowInstanceInfo>, tonic::Status> {
        let req = match crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::parse_workflow_id(&request_id.into_inner()) {
            Ok(val) => val,
            Err(err) => return Err(tonic::Status::internal(format!("Internal error when inspecting a workflow: {}", err))),
        };
        let ret = self.root_api.lock().await.inspect(req).await;
        match ret {
            Ok(info) => Ok(tonic::Response::new(crate::grpc_impl::api::WorkflowInstanceInfo {
                request: Some(crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_spawn_request(&info.request)),
                status: Some(crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_workflow_instance(&info.status)),
            })),
            Err(err) => Err(tonic::Status::internal(format!("Internal error when inspecting a workflow: {}", err))),
        }
    }

    async fn domains(
        &self,
        domain_id: tonic::Request<crate::grpc_impl::api::DomainId>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::DomainCapabilitiesList>, tonic::Status> {
        match self.root_api.lock().await.domains(domain_id.into_inner().domain_id).await {
            Ok(instances) => Ok(tonic::Response::new(
                crate::grpc_impl::workflow_instance::WorkflowInstanceConverters::serialize_domain_capabilities_list(&instances),
            )),
            Err(err) => Err(tonic::Status::internal(format!(
                "Internal error when listing domain capabilities: {}",
                err
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::function_instance::FunctionClassSpecification;
    use crate::workflow_instance::SpawnWorkflowRequest;
    use crate::workflow_instance::SpawnWorkflowResponse;
    use crate::workflow_instance::WorkflowFunction;
    use crate::workflow_instance::WorkflowFunctionMapping;
    use crate::workflow_instance::WorkflowId;
    use crate::workflow_instance::WorkflowInstance;
    use crate::workflow_instance::WorkflowResource;

    #[test]
    fn serialize_deserialize_workflow_id() {
        let messages = vec![WorkflowId {
            workflow_id: uuid::Uuid::new_v4(),
        }];

        for msg in messages {
            match WorkflowInstanceConverters::parse_workflow_id(&WorkflowInstanceConverters::serialize_workflow_id(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_workflow_function() {
        let messages = vec![WorkflowFunction {
            name: "f1".to_string(),
            function_class_specification: FunctionClassSpecification {
                function_class_id: "my_fun_class".to_string(),
                function_class_type: "my_fun_class_type".to_string(),
                function_class_version: "0.0.1".to_string(),
                function_class_code: "byte-code".to_string().as_bytes().to_vec(),
                function_class_outputs: vec!["out1".to_string(), "out2".to_string()],
            },
            output_mapping: HashMap::from([("out1".to_string(), "out3".to_string()), ("out2".to_string(), "out4".to_string())]),
            annotations: HashMap::from([("ann1".to_string(), "val1".to_string()), ("ann2".to_string(), "val2".to_string())]),
        }];

        for msg in messages {
            match WorkflowInstanceConverters::parse_workflow_function(&WorkflowInstanceConverters::serialize_workflow_function(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_workflow_resource() {
        let messages = vec![WorkflowResource {
            name: "res1".to_string(),
            class_type: "my_res_class_type".to_string(),
            output_mapping: HashMap::from([("out1".to_string(), "out3".to_string()), ("out2".to_string(), "out4".to_string())]),
            configurations: HashMap::from([("conf1".to_string(), "val1".to_string()), ("conf2".to_string(), "val2".to_string())]),
        }];

        for msg in messages {
            match WorkflowInstanceConverters::parse_workflow_resource(&WorkflowInstanceConverters::serialize_workflow_resource(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_workflow_workflow_spawn_request() {
        let messages = vec![SpawnWorkflowRequest {
            workflow_functions: vec![WorkflowFunction {
                name: "f1".to_string(),
                function_class_specification: FunctionClassSpecification {
                    function_class_id: "my_fun_class".to_string(),
                    function_class_type: "my_fun_class_type".to_string(),
                    function_class_version: "0.0.1".to_string(),
                    function_class_code: "byte-code".to_string().as_bytes().to_vec(),
                    function_class_outputs: vec!["out1".to_string(), "out2".to_string()],
                },
                output_mapping: HashMap::from([("out1".to_string(), "out3".to_string()), ("out2".to_string(), "out4".to_string())]),
                annotations: HashMap::from([("ann1".to_string(), "val1".to_string()), ("ann2".to_string(), "val2".to_string())]),
            }],
            annotations: HashMap::from([("ann1".to_string(), "val1".to_string()), ("ann2".to_string(), "val2".to_string())]),
            workflow_resources: vec![WorkflowResource {
                name: "res1".to_string(),
                class_type: "my_res_class_type".to_string(),
                output_mapping: HashMap::from([("out1".to_string(), "out3".to_string()), ("out2".to_string(), "out4".to_string())]),
                configurations: HashMap::from([("conf1".to_string(), "val1".to_string()), ("conf2".to_string(), "val2".to_string())]),
            }],
        }];

        for msg in messages {
            match WorkflowInstanceConverters::parse_workflow_spawn_request(&WorkflowInstanceConverters::serialize_workflow_spawn_request(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_workflow_function_mapping() {
        let messages = vec![WorkflowFunctionMapping {
            name: "fun1".to_string(),
            function_id: uuid::Uuid::new_v4(),
            domain_id: "domain1".to_string(),
        }];

        for msg in messages {
            match WorkflowInstanceConverters::parse_workflow_function_mapping(&WorkflowInstanceConverters::serialize_workflow_function_mapping(&msg))
            {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_workflow_instance() {
        let messages = vec![WorkflowInstance {
            workflow_id: WorkflowId {
                workflow_id: uuid::Uuid::new_v4(),
            },
            domain_mapping: vec![
                WorkflowFunctionMapping {
                    name: "fun1".to_string(),
                    function_id: uuid::Uuid::new_v4(),
                    domain_id: "domain1".to_string(),
                },
                WorkflowFunctionMapping {
                    name: "fun2".to_string(),
                    function_id: uuid::Uuid::new_v4(),
                    domain_id: "domain2".to_string(),
                },
            ],
        }];

        for msg in messages {
            match WorkflowInstanceConverters::parse_workflow_instance(&WorkflowInstanceConverters::serialize_workflow_instance(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_spawn_response() {
        let messages = vec![SpawnWorkflowResponse::WorkflowInstance(WorkflowInstance {
            workflow_id: WorkflowId {
                workflow_id: uuid::Uuid::new_v4(),
            },
            domain_mapping: vec![
                WorkflowFunctionMapping {
                    name: "fun1".to_string(),
                    function_id: uuid::Uuid::new_v4(),
                    domain_id: "domain1".to_string(),
                },
                WorkflowFunctionMapping {
                    name: "fun2".to_string(),
                    function_id: uuid::Uuid::new_v4(),
                    domain_id: "domain2".to_string(),
                },
            ],
        })];

        for msg in messages {
            match WorkflowInstanceConverters::parse_workflow_spawn_response(&WorkflowInstanceConverters::serialize_workflow_spawn_response(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }
}
