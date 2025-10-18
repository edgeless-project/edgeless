// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct ActiveWorkflow {
    // Workflow as it was requested by the client.
    pub desired_state: edgeless_api::workflow_instance::SpawnWorkflowRequest,

    // Workflow augmented with portal resources, if needed to enable
    // cross-domain interactions.
    pub augmented_spec: Option<edgeless_api::workflow_instance::SpawnWorkflowRequest>,

    // Mapping of each function/resource to a list of domains.
    pub domain_mapping: std::collections::HashMap<String, ActiveComponent>,
}

#[derive(Clone)]
pub struct ActiveComponent {
    // Function or resource.
    pub component_type: super::ComponentType,

    // Name of the function/resource within the workflow.
    pub name: String,

    // Name of the domain that manages the lifecycle of this function/resource.
    //
    // If the domain name is an empty string, then the component is current
    // not assigned to any domain, and the field `lid` is meaningless.
    //
    // [TODO] In principle a logical component could be mapped to _multiple_
    //  domains, in which case this field should be transformed in a container.
    pub domain_id: String,

    // Logical identifier of the function/resource.
    pub lid: edgeless_api::function_instance::ComponentId,
}

impl ActiveWorkflow {
    pub fn is_orphan(&self) -> bool {
        for component in self.domain_mapping.values() {
            if component.domain_id.is_empty() {
                return true;
            }
        }
        false
    }

    pub fn mapped_fids(&self, component_name: &str) -> Option<Vec<edgeless_api::function_instance::ComponentId>> {
        let comp = self.domain_mapping.get(component_name)?;
        Some(vec![comp.lid])
    }

    pub fn component_type(&self, component_name: &str) -> Option<super::ComponentType> {
        let item = self.domain_mapping.get(component_name);
        item.map(|item| item.component_type.clone())
    }

    /// Return the current domain assignments
    pub fn domain_assignments(&self) -> std::collections::HashMap<String, String> {
        self.domain_mapping
            .iter()
            .map(|(component, active)| (component.clone(), active.domain_id.clone()))
            .collect()
    }

    pub fn domain_mapping(&self) -> Vec<edgeless_api::workflow_instance::WorkflowFunctionMapping> {
        self.domain_mapping
            .iter()
            .map(|(name, component)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                name: name.clone(),
                function_id: component.lid,
                domain_id: component.domain_id.clone(),
            })
            .collect()
    }

    /// Convert a logical output_mapping to a physical one.
    ///
    /// Returned map:
    /// - key: channel name
    /// - value: PID
    pub fn physical_mapping(
        &self,
        logical_mapping: &std::collections::HashMap<String, String>,
    ) -> std::collections::HashMap<String, edgeless_api::function_instance::InstanceId> {
        let mut ret = std::collections::HashMap::new();

        // Loop on all the channels that needed to be
        // mapped for this function/resource.
        for (from_channel, to_name) in logical_mapping {
            // Loop on all the identifiers for the
            // target function/resource (once for each
            // assigned orchestration domain).
            for target_fid in self.mapped_fids(to_name).unwrap() {
                // [TODO] Issue#96 The output_mapping
                // structure should be changed so that
                // multiple values are possible (with
                // weights), and this change must be applied
                // to runners, as well.
                // For now, we just keep
                // overwriting the same entry.
                ret.insert(
                    from_channel.clone(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: target_fid,
                    },
                );
            }
        }

        ret
    }
}

impl std::fmt::Display for ActiveComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.component_type {
            super::ComponentType::Function => write!(f, "function name {}, domain {}, fid {}", self.name, self.domain_id, self.lid),
            super::ComponentType::Resource => write!(f, "resource name {}, domain {}, fid {}", self.name, self.domain_id, self.lid),
        }
    }
}
