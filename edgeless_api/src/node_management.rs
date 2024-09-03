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
pub struct HealthStatus {
    pub cpu_usage: i32,
    pub cpu_load: i32,
    pub mem_free: i32,
    pub mem_used: i32,
    pub mem_total: i32,
    pub mem_available: i32,
    pub proc_cpu_usage: i32,
    pub proc_memory: i32,
    pub proc_vmemory: i32,
    pub function_execution_times: std::collections::HashMap<crate::function_instance::ComponentId, Vec<f32>>,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "global cpu usage {:.1}%, load {}, memory free {} kb, used {} kb, total {} kb, available {} kb, process cpu usage {:.1}%, memory {} kb, vmemory {} kb",
            self.cpu_usage,
            self.cpu_load,
            self.mem_free,
            self.mem_used,
            self.mem_total,
            self.mem_available,
            self.proc_cpu_usage,
            self.proc_memory,
            self.proc_vmemory,
        )
    }
}

impl HealthStatus {
    pub fn empty() -> Self {
        Self {
            cpu_usage: 0,
            cpu_load: 0,
            mem_free: 0,
            mem_used: 0,
            mem_total: 0,
            mem_available: 0,
            proc_cpu_usage: 0,
            proc_memory: 0,
            proc_vmemory: 0,
            function_execution_times: std::collections::HashMap::new(),
        }
    }

    pub fn invalid() -> Self {
        Self {
            cpu_usage: -1,
            cpu_load: -1,
            mem_free: -1,
            mem_used: -1,
            mem_total: -1,
            mem_available: -1,
            proc_cpu_usage: -1,
            proc_memory: -1,
            proc_vmemory: -1,
            function_execution_times: std::collections::HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
pub trait NodeManagementAPI: NodeManagementAPIClone + Sync + Send {
    async fn update_peers(&mut self, request: UpdatePeersRequest) -> anyhow::Result<()>;
    async fn keep_alive(&mut self) -> anyhow::Result<HealthStatus>;
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
