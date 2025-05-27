// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use std::str::FromStr;

#[derive(Clone)]
pub struct NodeRegistrationClient {
    client: Option<crate::grpc_impl::api::node_registration_client::NodeRegistrationClient<tonic::transport::Channel>>,
    server_addr: String,
}

pub struct NodeRegistrationAPIService {
    pub node_registration_api: tokio::sync::Mutex<Box<dyn crate::node_registration::NodeRegistrationAPI>>,
}

impl NodeRegistrationClient {
    pub fn new(server_addr: String) -> Self {
        Self { client: None, server_addr }
    }

    /// Try connecting, if not already connected.
    ///
    /// If an error is returned, then the client is set to None (disconnected).
    /// Otherwise, the client is set to some value (connected).
    async fn try_connect(&mut self) -> anyhow::Result<()> {
        if self.client.is_none() {
            self.client = match crate::grpc_impl::api::node_registration_client::NodeRegistrationClient::connect(self.server_addr.clone()).await {
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
impl crate::node_registration::NodeRegistrationAPI for NodeRegistrationClient {
    async fn update_node(
        &mut self,
        request: crate::node_registration::UpdateNodeRequest,
    ) -> anyhow::Result<crate::node_registration::UpdateNodeResponse> {
        match self.try_connect().await {
            Ok(_) => {
                if let Some(client) = &mut self.client {
                    match client.update_node(tonic::Request::new(serialize_update_node_request(&request))).await {
                        Ok(res) => parse_update_node_response(&res.into_inner()),
                        Err(err) => {
                            self.disconnect();
                            Err(anyhow::anyhow!("Error when updating a node at {}: {}", self.server_addr, err.to_string()))
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
        labels: api_instance.labels.clone(),
        is_tee_running: api_instance.is_tee_running,
        has_tpm: api_instance.has_tpm,
        runtimes: api_instance.runtimes.clone(),
        disk_tot_space: api_instance.disk_tot_space,
        num_gpus: api_instance.num_gpus,
        model_name_gpu: api_instance.model_name_gpu.clone(),
        mem_size_gpu: api_instance.mem_size_gpu,
    }
}

fn serialize_node_capabilities(req: &crate::node_registration::NodeCapabilities) -> crate::grpc_impl::api::NodeCapabilities {
    crate::grpc_impl::api::NodeCapabilities {
        num_cpus: req.num_cpus,
        model_name_cpu: req.model_name_cpu.clone(),
        clock_freq_cpu: req.clock_freq_cpu,
        num_cores: req.num_cores,
        mem_size: req.mem_size,
        labels: req.labels.clone(),
        is_tee_running: req.is_tee_running,
        has_tpm: req.has_tpm,
        runtimes: req.runtimes.clone(),
        disk_tot_space: req.disk_tot_space,
        num_gpus: req.num_gpus,
        model_name_gpu: req.model_name_gpu.clone(),
        mem_size_gpu: req.mem_size_gpu,
    }
}

fn parse_update_node_request(api_instance: &crate::grpc_impl::api::UpdateNodeRequest) -> anyhow::Result<crate::node_registration::UpdateNodeRequest> {
    let node_id = uuid::Uuid::from_str(api_instance.node_id.as_str())?;
    let mut resource_providers = vec![];
    for resource_provider in &api_instance.resource_providers {
        match parse_resource_provider_specification(resource_provider) {
            Ok(val) => resource_providers.push(val),
            Err(err) => {
                return Err(anyhow::anyhow!("Ill-formed resource provider in UpdateNodeRequest message: {}", err));
            }
        }
    }

    Ok(crate::node_registration::UpdateNodeRequest {
        node_id,
        agent_url: api_instance.agent_url.clone(),
        invocation_url: api_instance.invocation_url.clone(),
        resource_providers,
        refresh_deadline: std::time::UNIX_EPOCH + std::time::Duration::from_secs(api_instance.refresh_deadline),
        capabilities: match &api_instance.capabilities {
            Some(val) => parse_node_capabilities(val),
            None => crate::node_registration::NodeCapabilities::default(),
        },
        nonce: api_instance.nonce,
        health_status: match &api_instance.health_status {
            Some(val) => parse_node_health_status(val),
            None => crate::node_registration::NodeHealthStatus::default(),
        },
        performance_samples: match &api_instance.performance_samples {
            Some(val) => parse_node_performance_samples(val),
            None => crate::node_registration::NodePerformanceSamples::default(),
        },
    })
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
    crate::grpc_impl::api::UpdateNodeRequest {
        node_id: req.node_id.to_string(),
        agent_url: req.agent_url.clone(),
        invocation_url: req.invocation_url.clone(),
        resource_providers: req.resource_providers.iter().map(serialize_resource_provider_specification).collect(),
        capabilities: Some(serialize_node_capabilities(&req.capabilities)),
        refresh_deadline: req.refresh_deadline.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        nonce: req.nonce,
        health_status: Some(serialize_node_health_status(&req.health_status)),
        performance_samples: Some(serialize_node_performance_samples(&req.performance_samples)),
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

pub fn parse_node_health_status(api_instance: &crate::grpc_impl::api::NodeHealthStatus) -> crate::node_registration::NodeHealthStatus {
    crate::node_registration::NodeHealthStatus {
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
        active_power: api_instance.active_power,
    }
}

fn serialize_node_health_status(req: &crate::node_registration::NodeHealthStatus) -> crate::grpc_impl::api::NodeHealthStatus {
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
        active_power: req.active_power,
    }
}

pub fn parse_sample(api_instance: &crate::grpc_impl::api::Sample) -> crate::node_registration::Sample {
    crate::node_registration::Sample {
        timestamp_sec: api_instance.timestamp_sec,
        timestamp_ns: api_instance.timestamp_ns,
        sample: api_instance.sample,
    }
}

pub fn parse_function_log_entry(api_instance: &crate::grpc_impl::api::FunctionLogEntry) -> crate::node_registration::FunctionLogEntry {
    crate::node_registration::FunctionLogEntry {
        timestamp_sec: api_instance.timestamp_sec,
        timestamp_ns: api_instance.timestamp_ns,
        target: api_instance.target.clone(),
        message: api_instance.msg.clone(),
    }
}

pub fn parse_node_performance_samples(
    api_instance: &crate::grpc_impl::api::NodePerformanceSamples,
) -> crate::node_registration::NodePerformanceSamples {
    crate::node_registration::NodePerformanceSamples {
        function_execution_times: api_instance
            .function_execution_times
            .iter()
            .filter_map(|x| match uuid::Uuid::from_str(&x.id) {
                Ok(val) => Some((val, x.samples.iter().map(parse_sample).collect())),
                _ => None,
            })
            .collect(),
        function_transfer_times: api_instance
            .function_transfer_times
            .iter()
            .filter_map(|x| match uuid::Uuid::from_str(&x.id) {
                Ok(val) => Some((val, x.samples.iter().map(parse_sample).collect())),
                _ => None,
            })
            .collect(),
        function_log_entries: api_instance
            .function_log_entries
            .iter()
            .filter_map(|x| match uuid::Uuid::from_str(&x.id) {
                Ok(val) => Some((val, x.entries.iter().map(parse_function_log_entry).collect())),
                _ => None,
            })
            .collect(),
    }
}

fn serialize_sample(req: &crate::node_registration::Sample) -> crate::grpc_impl::api::Sample {
    crate::grpc_impl::api::Sample {
        timestamp_sec: req.timestamp_sec,
        timestamp_ns: req.timestamp_ns,
        sample: req.sample,
    }
}

fn serialize_function_log_entry(req: &crate::node_registration::FunctionLogEntry) -> crate::grpc_impl::api::FunctionLogEntry {
    crate::grpc_impl::api::FunctionLogEntry {
        timestamp_sec: req.timestamp_sec,
        timestamp_ns: req.timestamp_ns,
        target: req.target.clone(),
        msg: req.message.clone(),
    }
}

fn serialize_node_performance_samples(req: &crate::node_registration::NodePerformanceSamples) -> crate::grpc_impl::api::NodePerformanceSamples {
    crate::grpc_impl::api::NodePerformanceSamples {
        function_execution_times: req
            .function_execution_times
            .iter()
            .map(|(id, samples)| crate::grpc_impl::api::Samples {
                id: id.to_string(),
                samples: samples.iter().map(serialize_sample).collect(),
            })
            .collect(),
        function_transfer_times: req
            .function_transfer_times
            .iter()
            .map(|(id, samples)| crate::grpc_impl::api::Samples {
                id: id.to_string(),
                samples: samples.iter().map(serialize_sample).collect(),
            })
            .collect(),
        function_log_entries: req
            .function_log_entries
            .iter()
            .map(|(id, entries)| crate::grpc_impl::api::FunctionLogEntries {
                id: id.to_string(),
                entries: entries.iter().map(serialize_function_log_entry).collect(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::node_registration::FunctionLogEntry;
    use crate::node_registration::NodeCapabilities;
    use crate::node_registration::NodeHealthStatus;
    use crate::node_registration::NodePerformanceSamples;
    use crate::node_registration::ResourceProviderSpecification;
    use crate::node_registration::Sample;
    use crate::node_registration::UpdateNodeRequest;
    use crate::node_registration::UpdateNodeResponse;

    #[test]
    fn serialize_deserialize_update_node_request() {
        let mut sample_cnt = 0;
        let mut new_sample = |value| {
            sample_cnt += 2;
            Sample {
                timestamp_sec: sample_cnt as i64,
                timestamp_ns: (sample_cnt + 1) as u32,
                sample: value,
            }
        };

        let mut log_cnt = 0;
        let mut new_log = |value| {
            log_cnt += 2;
            FunctionLogEntry {
                timestamp_sec: log_cnt as i64,
                timestamp_ns: (log_cnt + 1) as u32,
                target: String::from("target"),
                message: format!("value={}", value),
            }
        };

        let messages = vec![UpdateNodeRequest {
            node_id: uuid::Uuid::new_v4(),
            agent_url: "http://127.0.0.1:10000".to_string(),
            invocation_url: "http://127.0.0.1:10001".to_string(),
            resource_providers: vec![ResourceProviderSpecification {
                provider_id: "provider-1".to_string(),
                class_type: "class-type-1".to_string(),
                outputs: vec!["out1".to_string(), "out2".to_string()],
            }],
            capabilities: NodeCapabilities {
                num_cpus: 4,
                model_name_cpu: "ARMv8 Processor rev 0 (v8l)".to_string(),
                clock_freq_cpu: 62.50,
                num_cores: 20,
                mem_size: 15827,
                labels: vec!["red".to_string(), "powerful".to_string()],
                is_tee_running: true,
                has_tpm: true,
                runtimes: vec!["RUST_WASM".to_string()],
                disk_tot_space: 999,
                num_gpus: 3,
                model_name_gpu: "NVIDIA A100".to_string(),
                mem_size_gpu: 80 * 1024,
            },
            refresh_deadline: std::time::UNIX_EPOCH + std::time::Duration::from_secs(313714800),
            nonce: 1,
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
                active_power: 25,
            },
            performance_samples: NodePerformanceSamples {
                function_execution_times: std::collections::HashMap::from([
                    (uuid::Uuid::new_v4(), vec![new_sample(1.0), new_sample(2.5), new_sample(3.0)]),
                    (uuid::Uuid::new_v4(), vec![]),
                    (uuid::Uuid::new_v4(), vec![new_sample(0.1), new_sample(0.2), new_sample(999.0)]),
                ]),
                function_transfer_times: std::collections::HashMap::from([
                    (uuid::Uuid::new_v4(), vec![new_sample(1.0), new_sample(2.5), new_sample(3.0)]),
                    (uuid::Uuid::new_v4(), vec![]),
                    (uuid::Uuid::new_v4(), vec![new_sample(0.1), new_sample(0.2), new_sample(999.0)]),
                ]),
                function_log_entries: std::collections::HashMap::from([(uuid::Uuid::new_v4(), vec![new_log(100.0), new_log(200.1)])]),
            },
        }];
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
