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

    // // Name of the domain that manages the lifecycle of this function/resource.
    // pub domain_id: String,

    // Identifier returned by the e-ORC.
    pub fid: edgeless_api::function_instance::InstanceId,
}

pub enum LogicalOutput {
    Single((String, edgeless_api::function_instance::PortId)),
    Any(Vec<(String, edgeless_api::function_instance::PortId)>),
    All(Vec<(String, edgeless_api::function_instance::PortId)>),
}

impl ActiveWorkflow {
    pub fn mapped_fids(&self, component_name: &str) -> Option<Vec<edgeless_api::function_instance::InstanceId>> {
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

    pub fn active_inputs(&self, component_name: &str) -> Vec<edgeless_api::function_instance::PortId> {
        let mut items = std::collections::HashSet::new();

        let mut outputs: Vec<_> = self
            .desired_state
            .workflow_functions
            .iter()
            .flat_map(|wf| wf.output_mapping.values().collect::<Vec<_>>())
            .collect();
        outputs.append(
            &mut self
                .desired_state
                .workflow_resources
                .iter()
                .flat_map(|wr| wr.output_mapping.values().collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        );
        for port_mapping in outputs.iter() {
            match port_mapping {
                edgeless_api::workflow_instance::PortMapping::DirectTarget(component, port) => {
                    if component == component_name {
                        items.insert(port.clone());
                    }
                }
                edgeless_api::workflow_instance::PortMapping::AnyOfTargets(targets) => {
                    for (component, port) in targets {
                        if component == component_name {
                            items.insert(port.clone());
                        }
                    }
                }
                edgeless_api::workflow_instance::PortMapping::AllOfTargets(targets) => {
                    for (component, port) in targets {
                        if component == component_name {
                            items.insert(port.clone());
                        }
                    }
                }
                edgeless_api::workflow_instance::PortMapping::Topic(_) => {
                    panic!("This should have been replaced!");
                }
            }
        }

        items.into_iter().collect()
    }

    pub fn active_outputs(&self, component_name: &str) -> Vec<edgeless_api::function_instance::PortId> {
        for wf in &self.desired_state.workflow_functions {
            if wf.name == component_name {
                return wf.output_mapping.keys().map(|port| port.clone()).collect();
            }
        }
        for wr in &self.desired_state.workflow_resources {
            if wr.name == component_name {
                return wr.output_mapping.keys().map(|port| port.clone()).collect();
            }
        }
        return Vec::new();
    }

    pub fn domain_mapping(&self) -> Vec<edgeless_api::workflow_instance::WorkflowFunctionMapping> {
        self.domain_mapping
            .iter()
            .map(|(name, component)| edgeless_api::workflow_instance::WorkflowFunctionMapping {
                name: name.clone(),
                domain_id: component.fid.node_id.clone().to_string(),
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
        self.remove_unused_links();
    }

    fn remove_unused_links(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            changed = self.remove_unused_inputs() || changed;
            changed = self.remove_unused_outputs() || changed;
        }
    }

    fn remove_unused_outputs(&mut self) -> bool {
        let mut changed = false;

        let active_inputs: std::collections::HashMap<_, _> = self
            .desired_state
            .workflow_functions
            .iter()
            .map(|wf| (wf.name.to_string(), self.active_inputs(&wf.name)))
            .collect();

        self.desired_state.workflow_functions.iter_mut().for_each(|wf| {
            let active_inputs = active_inputs.get(&wf.name).unwrap();
            wf.output_mapping.retain(|output_id, output_mapping| {
                for (source, dests) in wf.function_class_specification.function_class_inner_structure.clone() {
                    if dests.contains(&edgeless_api::function_instance::MappingNode::Port(output_id.clone())) {
                        match source {
                            edgeless_api::function_instance::MappingNode::Port(port) => {
                                if active_inputs.contains(&port) {
                                    return true;
                                }
                                log::debug!("Not an Active Input");
                            }
                            edgeless_api::function_instance::MappingNode::SideEffect => {
                                return true;
                            }
                        }
                    } else {
                        log::debug!("Output not Mapped");
                    }
                }
                log::info!("Remove Unused Output: {}:{}", wf.name, output_id.0);
                changed = true;
                return false;
            });
        });
        changed
    }

    fn remove_unused_inputs(&mut self) -> bool {
        let active_outputs: std::collections::HashMap<_, _> = self
            .desired_state
            .workflow_functions
            .iter()
            .map(|wf| (wf.name.to_string(), self.active_outputs(&wf.name)))
            .collect();

        let mut inputs_to_be_removed = std::collections::HashSet::<(String, String)>::new();
        let mut changed = false;

        self.desired_state.workflow_functions.iter().for_each(|wf| {
            let active_outputs = active_outputs.get(&wf.name).unwrap();
            for (input_id, input_spec) in wf.function_class_specification.function_class_inputs.clone() {
                let full_id = edgeless_api::function_instance::MappingNode::Port(input_id.clone());
                'outer: for (source, dests) in &wf.function_class_specification.function_class_inner_structure.clone() {
                    // We only need to worry about removing casts as calls will always be usefull
                    if &full_id == source && input_spec.method == edgeless_api::function_instance::PortMethod::Cast {
                        if dests.contains(&edgeless_api::function_instance::MappingNode::SideEffect) {
                            continue;
                        } else {
                            for active_output in active_outputs {
                                if dests.contains(&edgeless_api::function_instance::MappingNode::Port(active_output.clone())) {
                                    continue 'outer;
                                }
                            }
                        }
                        inputs_to_be_removed.insert((wf.name.clone(), input_id.0.clone()));
                    }
                }
            }
        });

        self.desired_state.workflow_functions.iter_mut().for_each(|wf| {
            wf.output_mapping.retain(|_output_id, output_mapping| match output_mapping {
                edgeless_api::workflow_instance::PortMapping::DirectTarget(component, port) => {
                    if inputs_to_be_removed.contains(&(component.clone(), port.0.clone())) {
                        changed = true;
                        false
                    } else {
                        true
                    }
                }
                edgeless_api::workflow_instance::PortMapping::AnyOfTargets(targets) => {
                    let old_len = targets.len();
                    targets.retain(|(t_c, t_p)| !inputs_to_be_removed.contains(&(t_c.clone(), t_p.0.clone())));
                    if old_len != targets.len() {
                        log::error!("remove output: {:?}", targets);
                        changed = true;
                    }
                    targets.len() > 0
                }
                edgeless_api::workflow_instance::PortMapping::AllOfTargets(targets) => {
                    let old_len = targets.len();
                    targets.retain(|(t_c, t_p)| !inputs_to_be_removed.contains(&(t_c.clone(), t_p.0.clone())));
                    if old_len != targets.len() {
                        log::error!("remove output: {:?}", targets);
                        changed = true;
                    }
                    targets.len() > 0
                }
                edgeless_api::workflow_instance::PortMapping::Topic(_) => {
                    panic!("Topic Port shoud not exist anymore");
                }
            });
        });
        changed
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

        std::collections::HashMap::new()
    }

    pub fn inputs_for(&self, component_name: &str) -> std::collections::HashSet<String> {
        let mut inputing_functions = std::collections::HashSet::<String>::new();

        'each_fn: for f in &self.desired_state.workflow_functions {
            for (_, output) in &f.output_mapping {
                match output {
                    edgeless_api::workflow_instance::PortMapping::DirectTarget(target, _) => {
                        if target == component_name {
                            inputing_functions.insert(f.name.clone());
                            continue 'each_fn;
                        }
                    }
                    edgeless_api::workflow_instance::PortMapping::AnyOfTargets(targets) => {
                        for (target_actor, _) in targets {
                            if target_actor == component_name {
                                inputing_functions.insert(f.name.clone());
                                continue 'each_fn;
                            }
                        }
                    }
                    edgeless_api::workflow_instance::PortMapping::AllOfTargets(targets) => {
                        for (target_actor, _) in targets {
                            if target_actor == component_name {
                                inputing_functions.insert(f.name.clone());
                                continue 'each_fn;
                            }
                        }
                    }
                    edgeless_api::workflow_instance::PortMapping::Topic(_) => {
                        log::info!("Unused Topic!");
                    }
                }
            }
        }

        inputing_functions
    }
}

impl std::fmt::Display for ActiveComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.component_type {
            super::ComponentType::Function => write!(f, "function name {}, fid {}", self.name, self.fid),
            super::ComponentType::Resource => write!(f, "resource name {}, fid {}", self.name, self.fid),
        }
    }
}
