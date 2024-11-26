// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct DomainRegistrationClient {
    client: crate::grpc_impl::api::domain_registration_client::DomainRegistrationClient<tonic::transport::Channel>,
}

pub struct DomainRegistrationAPIService {
    pub domain_registration_api: tokio::sync::Mutex<Box<dyn crate::domain_registration::DomainRegistrationAPI>>,
}

impl DomainRegistrationClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::domain_registration_client::DomainRegistrationClient::connect(server_addr.to_string()).await {
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
impl crate::domain_registration::DomainRegistrationAPI for DomainRegistrationClient {
    async fn update_domain(
        &mut self,
        request: crate::domain_registration::UpdateDomainRequest,
    ) -> anyhow::Result<crate::domain_registration::UpdateDomainResponse> {
        match self
            .client
            .update_domain(tonic::Request::new(serialize_update_domain_request(&request)))
            .await
        {
            Ok(res) => parse_update_domain_response(&res.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Communication error while updating a domain: {}", err.to_string())),
        }
    }
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::domain_registration_server::DomainRegistration for DomainRegistrationAPIService {
    async fn update_domain(
        &self,
        request: tonic::Request<crate::grpc_impl::api::UpdateDomainRequest>,
    ) -> Result<tonic::Response<crate::grpc_impl::api::UpdateDomainResponse>, tonic::Status> {
        let parsed_request = match parse_update_domain_request(&request.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                log::error!("Parse UpdateDomainRequest Failed: {}", err);
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an UpdateDomainRequest message: {}",
                    err
                )));
            }
        };
        match self.domain_registration_api.lock().await.update_domain(parsed_request).await {
            Ok(res) => Ok(tonic::Response::new(serialize_update_domain_response(&res))),
            Err(err) => Err(tonic::Status::internal(format!("Error when updating a node: {}", err))),
        }
    }
}

fn parse_domain_capabilities(api_instance: &crate::grpc_impl::api::DomainCapabilities) -> crate::domain_registration::DomainCapabilities {
    crate::domain_registration::DomainCapabilities {
        num_cpus: api_instance.num_cpus,
        num_cores: api_instance.num_cores,
        mem_size: api_instance.mem_size,
        labels: std::collections::HashSet::from_iter(api_instance.labels.iter().cloned()),
        num_tee: api_instance.num_tee,
        num_tpm: api_instance.num_tpm,
        runtimes: std::collections::HashSet::from_iter(api_instance.runtimes.iter().cloned()),
        disk_tot_space: api_instance.disk_tot_space,
        num_gpus: api_instance.num_gpus,
        mem_size_gpu: api_instance.mem_size_gpu,
    }
}

fn serialize_domain_capabilities(req: &crate::domain_registration::DomainCapabilities) -> crate::grpc_impl::api::DomainCapabilities {
    crate::grpc_impl::api::DomainCapabilities {
        num_cpus: req.num_cpus,
        num_cores: req.num_cores,
        mem_size: req.mem_size,
        labels: req.labels.iter().cloned().collect::<Vec<String>>(),
        num_tee: req.num_tee,
        num_tpm: req.num_tpm,
        runtimes: req.runtimes.iter().cloned().collect::<Vec<String>>(),
        disk_tot_space: req.disk_tot_space,
        num_gpus: req.num_gpus,
        mem_size_gpu: req.mem_size_gpu,
    }
}

fn parse_update_domain_request(
    api_instance: &crate::grpc_impl::api::UpdateDomainRequest,
) -> anyhow::Result<crate::domain_registration::UpdateDomainRequest> {
    let capabilities = match &api_instance.capabilities {
        Some(capabilities) => parse_domain_capabilities(capabilities),
        None => crate::domain_registration::DomainCapabilities::default(),
    };
    Ok(crate::domain_registration::UpdateDomainRequest {
        domain_id: api_instance.domain_id.clone(),
        orchestrator_url: api_instance.orchestrator_url.clone(),
        capabilities,
        refresh_deadline: std::time::UNIX_EPOCH + std::time::Duration::from_secs(api_instance.refresh_deadline),
    })
}

fn serialize_update_domain_response(req: &crate::domain_registration::UpdateDomainResponse) -> crate::grpc_impl::api::UpdateDomainResponse {
    match req {
        crate::domain_registration::UpdateDomainResponse::ResponseError(err) => crate::grpc_impl::api::UpdateDomainResponse {
            response_error: Some(crate::grpc_impl::api::ResponseError {
                summary: err.summary.clone(),
                detail: err.detail.clone(),
            }),
        },
        crate::domain_registration::UpdateDomainResponse::Accepted => crate::grpc_impl::api::UpdateDomainResponse { response_error: None },
    }
}

fn parse_update_domain_response(
    api_instance: &crate::grpc_impl::api::UpdateDomainResponse,
) -> anyhow::Result<crate::domain_registration::UpdateDomainResponse> {
    match api_instance.response_error.as_ref() {
        Some(err) => Ok(crate::domain_registration::UpdateDomainResponse::ResponseError(
            crate::common::ResponseError {
                summary: err.summary.clone(),
                detail: err.detail.clone(),
            },
        )),
        None => Ok(crate::domain_registration::UpdateDomainResponse::Accepted),
    }
}

fn serialize_update_domain_request(req: &crate::domain_registration::UpdateDomainRequest) -> crate::grpc_impl::api::UpdateDomainRequest {
    crate::grpc_impl::api::UpdateDomainRequest {
        domain_id: req.domain_id.clone(),
        orchestrator_url: req.orchestrator_url.clone(),
        capabilities: Some(serialize_domain_capabilities(&req.capabilities)),
        refresh_deadline: req.refresh_deadline.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::domain_registration::DomainCapabilities;
    use crate::domain_registration::UpdateDomainRequest;
    use crate::domain_registration::UpdateDomainResponse;

    #[test]
    fn serialize_deserialize_update_domain_request() {
        let messages = vec![UpdateDomainRequest {
            domain_id: "my-domain".to_string(),
            orchestrator_url: "http://127.0.0.1:10000".to_string(),
            capabilities: DomainCapabilities::default(),
            refresh_deadline: std::time::UNIX_EPOCH + std::time::Duration::from_secs(313714800),
        }];
        for msg in messages {
            match parse_update_domain_request(&serialize_update_domain_request(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_update_domain_response() {
        let messages = vec![
            UpdateDomainResponse::ResponseError(crate::common::ResponseError {
                summary: "error summary".to_string(),
                detail: Some("error details".to_string()),
            }),
            UpdateDomainResponse::Accepted,
        ];
        for msg in messages {
            match parse_update_domain_response(&serialize_update_domain_response(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }
}
