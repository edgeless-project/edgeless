// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use std::str::FromStr;

#[derive(Clone)]
pub struct NodeManagementClient {
    client: Option<crate::grpc_impl::api::node_management_client::NodeManagementClient<tonic::transport::Channel>>,
    server_addr: String,
}

pub struct NodeManagementAPIService {
    pub node_management_api: tokio::sync::Mutex<Box<dyn crate::node_management::NodeManagementAPI>>,
}

impl NodeManagementClient {
    pub fn new(server_addr: String) -> Self {
        Self { client: None, server_addr }
    }

    /// Try connecting, if not already connected.
    ///
    /// If an error is returned, then the client is set to None (disconnected).
    /// Otherwise, the client is set to some value (connected).
    async fn try_connect(&mut self) -> anyhow::Result<()> {
        if self.client.is_none() {
            self.client = match crate::grpc_impl::api::node_management_client::NodeManagementClient::connect(self.server_addr.clone()).await {
                Ok(client) => Some(client.max_decoding_message_size(usize::MAX)),
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
impl crate::node_management::NodeManagementAPI for NodeManagementClient {
    async fn update_peers(&mut self, request: crate::node_management::UpdatePeersRequest) -> anyhow::Result<()> {
        match self.try_connect().await {
            Ok(_) => {
                if let Some(client) = &mut self.client {
                    if let Err(err) = client.update_peers(tonic::Request::new(serialize_update_peers_request(&request))).await {
                        self.disconnect();
                        anyhow::bail!("Error when updating peers at {}: {}", self.server_addr, err.to_string());
                    } else {
                        Ok(())
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
    async fn reset(&mut self) -> anyhow::Result<()> {
        match self.try_connect().await {
            Ok(_) => {
                if let Some(client) = &mut self.client {
                    if let Err(err) = client.reset(tonic::Request::new(())).await {
                        self.disconnect();
                        anyhow::bail!("Error when resetting at {}: {}", self.server_addr, err.to_string());
                    } else {
                        Ok(())
                    }
                } else {
                    panic!("The impossible happened");
                }
            }
            Err(err) => {
                anyhow::bail!("Error when resetting to {}: {}", self.server_addr, err);
            }
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
    async fn reset(&self, _request: tonic::Request<()>) -> Result<tonic::Response<()>, tonic::Status> {
        match self.node_management_api.lock().await.reset().await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(err) => Err(tonic::Status::internal(format!("Error when resetting: {}", err))),
        }
    }
}

fn parse_update_peers_request(
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

#[cfg(test)]
mod test {
    use super::*;
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
}
