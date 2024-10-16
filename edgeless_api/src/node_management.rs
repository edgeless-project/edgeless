// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq)]
pub enum UpdatePeersRequest {
    Add(uuid::Uuid, String), // node_id, invocation_url
    Del(uuid::Uuid),         // node_id
    Clear,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct NodePerformanceSamples {
    pub function_execution_times: std::collections::HashMap<crate::function_instance::ComponentId, Vec<f64>>,
}

impl NodePerformanceSamples {
    pub fn empty() -> Self {
        Self {
            function_execution_times: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeepAliveResponse {
    pub health_status: NodeHealthStatus,
    pub performance_samples: NodePerformanceSamples,
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
    pub fn empty() -> Self {
        Self {
            mem_free: 0,
            mem_used: 0,
            mem_available: 0,
            proc_cpu_usage: 0,
            proc_memory: 0,
            proc_vmemory: 0,
            load_avg_1: 0,
            load_avg_5: 0,
            load_avg_15: 0,
            tot_rx_bytes: 0,
            tot_rx_pkts: 0,
            tot_rx_errs: 0,
            tot_tx_bytes: 0,
            tot_tx_pkts: 0,
            tot_tx_errs: 0,
            disk_free_space: 0,
            disk_tot_reads: 0,
            disk_tot_writes: 0,
            gpu_load_perc: 0,
            gpu_temp_cels: 0,
        }
    }

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

impl KeepAliveResponse {
    pub fn empty() -> Self {
        Self {
            health_status: NodeHealthStatus::empty(),
            performance_samples: NodePerformanceSamples::empty(),
        }
    }
}

#[async_trait::async_trait]
pub trait NodeManagementAPI: NodeManagementAPIClone + Sync + Send {
    async fn update_peers(&mut self, request: UpdatePeersRequest) -> anyhow::Result<()>;
    async fn keep_alive(&mut self) -> anyhow::Result<KeepAliveResponse>;
}

// https://stackoverflow.com/a/30353928
pub trait NodeManagementAPIClone {
    fn clone_box(&self) -> Box<dyn NodeManagementAPI>;
}
impl<T> NodeManagementAPIClone for T
where
    T: 'static + NodeManagementAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn NodeManagementAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn NodeManagementAPI> {
    fn clone(&self) -> Box<dyn NodeManagementAPI> {
        self.clone_box()
    }
}
