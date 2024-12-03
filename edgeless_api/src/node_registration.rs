// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceProviderSpecification {
    pub provider_id: String,
    pub class_type: String,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct NodeCapabilities {
    // Number of (actual or virtual) CPUs associated with the edge node.
    pub num_cpus: u32,
    // Name of the CPU model.
    pub model_name_cpu: String,
    // Clock frequency of the CPU, in BogoMIPS.
    pub clock_freq_cpu: f32,
    // Number of cores for each CPU.
    pub num_cores: u32,
    // Size of memory available to applications running on the edge node, in MiB.
    pub mem_size: u32,
    // List of labels assigned to this node.
    pub labels: Vec<String>,
    // True if the node is running inside a Trusted Execution Environment.
    pub is_tee_running: bool,
    // True if the node has a Trusted Platform Module for authenticated registration.
    pub has_tpm: bool,
    // List of run-times supported by the node.
    pub runtimes: Vec<String>,
    // Total disk space, in MiB.
    pub disk_tot_space: u32,
    // Number of (actual or virtual) GPUs associated with the edge node.
    pub num_gpus: u32,
    // Name of the GPU model.
    pub model_name_gpu: String,
    // GPU memory available, in MiB.
    pub mem_size_gpu: u32,
}

impl NodeCapabilities {
    /// Create a usable node with minimum capabilities.
    pub fn minimum() -> Self {
        Self {
            num_cpus: 1,
            model_name_cpu: "".to_string(),
            clock_freq_cpu: 0.0,
            num_cores: 1,
            mem_size: 0,
            labels: vec![],
            is_tee_running: false,
            has_tpm: false,
            runtimes: vec!["RUST_WASM".to_string()],
            disk_tot_space: 0,
            num_gpus: 0,
            model_name_gpu: "".to_string(),
            mem_size_gpu: 0,
        }
    }

    /// Return true if this node must not be assigned function instances.
    pub fn do_not_use(&self) -> bool {
        self.num_cpus * self.num_cores == 0
    }

    pub fn csv_header() -> String {
        "num_cpus,model_name_cpu,clock_freq_cpu,num_cores,mem_size,labels,is_tee_running,has_tpm,runtimes,disk_tot_space,num_gpus,model_name_gpu,mem_size_gpu".to_string()
    }

    pub fn to_csv(&self) -> String {
        format!(
            "{},{},{},{},{},[{}],{},{},[{}],{},{},{},{}",
            self.num_cpus,
            self.model_name_cpu,
            self.clock_freq_cpu,
            self.num_cores,
            self.mem_size,
            self.labels.join(";"),
            self.is_tee_running,
            self.has_tpm,
            self.runtimes.join(";"),
            self.disk_tot_space,
            self.num_gpus,
            self.model_name_gpu,
            self.mem_size_gpu,
        )
    }
}

impl std::fmt::Display for NodeCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} CPU(s) at {} BogoMIPS, {} core(s), {} MiB memory, labels [{}]{}{}, runtimes [{}], disk space {} MiB, {} {} GPU(s) {} MiB",
            self.num_cpus,
            self.model_name_cpu,
            self.clock_freq_cpu,
            self.num_cores,
            self.mem_size,
            self.labels.join(","),
            match self.is_tee_running {
                true => ", TEE",
                false => "",
            },
            match self.has_tpm {
                true => ", TPM",
                false => "",
            },
            self.runtimes.join(","),
            self.disk_tot_space,
            self.num_gpus,
            self.model_name_gpu,
            self.mem_size_gpu,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct NodeHealthStatus {
    pub mem_free: i32,
    pub mem_used: i32,
    pub mem_available: i32,
    pub proc_cpu_usage: i32,
    pub proc_memory: i32,
    pub proc_vmemory: i32,
    pub load_avg_1: i32,
    pub load_avg_5: i32,
    pub load_avg_15: i32,
    pub tot_rx_bytes: i64,
    pub tot_rx_pkts: i64,
    pub tot_rx_errs: i64,
    pub tot_tx_bytes: i64,
    pub tot_tx_pkts: i64,
    pub tot_tx_errs: i64,
    pub disk_free_space: i64,
    pub disk_tot_reads: i64,
    pub disk_tot_writes: i64,
    pub gpu_load_perc: i32,
    pub gpu_temp_cels: i32,
}

impl NodeHealthStatus {
    pub fn csv_header() -> String {
        "cpu_usage,cpu_load,mem_free,mem_used,mem_total,mem_available,proc_cpu_usage,proc_memory,proc_vmemory,load_avg_1,load_avg_5,load_avg_15,tot_rx_bytes,tot_rx_pkts,tot_rx_errs,tot_tx_bytes,tot_tx_pkts,tot_tx_errs,disk_tot_space,disk_free_space,disk_tot_reads,disk_tot_writes,gpu_load_perc,gpu_temp_cels".to_string()
    }
    pub fn to_csv(&self) -> String {
        format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            self.mem_free,
            self.mem_used,
            self.mem_available,
            self.proc_cpu_usage,
            self.proc_memory,
            self.proc_vmemory,
            self.load_avg_1,
            self.load_avg_5,
            self.load_avg_15,
            self.tot_rx_bytes,
            self.tot_rx_pkts,
            self.tot_rx_errs,
            self.tot_tx_bytes,
            self.tot_tx_pkts,
            self.tot_tx_errs,
            self.disk_free_space,
            self.disk_tot_reads,
            self.disk_tot_writes,
            self.gpu_load_perc,
            (self.gpu_temp_cels as f32 / 1000.0),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NodePerformanceSamples {
    pub function_execution_times: std::collections::HashMap<crate::function_instance::ComponentId, Vec<f64>>,
}

impl std::fmt::Display for NodeHealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "memory free {} kb, used {} kb, available {} kb, process cpu usage {:.1}%, memory {} kb, vmemory {} kb, load avg 1 minute {}% 5 minutes {}% 15 minutes {}%, network tot rx {} bytes ({} pkts) {} errs, tot tx {} bytes ({} pkts) {} errs, disk available {} bytes, tot disk reads {} writes {}, gpu_load_perc {}%, gpu_temp_cels {:.2}°",
            self.mem_free,
            self.mem_used,
            self.mem_available,
            self.proc_cpu_usage,
            self.proc_memory,
            self.proc_vmemory,
            self.load_avg_1,
            self.load_avg_5,
            self.load_avg_15,
            self.tot_rx_bytes,
            self.tot_rx_pkts,
            self.tot_rx_errs,
            self.tot_tx_bytes,
            self.tot_tx_pkts,
            self.tot_tx_errs,
            self.disk_free_space,
            self.disk_tot_reads,
            self.disk_tot_writes,
            self.gpu_load_perc,
            (self.gpu_temp_cels as f32 / 1000.0)
        )
    }
}

impl NodeHealthStatus {
    pub fn invalid() -> Self {
        Self {
            mem_free: -1,
            mem_used: -1,
            mem_available: -1,
            proc_cpu_usage: -1,
            proc_memory: -1,
            proc_vmemory: -1,
            load_avg_1: -1,
            load_avg_5: -1,
            load_avg_15: -1,
            tot_rx_bytes: -1,
            tot_rx_pkts: -1,
            tot_rx_errs: -1,
            tot_tx_bytes: -1,
            tot_tx_pkts: -1,
            tot_tx_errs: -1,
            disk_free_space: -1,
            disk_tot_reads: -1,
            disk_tot_writes: -1,
            gpu_load_perc: -1,
            gpu_temp_cels: -1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateNodeRequest {
    // Node identifier.
    pub node_id: uuid::Uuid,
    // URL of the node's agent server.
    pub agent_url: String,
    // URL of the node's invocation server.
    pub invocation_url: String,
    // Resources offered by this node.
    pub resource_providers: Vec<ResourceProviderSpecification>,
    // Node capabilities.
    pub capabilities: NodeCapabilities,
    // Deadline for refreshing the node request, in seconds since Unix epoch.
    // After this time the node can be considered to be offline.
    pub refresh_deadline: std::time::SystemTime,
    // Incremental counter updated every time the capabilities change.
    pub counter: u64,
    // Node health status.
    pub health_status: NodeHealthStatus,
    // Node performance info.
    pub performance_samples: NodePerformanceSamples,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateNodeResponse {
    ResponseError(crate::common::ResponseError),
    Accepted,
}

#[async_trait::async_trait]
pub trait NodeRegistrationAPI: NodeRegistrationAPIClone + Sync + Send {
    async fn update_node(&mut self, request: UpdateNodeRequest) -> anyhow::Result<UpdateNodeResponse>;
}

impl std::fmt::Display for ResourceProviderSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "provider_id {}, class_type {}, outputs [{}]",
            self.provider_id,
            self.class_type,
            self.outputs.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
        )
    }
}

// https://stackoverflow.com/a/30353928
pub trait NodeRegistrationAPIClone {
    fn clone_box(&self) -> Box<dyn NodeRegistrationAPI>;
}
impl<T> NodeRegistrationAPIClone for T
where
    T: 'static + NodeRegistrationAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn NodeRegistrationAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn NodeRegistrationAPI> {
    fn clone(&self) -> Box<dyn NodeRegistrationAPI> {
        self.clone_box()
    }
}
