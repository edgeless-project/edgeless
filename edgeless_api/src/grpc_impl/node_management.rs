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

    async fn keep_alive(&mut self) -> anyhow::Result<crate::node_management::KeepAliveResponse> {
        match self.client.keep_alive(tonic::Request::new(())).await {
            Ok(res) => parse_keep_alive_response(&res.into_inner()),
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

    async fn keep_alive(&self, _request: tonic::Request<()>) -> Result<tonic::Response<crate::grpc_impl::api::KeepAliveResponse>, tonic::Status> {
        match self.node_management_api.lock().await.keep_alive().await {
            Ok(keep_alive_response) => Ok(tonic::Response::new(serialize_keep_alive_response(&keep_alive_response))),
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

pub fn parse_node_health_status(api_instance: &crate::grpc_impl::api::NodeHealthStatus) -> anyhow::Result<crate::node_management::NodeHealthStatus> {
    Ok(crate::node_management::NodeHealthStatus {
        mem_free: api_instance.mem_free,
        mem_used: api_instance.mem_used,
        mem_available: api_instance.mem_available,
        proc_cpu_usage: api_instance.proc_cpu_usage,
        proc_memory: api_instance.proc_memory,
        proc_vmemory: api_instance.proc_vmemory,
        load_avg_1: api_instance.load_avg_1,
        load_avg_5: api_instance.load_avg_5,
        load_avg_15: api_instance.load_avg_15,
        tot_rx_bytes: api_instance.tot_rx_bytes,
        tot_rx_pkts: api_instance.tot_rx_pkts,
        tot_rx_errs: api_instance.tot_rx_errs,
        tot_tx_pkts: api_instance.tot_tx_pkts,
        tot_tx_bytes: api_instance.tot_tx_bytes,
        tot_tx_errs: api_instance.tot_tx_errs,
        disk_free_space: api_instance.disk_free_space,
        disk_tot_reads: api_instance.disk_tot_reads,
        disk_tot_writes: api_instance.disk_tot_writes,
        gpu_load_perc: api_instance.gpu_load_perc,
        gpu_temp_cels: api_instance.gpu_temp_cels,
    })
}

pub fn parse_node_performance_samples(
    api_instance: &crate::grpc_impl::api::NodePerformanceSamples,
) -> anyhow::Result<crate::node_management::NodePerformanceSamples> {
    Ok(crate::node_management::NodePerformanceSamples {
        function_execution_times: api_instance
            .function_execution_times
            .iter()
            .filter_map(|x| match uuid::Uuid::from_str(&x.id) {
                Ok(val) => Some((val, x.samples.clone())),
                _ => None,
            })
            .collect(),
    })
}

pub fn parse_keep_alive_response(
    api_instance: &crate::grpc_impl::api::KeepAliveResponse,
) -> anyhow::Result<crate::node_management::KeepAliveResponse> {
    let health_status = match &api_instance.health_status {
        Some(val) => match parse_node_health_status(val) {
            Ok(res) => res,
            Err(_) => crate::node_management::NodeHealthStatus::invalid(),
        },
        None => crate::node_management::NodeHealthStatus::invalid(),
    };
    let performance_samples = match &api_instance.performance_samples {
        Some(val) => match parse_node_performance_samples(val) {
            Ok(res) => res,
            Err(_) => crate::node_management::NodePerformanceSamples::empty(),
        },
        None => crate::node_management::NodePerformanceSamples::empty(),
    };
    Ok(crate::node_management::KeepAliveResponse {
        health_status,
        performance_samples,
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

fn serialize_node_health_status(req: &crate::node_management::NodeHealthStatus) -> crate::grpc_impl::api::NodeHealthStatus {
    crate::grpc_impl::api::NodeHealthStatus {
        mem_free: req.mem_free,
        mem_used: req.mem_used,
        mem_available: req.mem_available,
        proc_cpu_usage: req.proc_cpu_usage,
        proc_memory: req.proc_memory,
        proc_vmemory: req.proc_vmemory,
        load_avg_1: req.load_avg_1,
        load_avg_5: req.load_avg_5,
        load_avg_15: req.load_avg_15,
        tot_rx_bytes: req.tot_rx_bytes,
        tot_rx_pkts: req.tot_rx_pkts,
        tot_rx_errs: req.tot_rx_errs,
        tot_tx_pkts: req.tot_tx_pkts,
        tot_tx_bytes: req.tot_tx_bytes,
        tot_tx_errs: req.tot_tx_errs,
        disk_free_space: req.disk_free_space,
        disk_tot_reads: req.disk_tot_reads,
        disk_tot_writes: req.disk_tot_writes,
        gpu_load_perc: req.gpu_load_perc,
        gpu_temp_cels: req.gpu_temp_cels,
    }
}

fn serialize_node_performance_samples(req: &crate::node_management::NodePerformanceSamples) -> crate::grpc_impl::api::NodePerformanceSamples {
    crate::grpc_impl::api::NodePerformanceSamples {
        function_execution_times: req
            .function_execution_times
            .iter()
            .map(|(id, samples)| crate::grpc_impl::api::Samples {
                id: id.to_string(),
                samples: samples.clone(),
            })
            .collect(),
    }
}

fn serialize_keep_alive_response(req: &crate::node_management::KeepAliveResponse) -> crate::grpc_impl::api::KeepAliveResponse {
    crate::grpc_impl::api::KeepAliveResponse {
        health_status: Some(serialize_node_health_status(&req.health_status)),
        performance_samples: Some(serialize_node_performance_samples(&req.performance_samples)),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::node_management::KeepAliveResponse;
    use crate::node_management::NodeHealthStatus;
    use crate::node_management::NodePerformanceSamples;
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
    fn serialize_deserialize_keep_alive_response() {
        let messages = vec![
            KeepAliveResponse::empty(),
            KeepAliveResponse {
                health_status: NodeHealthStatus::invalid(),
                performance_samples: NodePerformanceSamples::empty(),
            },
            KeepAliveResponse {
                health_status: NodeHealthStatus {
                    mem_free: 3,
                    mem_used: 4,
                    mem_available: 6,
                    proc_cpu_usage: 7,
                    proc_memory: 8,
                    proc_vmemory: 9,
                    load_avg_1: 10,
                    load_avg_5: 11,
                    load_avg_15: 12,
                    tot_rx_bytes: 13,
                    tot_rx_pkts: 14,
                    tot_rx_errs: 15,
                    tot_tx_bytes: 16,
                    tot_tx_pkts: 17,
                    tot_tx_errs: 18,
                    disk_free_space: 20,
                    disk_tot_reads: 21,
                    disk_tot_writes: 22,
                    gpu_load_perc: 23,
                    gpu_temp_cels: 24,
                },
                performance_samples: NodePerformanceSamples {
                    function_execution_times: std::collections::HashMap::from([
                        (uuid::Uuid::new_v4(), vec![1.0, 2.5, 3.0]),
                        (uuid::Uuid::new_v4(), vec![]),
                        (uuid::Uuid::new_v4(), vec![0.1, 0.2, 999.0]),
                    ]),
                },
            },
        ];
        for msg in messages {
            match parse_keep_alive_response(&serialize_keep_alive_response(&msg)) {
                Ok(val) => assert_eq!(msg, val),
                Err(err) => panic!("{}", err),
            }
        }
    }
}
