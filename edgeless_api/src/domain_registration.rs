// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DomainCapabilities {
    // Number of nodes.
    pub num_nodes: u32,
    // Total number of (actual or virtual) CPUs.
    pub num_cpus: u32,
    // Total number of physical cores.
    pub num_cores: u32,
    // Total size of memory available, in MiB.
    pub mem_size: u32,
    // Superset of all the labels advertised by the nodes in the domain.
    pub labels: std::collections::HashSet<String>,
    // Number of nodes with a Trusted Execution Environment.
    pub num_tee: u32,
    // Number of nodes with a Trusted Platform Module.
    pub num_tpm: u32,
    // Superset of all the run-times supported by the nodes in the domain.
    pub runtimes: std::collections::HashSet<String>,
    // Total disk space, in MiB.
    pub disk_tot_space: u32,
    // Total number of (actual or virtual) GPUs.
    pub num_gpus: u32,
    // Total GPU memory available, in MiB.
    pub mem_size_gpu: u32,
    // Superset of the names of the resource providers advertised by the nodes.
    pub resource_providers: std::collections::HashSet<String>,
    // Superset of the classes of the resource providers advertised by the nodes.
    pub resource_classes: std::collections::HashSet<String>,
}

impl std::fmt::Display for DomainCapabilities {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} nodes, {} CPUs ({} cores) with {} MiB, labels [{}], num TEE {}, num TPM {}, runtimes [{}], disk space {} MiB, {} GPUs with {} MiB",
            self.num_nodes,
            self.num_cpus,
            self.num_cores,
            self.mem_size,
            self.labels.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
            self.num_tee,
            self.num_tpm,
            self.runtimes.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
            self.disk_tot_space,
            self.num_gpus,
            self.mem_size_gpu,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateDomainRequest {
    // Domain name.
    pub domain_id: String,
    // URL of the orchestrator server.
    pub orchestrator_url: String,
    // Domain capabilities.
    pub capabilities: DomainCapabilities,
    // Deadline for refreshing the domain request.
    // After this time the orchestration domain can be considered to be offline.
    pub refresh_deadline: std::time::SystemTime,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateDomainResponse {
    ResponseError(crate::common::ResponseError),
    Accepted,
}

#[async_trait::async_trait]
pub trait DomainRegistrationAPI: DomainRegistrationAPIClone + Sync + Send {
    async fn update_domain(&mut self, request: UpdateDomainRequest) -> anyhow::Result<UpdateDomainResponse>;
}

// https://stackoverflow.com/a/30353928
pub trait DomainRegistrationAPIClone {
    fn clone_box(&self) -> Box<dyn DomainRegistrationAPI>;
}
impl<T> DomainRegistrationAPIClone for T
where
    T: 'static + DomainRegistrationAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn DomainRegistrationAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn DomainRegistrationAPI> {
    fn clone(&self) -> Box<dyn DomainRegistrationAPI> {
        self.clone_box()
    }
}
