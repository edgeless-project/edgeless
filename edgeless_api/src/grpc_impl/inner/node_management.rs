// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
use std::{str::FromStr, time::Duration};

const RECONNECT_TRIES: i32 = 5;
const RECONNECT_TIMEOUT: u64 = 500;
const TIMEOUT: u64 = 500;
const TCP_KEEPALIVE: u64 = 2000;

#[derive(Clone)]
pub struct NodeManagementClient {
    client: crate::grpc_impl::grpc_api_stubs::node_management_client::NodeManagementClient<tonic::transport::Channel>,
    server_addr: String,
}

pub struct NodeManagementAPIService {
    pub node_management_api: tokio::sync::Mutex<Box<dyn crate::node_management::NodeManagementAPI>>,
}

impl NodeManagementClient {
    pub async fn new(server_addr: String) -> Self {
        loop {
            match crate::grpc_impl::grpc_api_stubs::node_management_client::NodeManagementClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    // TODO: add client level retry policy
                    return Self {
                        client,
                        server_addr: server_addr.to_string(),
                    };
                }
                Err(e) => {
                    log::warn!("Waiting for NodeManagementAPI to connect: {e}");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn reconnect(&mut self) -> Result<(), anyhow::Error> {
        let mut retries = RECONNECT_TRIES;
        loop {
            if retries == 0 {
                log::error!("could not reconnect in reasonable time");
                anyhow::bail!("could not reconnect in reasonable time");
            }
            match crate::grpc_impl::grpc_api_stubs::node_management_client::NodeManagementClient::connect(self.server_addr.clone()).await {
                Ok(client) => {
                    self.client = client.max_decoding_message_size(usize::MAX);
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("Waiting for NodeManagementAPI to reconnect: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(RECONNECT_TIMEOUT)).await;
                }
            }
            retries -= 1;
        }
    }
}

#[async_trait::async_trait]
impl crate::node_management::NodeManagementAPI for NodeManagementClient {
    async fn update_peers(&mut self, request: crate::node_management::UpdatePeersRequest) -> anyhow::Result<()> {
        loop {
            let serialized_event = serialize_update_peers_request(&request);
            let res = tokio::time::timeout(
                Duration::from_millis(TIMEOUT),
                self.client.update_peers(tonic::Request::new(serialized_event)),
            )
            .await;
            // first Result layer is for the tokio::time::timeout
            if let Ok(_) = res {
                return anyhow::Ok(());
            } else if let Err(_) = res {
                let res = self.reconnect().await;
                if let Ok(_) = res {
                    log::info!("reconnected successfully, retrying the request");
                    continue;
                } else {
                    log::error!("reconnect did not work");
                    return Err(anyhow::anyhow!("Error in NodeManagementAPI {}", self.server_addr));
                }
            }
        }
    }
    async fn reset(&mut self) -> anyhow::Result<()> {
        loop {
            let res = tokio::time::timeout(Duration::from_millis(TIMEOUT), self.client.reset(tonic::Request::new(()))).await;
            // first Result layer is for the tokio::time::timeout
            if let Ok(_) = res {
                return anyhow::Ok(());
            } else if let Err(_) = res {
                let res = self.reconnect().await;
                if let Ok(_) = res {
                    log::info!("reconnected successfully, retrying the request");
                    continue;
                } else {
                    log::error!("reconnect did not work");
                    return Err(anyhow::anyhow!("Error in NodeManagementAPI {}", self.server_addr));
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::grpc_impl::grpc_api_stubs::node_management_server::NodeManagement for NodeManagementAPIService {
    async fn update_peers(
        &self,
        request: tonic::Request<crate::grpc_impl::grpc_api_stubs::UpdatePeersRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
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
        x if x == crate::grpc_impl::grpc_api_stubs::UpdatePeersRequestType::Add as i32 => {
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
        x if x == crate::grpc_impl::grpc_api_stubs::UpdatePeersRequestType::Del as i32 => {
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
        x if x == crate::grpc_impl::grpc_api_stubs::UpdatePeersRequestType::Clear as i32 => Ok(crate::node_management::UpdatePeersRequest::Clear),
        x => Err(anyhow::anyhow!("Ill-formed UpdatePeersRequest message: unknown type {}", x)),
    }
}

fn serialize_update_peers_request(req: &crate::node_management::UpdatePeersRequest) -> crate::grpc_impl::grpc_api_stubs::UpdatePeersRequest {
    match req {
        crate::node_management::UpdatePeersRequest::Add(node_id, invocation_url) => crate::grpc_impl::grpc_api_stubs::UpdatePeersRequest {
            request_type: crate::grpc_impl::grpc_api_stubs::UpdatePeersRequestType::Add as i32,
            node_id: Some(node_id.to_string()),
            invocation_url: Some(invocation_url.clone()),
        },
        crate::node_management::UpdatePeersRequest::Del(node_id) => crate::grpc_impl::grpc_api_stubs::UpdatePeersRequest {
            request_type: crate::grpc_impl::grpc_api_stubs::UpdatePeersRequestType::Del as i32,
            node_id: Some(node_id.to_string()),
            invocation_url: None,
        },
        crate::node_management::UpdatePeersRequest::Clear => crate::grpc_impl::grpc_api_stubs::UpdatePeersRequest {
            request_type: crate::grpc_impl::grpc_api_stubs::UpdatePeersRequestType::Clear as i32,
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
