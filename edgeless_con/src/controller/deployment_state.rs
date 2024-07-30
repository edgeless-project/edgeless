// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_api::grpc_impl::api::port_mapping;

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

pub enum LogicalOutput {
    Single((String, edgeless_api::function_instance::PortId)),
    Any(Vec<(String, edgeless_api::function_instance::PortId)>),
    All(Vec<(String, edgeless_api::function_instance::PortId)>),
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

    pub fn optimize_logical(&mut self) {
        self.convert_topic_ports();
    }

    fn convert_topic_ports(&mut self) {
        let mut targets = std::collections::HashMap::<String, Vec<(String, edgeless_api::function_instance::PortId)>>::new();

        // Find Targets
        for function in &mut self.desired_state.workflow_functions {
            function.input_mapping.retain(|port_id, port_mapping| match port_mapping {
                edgeless_api::workflow_instance::PortMapping::Topic(topic) => {
                    targets
                        .entry(topic.clone())
                        .or_insert(Vec::new())
                        .push((function.name.clone(), port_id.clone()));
                    false
                }
                _ => true,
            })
        }
        for resource in &mut self.desired_state.workflow_resources {
            resource.input_mapping.retain(|port_id, port_mapping| match port_mapping {
                edgeless_api::workflow_instance::PortMapping::Topic(topic) => {
                    targets
                        .entry(topic.clone())
                        .or_insert(Vec::new())
                        .push((resource.name.clone(), port_id.clone()));
                    false
                }
                _ => true,
            })
        }

        // Create Outputs
        for function in &mut self.desired_state.workflow_functions {
            function.output_mapping.iter_mut().for_each(|(_port_id, port_mapping)| {
                if let edgeless_api::workflow_instance::PortMapping::Topic(topic) = port_mapping.clone() {
                    *port_mapping = edgeless_api::workflow_instance::PortMapping::AllOfTargets(
                        targets
                            .get(&topic)
                            .unwrap_or(&Vec::<(String, edgeless_api::function_instance::PortId)>::new())
                            .clone(),
                    );
                }
            });
        }
        for resource in &mut self.desired_state.workflow_resources {
            resource.output_mapping.iter_mut().for_each(|(_port_id, port_mapping)| {
                if let edgeless_api::workflow_instance::PortMapping::Topic(topic) = port_mapping.clone() {
                    *port_mapping = edgeless_api::workflow_instance::PortMapping::AllOfTargets(
                        targets
                            .get(&topic)
                            .unwrap_or(&Vec::<(String, edgeless_api::function_instance::PortId)>::new())
                            .clone(),
                    );
                }
            });
        }
    }

    pub fn component_output_mapping(&self, component_name: &str) -> std::collections::HashMap<String, LogicalOutput> {
        
        if let Some(function) = self
            .desired_state
            .workflow_functions
            .iter()
            .find(|wf_function| wf_function.name == component_name)
        {
            return function
                .output_mapping
                .iter()
                .filter_map(|(port, dest_mapping)| match dest_mapping {
                    edgeless_api::workflow_instance::PortMapping::DirectTarget(component, dest_port) => {
                        Some((port.0.clone(), LogicalOutput::Single((component.clone(), dest_port.clone()))))
                    }
                    edgeless_api::workflow_instance::PortMapping::AnyOfTargets(targets) => {
                        Some((port.0.clone(), LogicalOutput::Any(targets.clone())))
                    }
                    edgeless_api::workflow_instance::PortMapping::AllOfTargets(targets) => {
                        Some((port.0.clone(), LogicalOutput::All(targets.clone())))
                    }
                    _ => None,
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
                .filter_map(|(port, dest_mapping)| match dest_mapping {
                    edgeless_api::workflow_instance::PortMapping::DirectTarget(component, port) => {
                        Some((port.0.clone(), LogicalOutput::Single((component.clone(), port.clone()))))
                    }
                    edgeless_api::workflow_instance::PortMapping::AnyOfTargets(targets) => {
                        Some((port.0.clone(), LogicalOutput::Any(targets.clone())))
                    }
                    edgeless_api::workflow_instance::PortMapping::AllOfTargets(targets) => {
                        Some((port.0.clone(), LogicalOutput::All(targets.clone())))
                    }
                    _ => None,
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
