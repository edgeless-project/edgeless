// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use std::str::FromStr;

#[derive(Clone)]
pub struct NodeManagementClient {
    client: crate::grpc_impl::api::node_management_client::NodeManagementClient<tonic::transport::Channel>,
}

pub struct NodeManagementAPIService {
    pub node_management_api: tokio::sync::Mutex<Box<dyn crate::node_managment::NodeManagementAPI>>,
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
impl crate::node_managment::NodeManagementAPI for NodeManagementClient {
    async fn update_peers(&mut self, request: crate::node_managment::UpdatePeersRequest) -> anyhow::Result<()> {
        match self
            .client
            .update_peers(tonic::Request::new(serialize_update_peers_request(&request)))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Communication error while updating peers: {}", err.to_string())),
        }
    }

    async fn keep_alive(&mut self) -> anyhow::Result<()> {
        match self.client.keep_alive(tonic::Request::new(())).await {
            Ok(_) => Ok(()),
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
            Ok(_) => Ok(tonic::Response::new(crate::grpc_impl::api::HealthStatus {})),
            Err(err) => Err(tonic::Status::internal(format!("Error during keep alive: {}", err))),
        }
    }
}

pub fn parse_update_peers_request(
    api_instance: &crate::grpc_impl::api::UpdatePeersRequest,
) -> anyhow::Result<crate::node_managment::UpdatePeersRequest> {
    match api_instance.request_type {
        x if x == crate::grpc_impl::api::UpdatePeersRequestType::Add as i32 => {
            if let (Some(node_id), Some(invocation_url)) = (&api_instance.node_id, &api_instance.invocation_url) {
                let node_id = uuid::Uuid::from_str(node_id.as_str());
                match node_id {
                    Ok(node_id) => Ok(crate::node_managment::UpdatePeersRequest::Add(node_id, invocation_url.clone())),
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
                    Ok(node_id) => Ok(crate::node_managment::UpdatePeersRequest::Del(node_id)),
                    Err(_) => Err(anyhow::anyhow!("Ill-formed UpdatePeersRequest: invalid UUID as node_id")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Ill-formed UpdatePeersRequest message: node_id not specified with del peer"
                ))
            }
        }
        x if x == crate::grpc_impl::api::UpdatePeersRequestType::Clear as i32 => Ok(crate::node_managment::UpdatePeersRequest::Clear),
        x => Err(anyhow::anyhow!("Ill-formed UpdatePeersRequest message: unknown type {}", x)),
    }
}

fn serialize_update_peers_request(req: &crate::node_managment::UpdatePeersRequest) -> crate::grpc_impl::api::UpdatePeersRequest {
    match req {
        crate::node_managment::UpdatePeersRequest::Add(node_id, invocation_url) => crate::grpc_impl::api::UpdatePeersRequest {
            request_type: crate::grpc_impl::api::UpdatePeersRequestType::Add as i32,
            node_id: Some(node_id.to_string()),
            invocation_url: Some(invocation_url.clone()),
        },
        crate::node_managment::UpdatePeersRequest::Del(node_id) => crate::grpc_impl::api::UpdatePeersRequest {
            request_type: crate::grpc_impl::api::UpdatePeersRequestType::Del as i32,
            node_id: Some(node_id.to_string()),
            invocation_url: None,
        },
        crate::node_managment::UpdatePeersRequest::Clear => crate::grpc_impl::api::UpdatePeersRequest {
            request_type: crate::grpc_impl::api::UpdatePeersRequestType::Clear as i32,
            node_id: None,
            invocation_url: None,
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::node_managment::UpdatePeersRequest;

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
}
