// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_api::function_instance::InstanceId;

pub trait WorkflowComponent {
    fn ports(&mut self) -> &mut ComponentPorts;
    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId>;
}

pub struct ActiveWorkflow {
    id: edgeless_api::workflow_instance::WorkflowId,
    original_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    functions: std::collections::HashMap<String, std::cell::RefCell<WorkflowFunction>>,
    resources: std::collections::HashMap<String, std::cell::RefCell<WorkflowResource>>,
    links: std::collections::HashMap<edgeless_api::link::LinkInstanceId, WorkflowLink>,
    subflows: std::collections::HashMap<edgeless_api::workflow_instance::WorkflowId, SubFlow>,
}

pub struct WorkflowFunction {
    pub image: ActorImage,
    pub annotations: std::collections::HashMap<String, String>,

    pub instances: Vec<std::cell::RefCell<WorkflowFunctionInstance>>,

    pub ports: ComponentPorts,
}

impl WorkflowComponent for WorkflowFunction {
    fn ports(&mut self) -> &mut ComponentPorts {
        &mut self.ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().map(|i| i.borrow().id.clone()).collect()
    }
}

pub struct WorkflowFunctionInstance {
    id: edgeless_api::function_instance::InstanceId,
    image: Option<ActorImage>,
    materialized: Option<MaterializedState>,
}

pub struct WorkflowResource {
    class: String,
    configurations: std::collections::HashMap<String, String>,

    pub instances: Vec<std::cell::RefCell<WorkflowResourceInstance>>,

    pub ports: ComponentPorts,
}

impl WorkflowComponent for WorkflowResource {
    fn ports(&mut self) -> &mut ComponentPorts {
        &mut self.ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().map(|i| i.borrow().id.clone()).collect()
    }
}

pub struct WorkflowResourceInstance {
    id: edgeless_api::function_instance::InstanceId,
    materialized: Option<MaterializedState>,
}

pub struct SubFlow {}

pub struct WorkflowLink {
    id: edgeless_api::link::LinkInstanceId,
    class: edgeless_api::link::LinkType,
    materialized: bool,
    nodes: Vec<(edgeless_api::function_instance::NodeId, edgeless_api::link::LinkProviderId, Vec<u8>, bool)>,
}

#[derive(Clone, Debug)]
pub struct ActorIdentifier {
    pub id: String,
    pub version: String,
}

#[derive(Clone, Debug)]
pub struct ActorClass {
    pub id: ActorIdentifier,
    pub inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::function_instance::Port>,
    pub outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, edgeless_api::function_instance::Port>,
    pub inner_structure: std::collections::HashMap<
        edgeless_api::function_instance::MappingNode,
        std::collections::HashSet<edgeless_api::function_instance::MappingNode>,
    >,
}

#[derive(Clone, Debug)]
pub struct ActorImage {
    pub class: ActorClass,
    pub format: String,
    pub enabled_inputs: std::collections::HashSet<edgeless_api::function_instance::PortId>,
    pub enabled_outputs: std::collections::HashSet<edgeless_api::function_instance::PortId>,
    pub code: Vec<u8>,
}

pub struct ComponentPorts {
    pub logical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalOutput>,
    pub logical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalInput>,
    pub physical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    pub physical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
}

pub struct MaterializedState {
    pub physical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    pub physical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
}

#[derive(Debug)]
pub enum RequiredChange {
    StartFunction {
        function_id: edgeless_api::function_instance::InstanceId,
        function_name: String,
        image: ActorImage,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
        annotations: std::collections::HashMap<String, String>,
    },
    StartResource {
        resource_id: edgeless_api::function_instance::InstanceId,
        resource_name: String,
        class_type: String,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
        configuration: std::collections::HashMap<String, String>,
    },
    PatchFunction {
        function_id: edgeless_api::function_instance::InstanceId,
        function_name: String,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
    PatchResource {
        resource_id: edgeless_api::function_instance::InstanceId,
        resource_name: String,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
    InstantiateLinkControlPlane {
        link_id: edgeless_api::link::LinkInstanceId,
        class: edgeless_api::link::LinkType,
    },
    CreateLinkOnNode {
        link_id: edgeless_api::link::LinkInstanceId,
        node_id: edgeless_api::function_instance::NodeId,
        provider_id: edgeless_api::link::LinkProviderId,
        config: Vec<u8>,
    },
    RemoveLinkFromNode {
        link_id: edgeless_api::link::LinkInstanceId,
        node_id: edgeless_api::function_instance::NodeId,
    },
}

impl WorkflowFunction {
    fn enabled_inputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.ports.logical_input_mapping.iter().map(|i| i.0.clone()).collect()
    }

    fn enabled_outputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.ports.logical_output_mapping.iter().map(|i| i.0.clone()).collect()
    }
}

#[derive(Clone)]
pub enum LogicalInput {
    Direct(Vec<(String, edgeless_api::function_instance::PortId)>),
    Topic(String),
}

pub type LogicalOutput = edgeless_api::workflow_instance::PortMapping;

pub type PhysicalOutput = edgeless_api::common::Output;
pub type PhysicalInput = edgeless_api::common::Input;

impl ActiveWorkflow {
    pub fn new(request: edgeless_api::workflow_instance::SpawnWorkflowRequest, id: edgeless_api::workflow_instance::WorkflowId) -> Self {
        if !request.annotations.is_empty() {
            log::warn!("Workflow annotations ({}) are currently ignored", request.annotations.len());
        }

        ActiveWorkflow {
            // state: WorkflowState::New,
            id: id,
            original_request: request.clone(),
            functions: request
                .workflow_functions
                .into_iter()
                .map(|function_req| {
                    (
                        function_req.name,
                        std::cell::RefCell::new(WorkflowFunction {
                            image: ActorImage {
                                enabled_inputs: function_req
                                    .function_class_specification
                                    .function_class_inputs
                                    .iter()
                                    .map(|(i, _)| i.clone())
                                    .collect(),
                                enabled_outputs: function_req
                                    .function_class_specification
                                    .function_class_outputs
                                    .iter()
                                    .map(|(o, _)| o.clone())
                                    .collect(),
                                class: ActorClass {
                                    id: ActorIdentifier {
                                        id: function_req.function_class_specification.function_class_id,
                                        version: function_req.function_class_specification.function_class_version,
                                    },
                                    inputs: function_req.function_class_specification.function_class_inputs,
                                    outputs: function_req.function_class_specification.function_class_outputs,
                                    inner_structure: function_req
                                        .function_class_specification
                                        .function_class_inner_structure
                                        .into_iter()
                                        .map(|(k, v)| (k, std::collections::HashSet::from_iter(v)))
                                        .collect(),
                                },
                                format: function_req.function_class_specification.function_class_type,
                                code: function_req.function_class_specification.function_class_code,
                            },
                            instances: Vec::new(),
                            annotations: function_req.annotations,
                            ports: ComponentPorts {
                                logical_input_mapping: function_req
                                    .input_mapping
                                    .into_iter()
                                    .map(|(port_id, port)| {
                                        (
                                            port_id,
                                            match port {
                                                edgeless_api::workflow_instance::PortMapping::DirectTarget(target_fid, target_port) => {
                                                    LogicalInput::Direct(vec![(target_fid, target_port)])
                                                }
                                                edgeless_api::workflow_instance::PortMapping::AnyOfTargets(targets) => LogicalInput::Direct(targets),
                                                edgeless_api::workflow_instance::PortMapping::AllOfTargets(targets) => LogicalInput::Direct(targets),
                                                edgeless_api::workflow_instance::PortMapping::Topic(topic) => LogicalInput::Topic(topic),
                                            },
                                        )
                                    })
                                    .collect(),
                                logical_output_mapping: function_req.output_mapping.clone(),
                                physical_input_mapping: std::collections::HashMap::new(),
                                physical_output_mapping: std::collections::HashMap::new(),
                            },
                        }),
                    )
                })
                .collect(),
            resources: request
                .workflow_resources
                .into_iter()
                .map(|resource_req| {
                    (
                        resource_req.name,
                        std::cell::RefCell::new(WorkflowResource {
                            class: resource_req.class_type,
                            configurations: resource_req.configurations,
                            instances: Vec::new(),
                            ports: ComponentPorts {
                                logical_input_mapping: resource_req
                                    .input_mapping
                                    .into_iter()
                                    .map(|(port_id, port)| {
                                        (
                                            port_id,
                                            match port {
                                                edgeless_api::workflow_instance::PortMapping::DirectTarget(target_fid, target_port) => {
                                                    LogicalInput::Direct(vec![(target_fid, target_port)])
                                                }
                                                edgeless_api::workflow_instance::PortMapping::AnyOfTargets(targets) => LogicalInput::Direct(targets),
                                                edgeless_api::workflow_instance::PortMapping::AllOfTargets(targets) => LogicalInput::Direct(targets),
                                                edgeless_api::workflow_instance::PortMapping::Topic(topic) => LogicalInput::Topic(topic),
                                            },
                                        )
                                    })
                                    .collect(),
                                logical_output_mapping: resource_req.output_mapping.clone(),
                                physical_input_mapping: std::collections::HashMap::new(),
                                physical_output_mapping: std::collections::HashMap::new(),
                            },
                        }),
                    )
                })
                .collect(),
            links: std::collections::HashMap::new(),
            subflows: std::collections::HashMap::new(),
        }
    }

    pub fn initial_spawn(
        &mut self,
        orchestration_logic: &mut crate::orchestration_logic::OrchestrationLogic,
        nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
        link_controllers: &mut std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>,
    ) -> Vec<RequiredChange> {
        self.transform_logical();
        self.place(orchestration_logic, nodes);
        self.transform_physical();
        self.generate_input_mapping();
        self.create_links(nodes, link_controllers);
        self.materialize()
    }

    pub fn node_removal(
        &mut self,
        removed_node_ids: &std::collections::HashSet<edgeless_api::function_instance::NodeId>,
        orchestration_logic: &mut crate::orchestration_logic::OrchestrationLogic,
        nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
        link_controllers: &mut std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>,
    ) -> Vec<RequiredChange> {
        if self.remove_nodes(removed_node_ids) {
            self.place(orchestration_logic, nodes);
            self.transform_physical();
            self.generate_input_mapping();
            self.create_links(nodes, link_controllers);
            self.materialize()
        } else {
            Vec::new()
        }
    }

    pub fn stop(&mut self) -> Vec<RequiredChange> {
        // TODO
        Vec::new()
    }

    fn transform_logical(&mut self) {
        self.convert_topic_ports();
        self.add_input_backlinks();
        self.remove_unused_links();
    }

    fn place(
        &mut self,
        orchestration_logic: &mut crate::orchestration_logic::OrchestrationLogic,
        nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
    ) {
        for (f_id, function) in &mut self.functions {
            let mut function = function.borrow_mut();
            if function.instances.is_empty() {
                let dst = orchestration_logic.next(nodes, &function.image.format, &function.annotations);

                if let Some(dst) = dst {
                    function.instances.push(std::cell::RefCell::new(WorkflowFunctionInstance {
                        id: edgeless_api::function_instance::InstanceId::new(dst),
                        image: None,
                        materialized: None,
                    }))
                } else {
                    log::info!("Found no viable node for {} in {}", &f_id, self.id.workflow_id);
                }
            }
        }

        for (_, resource) in &mut self.resources {
            let mut resource = resource.borrow_mut();
            if resource.instances.is_empty() {
                let dst = Self::select_node_for_resource(&resource, nodes);
                if let Some(dst) = dst {
                    resource.instances.push(std::cell::RefCell::new(WorkflowResourceInstance {
                        id: edgeless_api::function_instance::InstanceId::new(dst),
                        materialized: None,
                    }));
                }
            }
        }
    }

    fn generate_input_mapping(&mut self) {
        let components = self
            .components()
            .into_iter()
            .map(|(id, spec)| (id.clone(), spec.borrow_mut().instance_ids()))
            .collect::<std::collections::HashMap<String, Vec<InstanceId>>>();

        for (component_id, _) in &components {
            let mut component = self.get_component(&component_id).unwrap().borrow_mut();
            let ports = component.ports();

            for (output_id, output) in &ports.logical_output_mapping {
                match output {
                    LogicalOutput::DirectTarget(target_component, target_port_id) => {
                        let mut instances = components.get(target_component).unwrap().clone();
                        if let Some(id) = instances.pop() {
                            ports
                                .physical_output_mapping
                                .insert(output_id.clone(), PhysicalOutput::Single(id, target_port_id.clone()));
                        }
                    }
                    LogicalOutput::AnyOfTargets(targets) => {
                        let mut instances = Vec::new();
                        for (target_id, port_id) in targets {
                            instances.append(
                                &mut components
                                    .get(target_id)
                                    .unwrap()
                                    .iter()
                                    .map(|target| (target.clone(), port_id.clone()))
                                    .collect(),
                            )
                        }
                        ports.physical_output_mapping.insert(output_id.clone(), PhysicalOutput::Any(instances));
                    }
                    LogicalOutput::AllOfTargets(targets) => {
                        let mut instances = Vec::new();
                        for (target_id, port_id) in targets {
                            instances.append(
                                &mut components
                                    .get(target_id)
                                    .unwrap()
                                    .iter()
                                    .map(|target| (target.clone(), port_id.clone()))
                                    .collect(),
                            )
                        }
                        ports.physical_output_mapping.insert(output_id.clone(), PhysicalOutput::All(instances));
                    }
                    LogicalOutput::Topic(_) => {}
                }
            }
        }
    }

    fn transform_physical(&mut self) {
        self.compile_rust();
    }

    fn materialize(&mut self) -> Vec<RequiredChange> {
        let mut changes = Vec::new();

        for (link_id, link) in &mut self.links {
            if !link.materialized {
                changes.push(RequiredChange::InstantiateLinkControlPlane {
                    link_id: link_id.clone(),
                    class: link.class.clone(),
                });
            }

            for (node, link_provider_id, node_config, node_materialized) in &link.nodes {
                if !node_materialized {
                    changes.push(RequiredChange::CreateLinkOnNode {
                        node_id: node.clone(),
                        provider_id: link_provider_id.clone(),
                        link_id: link_id.clone(),
                        config: node_config.clone(),
                    });
                }
            }
        }

        for (f_name, function) in &self.functions {
            let function = function.borrow_mut();
            for i in function.instances.iter() {
                let mut current = i.borrow_mut();
                if let Some(materialized) = &current.materialized {
                    if materialized.physical_input_mapping != function.ports.physical_input_mapping
                        || materialized.physical_output_mapping != function.ports.physical_output_mapping
                    {
                        changes.push(RequiredChange::PatchFunction {
                            function_id: current.id.clone(),
                            function_name: f_name.clone(),
                            input_mapping: function.ports.physical_input_mapping.clone(),
                            output_mapping: function.ports.physical_output_mapping.clone(),
                        });
                    }
                } else {
                    changes.push(RequiredChange::StartFunction {
                        function_id: current.id.clone(),
                        function_name: f_name.clone(),
                        image: if let Some(custom_image) = &current.image {
                            custom_image.clone()
                        } else {
                            function.image.clone()
                        },
                        input_mapping: function.ports.physical_input_mapping.clone(),
                        output_mapping: function.ports.physical_output_mapping.clone(),
                        annotations: function.annotations.clone(),
                    });
                    current.materialized = Some(MaterializedState {
                        physical_input_mapping: function.ports.physical_input_mapping.clone(),
                        physical_output_mapping: function.ports.physical_output_mapping.clone(),
                    });
                }
            }
        }

        for (r_name, resource) in &mut self.resources {
            let mut resource = resource.borrow_mut();
            for i in &resource.instances {
                let mut current = i.borrow_mut();
                if let Some(materialized) = &current.materialized {
                    if materialized.physical_input_mapping != resource.ports.physical_input_mapping
                        || materialized.physical_output_mapping != resource.ports.physical_output_mapping
                    {
                        changes.push(RequiredChange::PatchResource {
                            resource_id: current.id.clone(),
                            resource_name: r_name.clone(),
                            input_mapping: resource.ports.physical_input_mapping.clone(),
                            output_mapping: resource.ports.physical_output_mapping.clone(),
                        });
                    }
                } else {
                    changes.push(RequiredChange::StartResource {
                        resource_id: current.id.clone(),
                        resource_name: r_name.clone(),
                        class_type: resource.class.clone(),
                        output_mapping: resource.ports.physical_output_mapping.clone(),
                        input_mapping: resource.ports.physical_input_mapping.clone(),
                        configuration: resource.configurations.clone(),
                    });
                    current.materialized = Some(MaterializedState {
                        physical_input_mapping: resource.ports.physical_input_mapping.clone(),
                        physical_output_mapping: resource.ports.physical_output_mapping.clone(),
                    });
                }
            }
        }

        changes
    }

    fn remove_nodes(&mut self, node_ids: &std::collections::HashSet<edgeless_api::function_instance::NodeId>) -> bool {
        let mut changed = false;
        for (_, function) in &mut self.functions {
            let mut function = function.borrow_mut();
            let before = function.instances.len();
            function
                .instances
                .retain(|instance| !node_ids.contains(&instance.borrow_mut().id.node_id));
            if before != function.instances.len() {
                changed = true;
            }
        }
        for (_, resource) in &mut self.resources {
            let mut resource = resource.borrow_mut();
            let before = resource.instances.len();
            resource
                .instances
                .retain(|instance| !node_ids.contains(&instance.borrow_mut().id.node_id));
            if before != resource.instances.len() {
                changed = true;
            }
        }
        changed
    }

    fn add_input_backlinks(&mut self) {
        let mut inputs = std::collections::HashMap::<
            String,
            std::collections::HashMap<edgeless_api::function_instance::PortId, Vec<(String, edgeless_api::function_instance::PortId)>>,
        >::new();

        for (out_cid, fdesc) in self.components() {
            for (out_port, mapping) in &fdesc.borrow_mut().ports().logical_output_mapping {
                match mapping {
                    LogicalOutput::DirectTarget(target_fid, target_port) => inputs
                        .entry(target_fid.clone())
                        .or_default()
                        .entry(target_port.clone())
                        .or_default()
                        .push((out_cid.clone(), out_port.clone())),
                    LogicalOutput::AnyOfTargets(targets) => {
                        for (target_fid, target_port) in targets {
                            inputs
                                .entry(target_fid.clone())
                                .or_default()
                                .entry(target_port.clone())
                                .or_default()
                                .push((out_cid.clone(), out_port.clone()))
                        }
                    }
                    LogicalOutput::AllOfTargets(targets) => {
                        for (target_fid, target_port) in targets {
                            inputs
                                .entry(target_fid.clone())
                                .or_default()
                                .entry(target_port.clone())
                                .or_default()
                                .push((out_cid.clone(), out_port.clone()))
                        }
                    }
                    LogicalOutput::Topic(_) => {}
                }
            }
        }

        for (targed_fid, links) in &inputs {
            if let Some(target) = self.functions.get_mut(targed_fid) {
                for (target_port, sources) in links {
                    target
                        .borrow_mut()
                        .ports
                        .logical_input_mapping
                        .insert(target_port.clone(), LogicalInput::Direct(sources.clone()));
                }
            } else if let Some(target) = self.resources.get_mut(targed_fid) {
                // Some(&mut target.borrow_mut().ports)
                for (target_port, sources) in links {
                    target
                        .borrow_mut()
                        .ports
                        .logical_input_mapping
                        .insert(target_port.clone(), LogicalInput::Direct(sources.clone()));
                }
            }
        }
    }

    fn convert_topic_ports(&mut self) {
        let mut targets = std::collections::HashMap::<String, Vec<(String, edgeless_api::function_instance::PortId)>>::new();

        // Find Targets
        for (cid, component) in &mut self.components() {
            component
                .borrow_mut()
                .ports()
                .logical_input_mapping
                .retain(|port_id, port_mapping| match port_mapping {
                    LogicalInput::Topic(topic) => {
                        targets.entry(topic.clone()).or_insert(Vec::new()).push((cid.clone(), port_id.clone()));
                        false
                    }
                    _ => true,
                })
        }

        // Create Outputs

        for (cid, component) in &mut self.components() {
            component
                .borrow_mut()
                .ports()
                .logical_output_mapping
                .iter_mut()
                .for_each(|(_port_id, port_mapping)| {
                    if let LogicalOutput::Topic(topic) = port_mapping.clone() {
                        *port_mapping = LogicalOutput::AllOfTargets(
                            targets
                                .get(&topic)
                                .unwrap_or(&Vec::<(String, edgeless_api::function_instance::PortId)>::new())
                                .clone(),
                        );
                    }
                });
        }
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

        let mut input_links_to_remove = Vec::new();

        for (f_id, f) in &mut self.functions {
            let mut f = f.borrow_mut();
            let inner: std::collections::HashMap<
                edgeless_api::function_instance::MappingNode,
                std::collections::HashSet<edgeless_api::function_instance::MappingNode>,
            > = f.image.class.inner_structure.clone();
            let ports = &mut f.ports;
            ports.logical_output_mapping.retain(|output_id, output_spec: &mut LogicalOutput| {
                assert!(!std::matches!(output_spec, LogicalOutput::Topic(_)));
                let this = edgeless_api::function_instance::MappingNode::Port(output_id.clone());
                for (src, dests) in &inner {
                    if dests.contains(&this) {
                        match src {
                            edgeless_api::function_instance::MappingNode::Port(port) => {
                                if ports.logical_input_mapping.contains_key(&port) {
                                    return true;
                                }
                                log::debug!("Not an Active Input");
                            }
                            edgeless_api::function_instance::MappingNode::SideEffect => {
                                return true;
                            }
                        }
                    }
                }

                let mut to_remove = match output_spec {
                    LogicalOutput::DirectTarget(target_node_id, target_port_id) => {
                        vec![((target_node_id.clone(), target_port_id.clone()), (f_id.clone(), output_id.clone()))]
                    }
                    LogicalOutput::AnyOfTargets(targets) => targets
                        .iter()
                        .map(|(target_node_id, target_port_id)| {
                            (((target_node_id.clone(), target_port_id.clone()), (f_id.clone(), output_id.clone())))
                        })
                        .collect(),
                    LogicalOutput::AllOfTargets(targets) => targets
                        .iter()
                        .map(|(target_node_id, target_port_id)| {
                            (((target_node_id.clone(), target_port_id.clone()), (f_id.clone(), output_id.clone())))
                        })
                        .collect(),
                    LogicalOutput::Topic(_) => vec![],
                };
                input_links_to_remove.append(&mut to_remove);
                changed = true;
                false
            });
        }

        for ((target_component_id, target_port_id), (source_component_id, source_port_id)) in &input_links_to_remove {
            if let Some(source) = self.functions.get_mut(target_component_id) {
                let mut source = source.borrow_mut();
                let mut remove = false;
                if let Some(source_port) = source.ports.logical_input_mapping.get_mut(target_port_id) {
                    if let LogicalInput::Direct(sources) = source_port {
                        sources.retain(|(s_id, s_p_id)| s_id != source_component_id && s_p_id != source_port_id);
                        if sources.len() == 0 {
                            remove = true;
                        }
                    }
                }
                if remove {
                    source.ports.logical_input_mapping.remove(target_port_id);
                }
            }
        }

        changed
    }

    fn remove_unused_inputs(&mut self) -> bool {
        let mut changed = false;

        let mut output_links_to_remove = Vec::new();

        for (f_id, f) in &mut self.functions {
            let mut f = f.borrow_mut();
            let class = f.image.class.clone();
            let f_ports = &mut f.ports;
            f_ports.logical_input_mapping.retain(|input_id, input_spec| {
                if let LogicalInput::Direct(mapped_inputs) = input_spec {
                    let port_method = class.inputs.get(input_id).unwrap().method.clone();
                    // We only need to worry about removing casts as calls will always be usefull
                    if port_method == edgeless_api::function_instance::PortMethod::Cast {
                        let inner_for_this = class
                            .inner_structure
                            .get(&edgeless_api::function_instance::MappingNode::Port(input_id.clone()));
                        if let Some(inner_targets) = inner_for_this {
                            if inner_targets.contains(&edgeless_api::function_instance::MappingNode::SideEffect) {
                                return true;
                            } else {
                                for output in f_ports.logical_output_mapping.keys() {
                                    if inner_targets.contains(&edgeless_api::function_instance::MappingNode::Port(output.clone())) {
                                        return true;
                                    }
                                }
                            }
                        }

                        output_links_to_remove.append(
                            &mut mapped_inputs
                                .iter()
                                .map(|(o_comp, o_port)| ((o_comp.clone(), o_port.clone()), (f_id.clone(), input_id.clone())))
                                .collect(),
                        );
                        changed = true;
                        return false;
                    } else {
                        return true;
                    }
                } else {
                    return true;
                }
            });
        }

        for ((source_id, source_port_id), (dest_id, dest_port_id)) in &output_links_to_remove {
            if let Some(source) = self.functions.get_mut(source_id) {
                let mut source = source.borrow_mut();
                let mut remove = false;
                if let Some(source_port) = source.ports.logical_output_mapping.get_mut(source_port_id) {
                    match source_port {
                        LogicalOutput::DirectTarget(target_id, target_port_id) => {
                            if target_id == dest_id && target_port_id == dest_port_id {
                                remove = true;
                            }
                        }
                        LogicalOutput::AnyOfTargets(targets) => {
                            targets.retain(|(target_id, target_port_id)| !(target_id == dest_id && target_port_id == dest_port_id));
                            if targets.len() == 0 {
                                remove = true;
                            }
                        }
                        LogicalOutput::AllOfTargets(targets) => {
                            targets.retain(|(target_id, target_port_id)| !(target_id == dest_id && target_port_id == dest_port_id));
                            if targets.len() == 0 {
                                remove = true;
                            }
                        }
                        LogicalOutput::Topic(_) => {}
                    }
                }
                if remove {
                    source.ports.logical_output_mapping.remove(source_port_id);
                }
            }
        }

        changed
    }

    fn create_links(
        &mut self,
        nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
        link_controllers: &mut std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>,
    ) {
        let mcast = edgeless_api::link::LinkType("MULTICAST".to_string());

        let mut new_links = Vec::<(edgeless_api::link::LinkInstanceId, WorkflowLink)>::new();

        for (c_id, c) in self.components() {
            let mut current = c.borrow_mut();
            let current_ports = current.ports();
            for (out_id, out) in &mut current_ports.physical_output_mapping {
                match out {
                    edgeless_api::common::Output::All(targets) => {
                        if targets.len() >= 2 {
                            let target_nodes: std::collections::HashSet<_> = targets.iter().map(|(t_id, _)| t_id.node_id.clone()).collect();
                            let new_link = link_controllers
                                .get_mut(&mcast)
                                .unwrap()
                                .new_link(target_nodes.clone().into_iter().collect())
                                .unwrap();

                            let node_links: Vec<_> = target_nodes
                                .iter()
                                .map(|n| {
                                    (
                                        n.clone(),
                                        nodes.get(n).unwrap().supported_link_types.get(&mcast).unwrap().clone(),
                                        link_controllers.get(&mcast).unwrap().config_for(new_link.clone(), n.clone()).unwrap(),
                                        false,
                                    )
                                })
                                .collect();

                            new_links.push((
                                new_link.clone(),
                                WorkflowLink {
                                    id: new_link.clone(),
                                    class: mcast.clone(),
                                    materialized: false,
                                    nodes: node_links,
                                },
                            ));
                            *out = PhysicalOutput::Link(new_link.clone());

                            let logical_port = current_ports.logical_output_mapping.get(out_id).unwrap();
                            if let edgeless_api::workflow_instance::PortMapping::AllOfTargets(logical_targets) = logical_port {
                                for (target_name, target_port_id) in logical_targets {
                                    self.get_component(&target_name)
                                        .unwrap()
                                        .borrow_mut()
                                        .ports()
                                        .physical_input_mapping
                                        .insert(target_port_id.clone(), PhysicalInput::Link(new_link.clone()));
                                }
                            } else {
                                panic!("Mapping is Wrong!");
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        for (id, spec) in new_links {
            self.links.insert(id, spec);
        }
    }

    fn compile_rust(&mut self) {
        for (_, function) in &mut self.functions {
            let function = function.borrow_mut();
            if function.image.format == "RUST" {
                let enabled_inputs = function.enabled_inputs();
                let enabled_outputs = function.enabled_outputs();

                let mut enabled_features: Vec<String> = Vec::new();
                for input in &enabled_inputs {
                    enabled_features.push(format!("input_{}", input.0))
                }
                for output in &enabled_outputs {
                    enabled_features.push(format!("output_{}", output.0))
                }

                // panic!("{:?}", enabled_features);

                let rust_dir = edgeless_build::unpack_rust_package(&function.image.code).unwrap();
                let wasm_file = edgeless_build::rust_to_wasm(rust_dir, enabled_features, true, false).unwrap();
                let wasm_code = std::fs::read(wasm_file).unwrap();

                for instance in &function.instances {
                    instance.borrow_mut().image = Some(ActorImage {
                        class: function.image.class.clone(),
                        format: "RUST_WASM".to_string(),
                        enabled_inputs: enabled_inputs.iter().map(|i| i.clone()).collect(),
                        enabled_outputs: enabled_outputs.iter().map(|o| o.clone()).collect(),
                        code: wasm_code.clone(),
                    })
                }
            }
        }
    }

    fn components(&self) -> Vec<(&String, &std::cell::RefCell<dyn WorkflowComponent>)> {
        let mut components = self
            .functions
            .iter()
            .map(|(f_id, f)| (f_id, f as &std::cell::RefCell<dyn WorkflowComponent>))
            .collect::<Vec<_>>();
        components.append(
            &mut self
                .resources
                .iter()
                .map(|(r_id, r)| (r_id, r as &std::cell::RefCell<dyn WorkflowComponent>))
                .collect::<Vec<_>>(),
        );
        components
    }

    fn instances_for_component(&mut self, component_name: &str) -> Option<Vec<edgeless_api::function_instance::InstanceId>> {
        if let Some(component) = self.get_component(component_name) {
            Some(component.borrow_mut().instance_ids())
        } else {
            None
        }
    }

    fn get_component(&self, component_name: &str) -> Option<&std::cell::RefCell<dyn WorkflowComponent>> {
        if let Some(component) = self.functions.get(component_name) {
            return Some(component as &std::cell::RefCell<dyn WorkflowComponent>);
        } else if let Some(compoenent) = self.resources.get(component_name) {
            return Some(compoenent as &std::cell::RefCell<dyn WorkflowComponent>);
        } else {
            return None;
        }
    }

    fn select_node_for_resource(
        resource: &WorkflowResource,
        nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
    ) -> Option<edgeless_api::function_instance::NodeId> {
        if let Some((id, _)) = nodes
            .iter()
            .find(|(_, n)| n.resource_providers.iter().find(|(_, r)| r.class_type == resource.class).is_some())
        {
            Some(id.clone())
        } else {
            None
        }
    }
}
