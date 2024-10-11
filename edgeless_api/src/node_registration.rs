// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceProviderSpecification {
    pub provider_id: String,
    pub class_type: String,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
    /// Create capabilities with all values empty.
    pub fn empty() -> Self {
        Self {
            num_cpus: 0,
            model_name_cpu: "".to_string(),
            clock_freq_cpu: 0.0,
            num_cores: 0,
            mem_size: 0,
            labels: vec![],
            is_tee_running: false,
            has_tpm: false,
            runtimes: vec![],
            disk_tot_space: 0,
            num_gpus: 0,
            model_name_gpu: "".to_string(),
            mem_size_gpu: 0,
        }
    }

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

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateNodeRequest {
    // 0: node_id (cannot be nil)
    // 1: agent_url (cannot be empty)
    // 2: invocation_url (cannot be empty)
    // 3: resource provider specifications (can be empty)
    // 4: node capabilities
    Registration(uuid::Uuid, String, String, Vec<ResourceProviderSpecification>, NodeCapabilities),

    // 0: node_id (cannot be empty)
    Deregistration(uuid::Uuid),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateNodeResponse {
    ResponseError(crate::common::ResponseError),
    Accepted,
}

#[async_trait::async_trait]
pub trait NodeRegistrationAPI: NodeRegistrationAPIClone + Sync + Send {
    async fn update_node(&mut self, request: UpdateNodeRequest) -> anyhow::Result<UpdateNodeResponse>;
    async fn keep_alive(&mut self);
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
