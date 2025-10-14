// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(PartialEq, Debug, Clone)]
/// Deployment requirements for functions, as specified by annotations.
pub struct DeploymentRequirements {
    /// Maximum number of function instances in this orchestration domain.
    /// 0 means unlimited.
    pub max_instances: usize,
    /// The function instance must be created on a node matching one
    /// of the given UUIDs, if any is given.
    pub node_id_match_any: Vec<uuid::Uuid>,
    /// The function instance must be created on a node that matches all
    /// the labels specified, if any is given.
    pub label_match_all: Vec<String>,
    /// The function instance must be created on a node that hosts all the
    /// resources providers specified, if any is given.
    pub resource_match_all: Vec<String>,
    /// Function instance's node affinity with Trusted Execution Environment.
    pub tee: crate::affinity_level::AffinityLevel,
    /// Function instance's node affinity with Trusted Platform Module.
    pub tpm: crate::affinity_level::AffinityLevel,
}

impl std::fmt::Display for DeploymentRequirements {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "max_instances {}, node_id_match_any {}, label_match_all {}, resource_match_all {}, tee {}, tpm {}",
            self.max_instances,
            self.node_id_match_any.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","),
            self.label_match_all.join(","),
            self.resource_match_all.join(","),
            self.tee,
            self.tpm
        )
    }
}

impl DeploymentRequirements {
    #[cfg(test)]
    /// No specific deployment requirements.
    pub fn none() -> Self {
        Self {
            max_instances: 0,
            node_id_match_any: vec![],
            label_match_all: vec![],
            resource_match_all: vec![],
            tee: crate::affinity_level::AffinityLevel::NotRequired,
            tpm: crate::affinity_level::AffinityLevel::NotRequired,
        }
    }
    /// Deployment requirements from the annotations in the function's spawn request.
    pub fn from_annotations(annotations: &std::collections::HashMap<String, String>) -> Self {
        let mut max_instances = 0;
        if let Some(val) = annotations.get("max_instances") {
            max_instances = val.parse::<usize>().unwrap_or_default();
        }

        let mut node_id_match_any = vec![];
        if let Some(val) = annotations.get("node_id_match_any") {
            node_id_match_any = val
                .split(",")
                .filter_map(|x| uuid::Uuid::parse_str(x).ok())
                .collect();
        }

        let mut label_match_all = vec![];
        if let Some(val) = annotations.get("label_match_all") {
            label_match_all = val.split(",").map(|x| x.to_string()).collect();
        }

        let mut resource_match_all = vec![];
        if let Some(val) = annotations.get("resource_match_all") {
            resource_match_all = val.split(",").map(|x| x.to_string()).collect();
        }

        let mut tee = crate::affinity_level::AffinityLevel::NotRequired;
        if let Some(val) = annotations.get("tee") {
            tee = crate::affinity_level::AffinityLevel::from_string(val);
        }

        let mut tpm = crate::affinity_level::AffinityLevel::NotRequired;
        if let Some(val) = annotations.get("tpm") {
            tpm = crate::affinity_level::AffinityLevel::from_string(val);
        }

        Self {
            max_instances,
            node_id_match_any,
            label_match_all,
            resource_match_all,
            tee,
            tpm,
        }
    }
}
