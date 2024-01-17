// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceProviderSpecification {
    pub provider_id: String,
    pub class_type: String,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeCapabilities {
    // Number of (actual or virtual) CPUs associated with the edge node.
    pub num_cpus: u32,
    // Name of the CPU model.
    pub model_name_cpu: String,
    // Clock frequency of the CPU, in BogoMIPS.
    pub clock_freq_cpu: f32,
    // Number of cores for each CPU.
    pub num_cores: u32,
    // Size of memory available to applications running on the edge node, in MB.
    pub mem_size: u32,
}

impl NodeCapabilities {
    pub fn default() -> Self {
        Self {
            num_cpus: 0,
            model_name_cpu: "".to_string(),
            clock_freq_cpu: 0.0,
            num_cores: 0,
            mem_size: 0,
        }
    }
}

impl std::fmt::Display for NodeCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} CPU(s) at {} BogoMIPS, {} core(s), {} MB memory",
            self.num_cpus, self.model_name_cpu, self.clock_freq_cpu, self.num_cores, self.mem_size,
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
