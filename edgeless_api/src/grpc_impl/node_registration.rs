// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use std::str::FromStr;

#[derive(Clone)]
pub struct NodeRegistrationClient {
    client: crate::grpc_impl::api::node_registration_client::NodeRegistrationClient<tonic::transport::Channel>,
}

pub struct NodeRegistrationAPIService {
    pub node_registration_api: tokio::sync::Mutex<Box<dyn crate::node_registration::NodeRegistrationAPI>>,
}

impl NodeRegistrationClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::node_registration_client::NodeRegistrationClient::connect(server_addr.to_string()).await {
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
impl crate::node_registration::NodeRegistrationAPI for NodeRegistrationClient {
    async fn update_node(
        &mut self,
        request: crate::node_registration::UpdateNodeRequest,
    ) -> anyhow::Result<crate::node_registration::UpdateNodeResponse> {
        match self
            .client
            .update_node(tonic::Request::new(serialize_update_node_request(&request)))
            .await
        {
            Ok(res) => parse_update_node_response(&res.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Communication error while updating a node: {}", err.to_string())),
        }
    }
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::node_registration_server::NodeRegistration for NodeRegistrationAPIService {
    async fn update_node(
        &self,
        request: tonic::Request<crate::grpc_impl::api::UpdateNodeRequest>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::UpdateNodeResponse>, tonic::Status> {
        let parsed_request = match parse_update_node_request(&request.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                log::error!("Parse UpdateNodeRequest Failed: {}", err);
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an UpdateNodeRequest message: {}",
                    err
                )));
            }
        };
        match self.node_registration_api.lock().await.update_node(parsed_request).await {
            Ok(res) => Ok(tonic::Response::new(serialize_update_node_response(&res))),
            Err(err) => Err(tonic::Status::internal(format!("Error when updating a node: {}", err))),
        }
    }
}

fn parse_node_capabilities(api_instance: &crate::grpc_impl::api::NodeCapabilities) -> crate::node_registration::NodeCapabilities {
    crate::node_registration::NodeCapabilities {
        num_cpus: api_instance.num_cpus,
        model_name_cpu: api_instance.model_name_cpu.clone(),
        clock_freq_cpu: api_instance.clock_freq_cpu,
        num_cores: api_instance.num_cores,
        mem_size: api_instance.mem_size,
    }
}

fn serialize_node_capabilities(req: &crate::node_registration::NodeCapabilities) -> crate::grpc_impl::api::NodeCapabilities {
    crate::grpc_impl::api::NodeCapabilities {
        num_cpus: req.num_cpus,
        model_name_cpu: req.model_name_cpu.clone(),
        clock_freq_cpu: req.clock_freq_cpu,
        num_cores: req.num_cores,
        mem_size: req.mem_size,
    }
}

fn parse_update_node_request(api_instance: &crate::grpc_impl::api::UpdateNodeRequest) -> anyhow::Result<crate::node_registration::UpdateNodeRequest> {
    let node_id = uuid::Uuid::from_str(api_instance.node_id.as_str());
    if let Err(err) = node_id {
        return Err(anyhow::anyhow!("Ill-formed node_id field in UpdateNodeRequest message: {}", err));
    }
    match api_instance.request_type {
        x if x == crate::grpc_impl::api::UpdateNodeRequestType::Register as i32 => {
            let mut resource_providers = vec![];
            for resource_provider in &api_instance.resource_providers {
                match parse_resource_provider_specification(&resource_provider) {
                    Ok(val) => resource_providers.push(val),
                    Err(err) => {
                        return Err(anyhow::anyhow!("Ill-formed resource provider in UpdateNodeRequest message: {}", err));
                    }
                }
            }
            if let (Some(agent_url), Some(invocation_url)) = (api_instance.agent_url.as_ref(), api_instance.invocation_url.as_ref()) {
                Ok(crate::node_registration::UpdateNodeRequest::Registration(
                    node_id.unwrap(),
                    agent_url.to_string(),
                    invocation_url.to_string(),
                    resource_providers,
                    match &api_instance.capabilities {
                        Some(val) => parse_node_capabilities(&val),
                        None => crate::node_registration::NodeCapabilities::default(),
                    },
                ))
            } else {
                return Err(anyhow::anyhow!(
                    "Ill-formed UpdateNodeRequest message: agent or invocation URL not present in registration"
                ));
            }
        }
        x if x == crate::grpc_impl::api::UpdateNodeRequestType::Deregister as i32 => {
            Ok(crate::node_registration::UpdateNodeRequest::Deregistration(node_id.unwrap()))
        }
        x => Err(anyhow::anyhow!("Ill-formed UpdateNodeRequest message: unknown type {}", x)),
    }
}

fn serialize_update_node_response(req: &crate::node_registration::UpdateNodeResponse) -> crate::grpc_impl::api::UpdateNodeResponse {
    match req {
        crate::node_registration::UpdateNodeResponse::ResponseError(err) => crate::grpc_impl::api::UpdateNodeResponse {
            response_error: Some(crate::grpc_impl::api::ResponseError {
                summary: err.summary.clone(),
                detail: err.detail.clone(),
            }),
        },
        crate::node_registration::UpdateNodeResponse::Accepted => crate::grpc_impl::api::UpdateNodeResponse { response_error: None },
    }
}

fn parse_update_node_response(
    api_instance: &crate::grpc_impl::api::UpdateNodeResponse,
) -> anyhow::Result<crate::node_registration::UpdateNodeResponse> {
    match api_instance.response_error.as_ref() {
        Some(err) => Ok(crate::node_registration::UpdateNodeResponse::ResponseError(
            crate::common::ResponseError {
                summary: err.summary.clone(),
                detail: err.detail.clone(),
            },
        )),
        None => Ok(crate::node_registration::UpdateNodeResponse::Accepted),
    }
}

fn serialize_update_node_request(req: &crate::node_registration::UpdateNodeRequest) -> crate::grpc_impl::api::UpdateNodeRequest {
    match req {
        crate::node_registration::UpdateNodeRequest::Registration(node_id, agent_url, invocation_url, resource_providers, capabilities) => {
            crate::grpc_impl::api::UpdateNodeRequest {
                request_type: crate::grpc_impl::api::UpdateNodeRequestType::Register as i32,
                node_id: node_id.to_string(),
                agent_url: Some(agent_url.to_string()),
                invocation_url: Some(invocation_url.to_string()),
                resource_providers: resource_providers.iter().map(|x| serialize_resource_provider_specification(x)).collect(),
                capabilities: Some(serialize_node_capabilities(capabilities)),
            }
        }
        crate::node_registration::UpdateNodeRequest::Deregistration(node_id) => crate::grpc_impl::api::UpdateNodeRequest {
            request_type: crate::grpc_impl::api::UpdateNodeRequestType::Deregister as i32,
            node_id: node_id.to_string(),
            agent_url: None,
            invocation_url: None,
            resource_providers: vec![],
            capabilities: None,
        },
    }
}

fn parse_resource_provider_specification(
    api_spec: &crate::grpc_impl::api::ResourceProviderSpecification,
) -> anyhow::Result<crate::node_registration::ResourceProviderSpecification> {
    if api_spec.provider_id.is_empty() {
        return Err(anyhow::anyhow!(
            "Ill-formed ResourceProviderSpecification message: provider_id cannot be empty"
        ));
    }
    if api_spec.class_type.is_empty() {
        return Err(anyhow::anyhow!(
            "Ill-formed ResourceProviderSpecification message: class_type cannot be empty"
        ));
    }
    Ok(crate::node_registration::ResourceProviderSpecification {
        provider_id: api_spec.provider_id.clone(),
        class_type: api_spec.class_type.clone(),
        outputs: api_spec.outputs.clone(),
    })
}

fn serialize_resource_provider_specification(
    crate_spec: &crate::node_registration::ResourceProviderSpecification,
) -> crate::grpc_impl::api::ResourceProviderSpecification {
    crate::grpc_impl::api::ResourceProviderSpecification {
        provider_id: crate_spec.provider_id.clone(),
        class_type: crate_spec.class_type.clone(),
        outputs: crate_spec.outputs.clone(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::node_registration::NodeCapabilities;
    use crate::node_registration::ResourceProviderSpecification;
    use crate::node_registration::UpdateNodeRequest;
    use crate::node_registration::UpdateNodeResponse;

    #[test]
    fn serialize_deserialize_update_node_request() {
        let messages = vec![
            UpdateNodeRequest::Registration(
                uuid::Uuid::new_v4(),
                "http://127.0.0.1:10000".to_string(),
                "http://127.0.0.1:10001".to_string(),
                vec![ResourceProviderSpecification {
                    provider_id: "provider-1".to_string(),
                    class_type: "class-type-1".to_string(),
                    outputs: vec!["out1".to_string(), "out2".to_string()],
                }],
                NodeCapabilities {
                    num_cpus: 4,
                    model_name_cpu: "ARMv8 Processor rev 0 (v8l)".to_string(),
                    clock_freq_cpu: 62.50,
                    num_cores: 20,
                    mem_size: 15827,
                },
            ),
            UpdateNodeRequest::Registration(
                uuid::Uuid::new_v4(),
                "http://127.0.0.1:10000".to_string(),
                "http://127.0.0.1:10001".to_string(),
                vec![],
                NodeCapabilities::default(),
            ),
            UpdateNodeRequest::Deregistration(uuid::Uuid::new_v4()),
        ];
        for msg in messages {
            match parse_update_node_request(&serialize_update_node_request(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_update_node_response() {
        let messages = vec![
            UpdateNodeResponse::ResponseError(crate::common::ResponseError {
                summary: "error summary".to_string(),
                detail: Some("error details".to_string()),
            }),
            UpdateNodeResponse::Accepted,
        ];
        for msg in messages {
            match parse_update_node_response(&serialize_update_node_response(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }
}
