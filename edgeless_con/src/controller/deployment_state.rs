// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Clone)]
pub struct ActiveWorkflow {
    // Workflow as it was requested by the client.
    pub desired_state: edgeless_api::workflow_instance::SpawnWorkflowRequest,

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
    pub domain_id: String,

    // Identifier returned by the e-ORC.
    pub fid: edgeless_api::function_instance::ComponentId,
}

impl ActiveWorkflow {
    pub fn mapped_fids(&self, component_name: &str) -> Option<Vec<edgeless_api::function_instance::ComponentId>> {
        let comp = self.domain_mapping.get(component_name)?;
        Some(vec![comp.fid])
    }

    pub fn component_type(&self, component_name: &str) -> Option<super::ComponentType> {
        let item = self.domain_mapping.get(component_name);
        if let Some(item) = item {
            return Some(item.component_type.clone());
        } else {
            return None;
        }
    }

    pub fn domain_mapping(&self) -> Vec<edgeless_api::workflow_instance::WorkflowFunctionMapping> {
        self.domain_mapping
            .iter()
            .map(|(name, component)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                name: name.clone(),
                domain_id: component.domain_id.clone(),
            })
            .collect()
    }

    pub fn components(&self) -> Vec<String> {
        // Collect all the names+output_mapping from the
        // functions and resources of this workflow.
        let mut component_names = vec![];
        for function in &self.desired_state.workflow_functions {
            component_names.push(function.name.clone());
        }
        for resource in &self.desired_state.workflow_resources {
            component_names.push(resource.name.clone());
        }
        component_names
    }

    pub fn component_output_mapping(&self, component_name: &str) -> std::collections::HashMap<String, String> {
        if let Some(function) = self
            .desired_state
            .workflow_functions
            .iter()
            .find(|wf_function| wf_function.name == component_name)
        {
            return function
                .output_mapping
                .iter()
                .filter_map(|(port, dest_mapping)| {
                    if let edgeless_api::workflow_instance::PortMapping::DirectTarget(component, _port) = dest_mapping {
                        Some((port.0.clone(), component.clone()))
                    } else {
                        None
                    }
                })
                .collect();
        }

        if let Some(resource) = self
            .desired_state
            .workflow_resources
            .iter()
            .find(|wf_resource| wf_resource.name == component_name)
        {
            return resource
                .output_mapping
                .iter()
                .filter_map(|(port, dest_mapping)| {
                    if let edgeless_api::workflow_instance::PortMapping::DirectTarget(component, _port) = dest_mapping {
                        Some((port.0.clone(), component.clone()))
                    } else {
                        None
                    }
                })
                .collect();
        }

        std::collections::HashMap::new()
    }
}

impl std::fmt::Display for ActiveComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.component_type {
            super::ComponentType::Function => write!(f, "function name {}, domain {}, fid {}", self.name, self.domain_id, self.fid),
            super::ComponentType::Resource => write!(f, "resource name {}, domain {}, fid {}", self.name, self.domain_id, self.fid),
        }
    }
}
