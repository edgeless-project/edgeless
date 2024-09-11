// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

// Originally copied over from edgeless_orc.

#[derive(PartialEq, Debug, Clone)]
pub enum AffinityLevel {
    Required,
    NotRequired,
}

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
    pub tee: AffinityLevel,
    /// Function instance's node affinity with Trusted Platform Module.
    pub tpm: AffinityLevel,
}

impl std::fmt::Display for AffinityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                AffinityLevel::Required => "required",
                AffinityLevel::NotRequired => "not-required",
            }
        )
    }
}

impl AffinityLevel {
    pub fn from_string(val: &str) -> Self {
        if val.to_lowercase() == "required" {
            AffinityLevel::Required
        } else {
            AffinityLevel::NotRequired
        }
    }
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
            tee: AffinityLevel::NotRequired,
            tpm: AffinityLevel::NotRequired,
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
            node_id_match_any = val.split(",").filter_map(|x| uuid::Uuid::parse_str(x).ok()).collect();
        }

        let mut label_match_all = vec![];
        if let Some(val) = annotations.get("label_match_all") {
            label_match_all = val.split(",").map(|x| x.to_string()).collect();
        }

        let mut resource_match_all = vec![];
        if let Some(val) = annotations.get("resource_match_all") {
            resource_match_all = val.split(",").map(|x| x.to_string()).collect();
        }

        let mut tee = AffinityLevel::NotRequired;
        if let Some(val) = annotations.get("tee") {
            tee = AffinityLevel::from_string(val);
        }

        let mut tpm = AffinityLevel::NotRequired;
        if let Some(val) = annotations.get("tpm") {
            tpm = AffinityLevel::from_string(val);
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum OrchestrationStrategy {
    /// Random strategy utilizes a random number generator to select the worker
    /// node where a function instance is started. It is the default strategy.
    Random,
}
