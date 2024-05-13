// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use std::str::FromStr;

#[derive(Clone)]
pub struct NodeManagementClient {
    client: crate::grpc_impl::api::node_management_client::NodeManagementClient<tonic::transport::Channel>,
}

pub struct NodeManagementAPIService {
    pub node_management_api: tokio::sync::Mutex<Box<dyn crate::node_management::NodeManagementAPI>>,
}

impl NodeManagementClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::node_management_client::NodeManagementClient::connect(server_addr.to_string()).await {
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
impl crate::node_management::NodeManagementAPI for NodeManagementClient {
    async fn update_peers(&mut self, request: crate::node_management::UpdatePeersRequest) -> anyhow::Result<()> {
        match self
            .client
            .update_peers(tonic::Request::new(serialize_update_peers_request(&request)))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Communication error while updating peers: {}", err.to_string())),
        }
    }

    async fn keep_alive(&mut self) -> anyhow::Result<crate::node_management::HealthStatus> {
        match self.client.keep_alive(tonic::Request::new(())).await {
            Ok(res) => parse_health_status(&res.into_inner()),
            Err(err) => Err(anyhow::anyhow!("Communication error during keep alive: {}", err.to_string())),
        }
    }
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::node_management_server::NodeManagement for NodeManagementAPIService {
    async fn update_peers(&self, request: tonic::Request<crate::grpc_impl::api::UpdatePeersRequest>) -> Result<tonic::Response<()>, tonic::Status> {
        let parsed_request = match parse_update_peers_request(&request.into_inner()) {
            Ok(parsed_request) => parsed_request,
            Err(err) => {
                log::error!("Parse UpdatePeersRequest Failed: {}", err);
                return Err(tonic::Status::invalid_argument(format!(
                    "Error when parsing an UpdatePeersRequest message: {}",
                    err
                )));
            }
        };
        match self.node_management_api.lock().await.update_peers(parsed_request).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when updating peers: {}", err))),
        }
    }

    async fn keep_alive(&self, _request: tonic::Request<()>) -> Result<tonic::Response<crate::grpc_impl::api::HealthStatus>, tonic::Status> {
        match self.node_management_api.lock().await.keep_alive().await {
            Ok(health_status) => Ok(tonic::Response::new(serialize_health_status(&health_status))),
            Err(err) => Err(tonic::Status::internal(format!("Error during keep alive: {}", err))),
        }
    }
}

pub fn parse_update_peers_request(
    api_instance: &crate::grpc_impl::api::UpdatePeersRequest,
) -> anyhow::Result<crate::node_management::UpdatePeersRequest> {
    match api_instance.request_type {
        x if x == crate::grpc_impl::api::UpdatePeersRequestType::Add as i32 => {
            if let (Some(node_id), Some(invocation_url)) = (&api_instance.node_id, &api_instance.invocation_url) {
                let node_id = uuid::Uuid::from_str(node_id.as_str());
                match node_id {
                    Ok(node_id) => Ok(crate::node_management::UpdatePeersRequest::Add(node_id, invocation_url.clone())),
                    Err(_) => Err(anyhow::anyhow!("Ill-formed UpdatePeersRequest: invalid UUID as node_id")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Ill-formed UpdatePeersRequest message: node_id or invocation_url not specified with add peer"
                ))
            }
        }
        x if x == crate::grpc_impl::api::UpdatePeersRequestType::Del as i32 => {
            if let Some(node_id) = &api_instance.node_id {
                let node_id = uuid::Uuid::from_str(node_id.as_str());
                match node_id {
                    Ok(node_id) => Ok(crate::node_management::UpdatePeersRequest::Del(node_id)),
                    Err(_) => Err(anyhow::anyhow!("Ill-formed UpdatePeersRequest: invalid UUID as node_id")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Ill-formed UpdatePeersRequest message: node_id not specified with del peer"
                ))
            }
        }
        x if x == crate::grpc_impl::api::UpdatePeersRequestType::Clear as i32 => Ok(crate::node_management::UpdatePeersRequest::Clear),
        x => Err(anyhow::anyhow!("Ill-formed UpdatePeersRequest message: unknown type {}", x)),
    }
}

pub fn parse_health_status(api_instance: &crate::grpc_impl::api::HealthStatus) -> anyhow::Result<crate::node_management::HealthStatus> {
    Ok(crate::node_management::HealthStatus {
        cpu_usage: api_instance.cpu_usage,
        cpu_load: api_instance.cpu_load,
        mem_free: api_instance.mem_free,
        mem_used: api_instance.mem_used,
        mem_total: api_instance.mem_total,
        mem_available: api_instance.mem_available,
        proc_cpu_usage: api_instance.proc_cpu_usage,
        proc_memory: api_instance.proc_memory,
        proc_vmemory: api_instance.proc_vmemory,
    })
}

fn serialize_update_peers_request(req: &crate::node_management::UpdatePeersRequest) -> crate::grpc_impl::api::UpdatePeersRequest {
    match req {
        crate::node_management::UpdatePeersRequest::Add(node_id, invocation_url) => crate::grpc_impl::api::UpdatePeersRequest {
            request_type: crate::grpc_impl::api::UpdatePeersRequestType::Add as i32,
            node_id: Some(node_id.to_string()),
            invocation_url: Some(invocation_url.clone()),
        },
        crate::node_management::UpdatePeersRequest::Del(node_id) => crate::grpc_impl::api::UpdatePeersRequest {
            request_type: crate::grpc_impl::api::UpdatePeersRequestType::Del as i32,
            node_id: Some(node_id.to_string()),
            invocation_url: None,
        },
        crate::node_management::UpdatePeersRequest::Clear => crate::grpc_impl::api::UpdatePeersRequest {
            request_type: crate::grpc_impl::api::UpdatePeersRequestType::Clear as i32,
            node_id: None,
            invocation_url: None,
        },
    }
}

fn serialize_health_status(req: &crate::node_management::HealthStatus) -> crate::grpc_impl::api::HealthStatus {
    crate::grpc_impl::api::HealthStatus {
        cpu_usage: req.cpu_usage,
        cpu_load: req.cpu_load,
        mem_free: req.mem_free,
        mem_used: req.mem_used,
        mem_total: req.mem_total,
        mem_available: req.mem_available,
        proc_cpu_usage: req.proc_cpu_usage,
        proc_memory: req.proc_memory,
        proc_vmemory: req.proc_vmemory,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::node_management::HealthStatus;
    use crate::node_management::UpdatePeersRequest;

    #[test]
    fn serialize_deserialize_update_peers_request() {
        let messages = vec![
            UpdatePeersRequest::Add(uuid::Uuid::new_v4(), "http://127.0.0.10001".to_string()),
            UpdatePeersRequest::Del(uuid::Uuid::new_v4()),
            UpdatePeersRequest::Clear,
        ];
        for msg in messages {
            match parse_update_peers_request(&serialize_update_peers_request(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn serialize_deserialize_health_status() {
        let messages = vec![
            HealthStatus::empty(),
            HealthStatus::invalid(),
            HealthStatus {
                cpu_usage: 1,
                cpu_load: 2,
                mem_free: 3,
                mem_used: 4,
                mem_total: 5,
                mem_available: 6,
                proc_cpu_usage: 7,
                proc_memory: 8,
                proc_vmemory: 9,
            },
        ];
        for msg in messages {
            match parse_health_status(&serialize_health_status(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }
}
