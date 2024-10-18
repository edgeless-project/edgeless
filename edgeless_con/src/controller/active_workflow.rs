// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_api::function_instance::InstanceId;

pub trait WorkflowComponent {
    fn logical_ports(&mut self) -> &mut LogicalPorts;
    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId>;
    fn instances(&mut self) -> Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>>;
    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>>);
}

pub trait WorkflowComponentInstance {
    fn physical_ports(&mut self) -> &mut PhysicalPorts;
}

pub struct ActiveWorkflow {
    id: edgeless_api::workflow_instance::WorkflowId,

    original_request: edgeless_api::workflow_instance::SpawnWorkflowRequest,

    functions: std::collections::HashMap<String, std::cell::RefCell<WorkflowFunction>>,
    resources: std::collections::HashMap<String, std::cell::RefCell<WorkflowResource>>,
    subflows: std::collections::HashMap<String, std::cell::RefCell<SubFlow>>,
    proxy: std::cell::RefCell<WorkflowProxy>,

    links: std::collections::HashMap<edgeless_api::link::LinkInstanceId, WorkflowLink>,
}

pub struct WorkflowFunction {
    pub image: ActorImage,
    pub annotations: std::collections::HashMap<String, String>,

    pub logical_ports: LogicalPorts,

    pub instances: Vec<std::cell::RefCell<WorkflowFunctionInstance>>,
}

impl WorkflowComponent for WorkflowFunction {
    fn logical_ports(&mut self) -> &mut LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().map(|i| i.borrow().id.clone()).collect()
    }

    fn instances(&mut self) -> Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>> {
        self.instances
            .iter()
            .map(|i| i as &std::cell::RefCell<dyn WorkflowComponentInstance>)
            .collect()
    }

    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>>) {
        (
            &mut self.logical_ports,
            self.instances
                .iter()
                .map(|i| i as &std::cell::RefCell<dyn WorkflowComponentInstance>)
                .collect(),
        )
    }
}

pub struct WorkflowFunctionInstance {
    id: edgeless_api::function_instance::InstanceId,
    image: Option<ActorImage>,
    desired_mapping: PhysicalPorts,
    materialized: Option<PhysicalPorts>,
}

impl WorkflowComponentInstance for WorkflowFunctionInstance {
    fn physical_ports(&mut self) -> &mut PhysicalPorts {
        &mut self.desired_mapping
    }
}

pub struct WorkflowResource {
    class: String,
    configurations: std::collections::HashMap<String, String>,

    pub instances: Vec<std::cell::RefCell<WorkflowResourceInstance>>,

    pub logical_ports: LogicalPorts,
}

impl WorkflowComponent for WorkflowResource {
    fn logical_ports(&mut self) -> &mut LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().map(|i| i.borrow().id.clone()).collect()
    }

    fn instances(&mut self) -> Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>> {
        self.instances
            .iter()
            .map(|i| i as &std::cell::RefCell<dyn WorkflowComponentInstance>)
            .collect()
    }

    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>>) {
        (
            &mut self.logical_ports,
            self.instances
                .iter()
                .map(|i| i as &std::cell::RefCell<dyn WorkflowComponentInstance>)
                .collect(),
        )
    }
}

pub struct WorkflowResourceInstance {
    id: edgeless_api::function_instance::InstanceId,
    desired_mapping: PhysicalPorts,
    materialized: Option<PhysicalPorts>,
}

impl WorkflowComponentInstance for WorkflowResourceInstance {
    fn physical_ports(&mut self) -> &mut PhysicalPorts {
        &mut self.desired_mapping
    }
}

pub struct WorkflowProxy {
    pub logical_ports: LogicalPorts,

    pub external_ports: ExternalPorts,

    pub instances: Vec<std::cell::RefCell<WorkflowProxyInstance>>,
}

impl WorkflowComponent for WorkflowProxy {
    fn logical_ports(&mut self) -> &mut LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().map(|i| i.borrow().id.clone()).collect()
    }

    fn instances(&mut self) -> Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>> {
        self.instances
            .iter()
            .map(|i| i as &std::cell::RefCell<dyn WorkflowComponentInstance>)
            .collect()
    }

    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>>) {
        (
            &mut self.logical_ports,
            self.instances
                .iter()
                .map(|i| i as &std::cell::RefCell<dyn WorkflowComponentInstance>)
                .collect(),
        )
    }
}

// pub struct WorkflowEgressProxy {
//     pub id: edgeless_api::function_instance::ComponentId,
//     pub instances: Vec<std::cell::RefCell<WorkflowProxyInstance>>
// }

pub struct WorkflowProxyInstance {
    id: edgeless_api::function_instance::InstanceId,
    desired_mapping: PhysicalPorts,
    materialized: Option<PhysicalPorts>,
}

impl WorkflowComponentInstance for WorkflowProxyInstance {
    fn physical_ports(&mut self) -> &mut PhysicalPorts {
        &mut self.desired_mapping
    }
}

pub struct SubFlow {
    functions: std::collections::HashMap<String, SubFlowFunction>,
    resources: std::collections::HashMap<String, SubFlowResource>,

    logical_ports: LogicalPorts,

    internal_ports: InternalPorts,

    instances: Vec<std::cell::RefCell<SubFlowInstance>>,

    annotations: std::collections::HashMap<String, String>,
}

impl WorkflowComponent for SubFlow {
    fn logical_ports(&mut self) -> &mut LogicalPorts {
        &mut self.logical_ports
    }

    fn instance_ids(&mut self) -> Vec<edgeless_api::function_instance::InstanceId> {
        self.instances.iter().map(|i| i.borrow().id.clone()).collect()
    }

    fn instances(&mut self) -> Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>> {
        self.instances
            .iter()
            .map(|i| i as &std::cell::RefCell<dyn WorkflowComponentInstance>)
            .collect()
    }

    fn split_view(&mut self) -> (&mut LogicalPorts, Vec<&std::cell::RefCell<dyn WorkflowComponentInstance>>) {
        (
            &mut self.logical_ports,
            self.instances
                .iter()
                .map(|i| i as &std::cell::RefCell<dyn WorkflowComponentInstance>)
                .collect(),
        )
    }
}

pub struct SubFlowInstance {
    id: edgeless_api::function_instance::InstanceId,
    desired_mapping: PhysicalPorts,
    materialized: Option<PhysicalPorts>,
}

impl WorkflowComponentInstance for SubFlowInstance {
    fn physical_ports(&mut self) -> &mut PhysicalPorts {
        &mut self.desired_mapping
    }
}

pub struct SubFlowFunction {}

pub struct SubFlowResource {}

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

#[derive(Default)]
pub struct LogicalPorts {
    pub logical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalOutput>,
    pub logical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalInput>,
}

#[derive(Default)]
pub struct PhysicalPorts {
    pub physical_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    pub physical_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
}

#[derive(Default)]
pub struct ExternalPorts {
    pub external_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
    pub external_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
}

pub struct InternalPorts {
    pub internal_input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalOutput>,
    pub internal_output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, LogicalInput>,
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
    CreateSubflow {
        subflow_id: edgeless_api::function_instance::InstanceId,
        spawn_req: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    },
    PatchSubflow {
        subflow_id: edgeless_api::function_instance::InstanceId,
        input_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        output_mapping: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
    PatchProxy {
        proxy_id: edgeless_api::function_instance::InstanceId,
        internal_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        internal_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
        external_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        external_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
    CrateProxy {
        proxy_id: edgeless_api::function_instance::InstanceId,
        internal_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        internal_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
        external_inputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalInput>,
        external_outputs: std::collections::HashMap<edgeless_api::function_instance::PortId, PhysicalOutput>,
    },
}

impl WorkflowFunction {
    fn enabled_inputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.logical_ports.logical_input_mapping.iter().map(|i| i.0.clone()).collect()
    }

    fn enabled_outputs(&self) -> Vec<edgeless_api::function_instance::PortId> {
        self.logical_ports.logical_output_mapping.iter().map(|i| i.0.clone()).collect()
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
                            logical_ports: LogicalPorts {
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
                            logical_ports: LogicalPorts {
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
                            },
                        }),
                    )
                })
                .collect(),
            links: std::collections::HashMap::new(),
            subflows: std::collections::HashMap::new(),
            proxy: std::cell::RefCell::new(WorkflowProxy {
                logical_ports: LogicalPorts {
                    logical_output_mapping: request
                        .workflow_ingress_proxies
                        .iter()
                        .map(|i| (edgeless_api::function_instance::PortId(i.id.clone()), i.inner_output.clone()))
                        .collect(),
                    logical_input_mapping: request
                        .workflow_egress_proxies
                        .iter()
                        .filter_map(|e| match &e.inner_input {
                            edgeless_api::workflow_instance::PortMapping::Topic(t) => {
                                Some((edgeless_api::function_instance::PortId(e.id.clone()), LogicalInput::Topic(t.clone())))
                            }
                            _ => None,
                        })
                        .collect(),
                },
                external_ports: ExternalPorts {
                    external_input_mapping: request
                        .workflow_ingress_proxies
                        .iter()
                        .map(|i| (edgeless_api::function_instance::PortId(i.id.clone()), i.external_input.clone()))
                        .collect(),
                    external_output_mapping: request
                        .workflow_egress_proxies
                        .iter()
                        .map(|i| (edgeless_api::function_instance::PortId(i.id.clone()), i.external_output.clone()))
                        .collect(),
                },
                instances: Vec::new(),
            }),
        }
    }

    pub fn initial_spawn(
        &mut self,
        orchestration_logic: &mut crate::orchestration_logic::OrchestrationLogic,
        nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
        link_controllers: &mut std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>,
        peer_clusters: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>,
    ) -> Vec<RequiredChange> {
        self.transform_logical();
        self.place(orchestration_logic, nodes, peer_clusters);
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
        peer_clusters: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>,
        link_controllers: &mut std::collections::HashMap<edgeless_api::link::LinkType, Box<dyn edgeless_api::link::LinkController>>,
    ) -> Vec<RequiredChange> {
        if self.remove_nodes(removed_node_ids) {
            self.place(orchestration_logic, nodes, peer_clusters);
            self.transform_physical();
            self.generate_input_mapping();
            self.create_links(nodes, link_controllers);
            self.materialize()
        } else {
            Vec::new()
        }
    }

    pub fn patch_external_links(&mut self, update: edgeless_api::common::PatchRequest) -> Vec<RequiredChange> {
        {
            let mut prx = self.proxy.borrow_mut();
            prx.external_ports.external_input_mapping = update.input_mapping;
            prx.external_ports.external_output_mapping = update.output_mapping;
        }
        self.materialize()
    }

    pub fn peer_cluster_removal(&self, removed_cluster_ids: edgeless_api::function_instance::NodeId) -> Vec<RequiredChange> {
        Vec::new()
    }

    pub fn stop(&mut self) -> Vec<RequiredChange> {
        // TODO
        Vec::new()
    }

    fn transform_logical(&mut self) {
        self.convert_topic_ports();
        self.add_input_backlinks();
        self.split_out_subflows();
        self.remove_unused_links();
    }

    fn place(
        &mut self,
        orchestration_logic: &mut crate::orchestration_logic::OrchestrationLogic,
        nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
        peer_clusters: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>,
    ) {
        for (f_id, function) in &mut self.functions {
            let mut function = function.borrow_mut();
            if function.instances.is_empty() {
                let dst = orchestration_logic.next(nodes, &function.image.format, &function.annotations);

                if let Some(dst) = dst {
                    function.instances.push(std::cell::RefCell::new(WorkflowFunctionInstance {
                        id: edgeless_api::function_instance::InstanceId::new(dst),
                        desired_mapping: PhysicalPorts::default(),
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
                        desired_mapping: PhysicalPorts::default(),
                        materialized: None,
                    }));
                }
            }
        }

        for (_, subflow) in &mut self.subflows {
            let mut subflow = subflow.borrow_mut();
            if subflow.instances.is_empty() {
                if subflow.instances.is_empty() {
                    let dst = Self::select_cluster_for_subflow(&subflow, peer_clusters);
                    if let Some(dst) = dst {
                        subflow.instances.push(std::cell::RefCell::new(SubFlowInstance {
                            id: edgeless_api::function_instance::InstanceId::new(dst),
                            desired_mapping: PhysicalPorts::default(),
                            materialized: None,
                        }))
                    }
                }
            }
        }

        {
            let mut proxy = self.proxy.borrow_mut();
            if !proxy.logical_ports.logical_input_mapping.is_empty() || !proxy.logical_ports.logical_output_mapping.is_empty() {
                if proxy.instances.is_empty() {
                    let dst = Self::select_node_for_proxy(&proxy, nodes);
                    if let Some(dst) = dst {
                        proxy.instances.push(std::cell::RefCell::new(WorkflowProxyInstance {
                            id: edgeless_api::function_instance::InstanceId::new(dst),
                            desired_mapping: PhysicalPorts::default(),
                            materialized: None,
                        }));
                    }
                }
            }
        }
    }

    fn split_out_subflows(&mut self) {
        // TODO
        // for (f_name, f) in &self.functions {
        //     if f.borrow_mut().annotations.get("")
        // }
    }

    fn generate_input_mapping(&mut self) {
        let components = self
            .components()
            .into_iter()
            .map(|(id, spec)| (id.to_string(), spec.borrow_mut().instance_ids()))
            .collect::<std::collections::HashMap<String, Vec<InstanceId>>>();

        for (component_id, _) in &components {
            let mut component = self.get_component(&component_id).unwrap().borrow_mut();
            let (logical_ports, physical_instances) = component.split_view();

            for (output_id, output) in &logical_ports.logical_output_mapping {
                match output {
                    LogicalOutput::DirectTarget(target_component, target_port_id) => {
                        let mut instances = components.get(target_component).unwrap().clone();
                        if let Some(id) = instances.pop() {
                            for c_instance in &physical_instances {
                                c_instance
                                    .borrow_mut()
                                    .physical_ports()
                                    .physical_output_mapping
                                    .insert(output_id.clone(), PhysicalOutput::Single(id, target_port_id.clone()));
                            }
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
                        for c_instance in &physical_instances {
                            c_instance
                                .borrow_mut()
                                .physical_ports()
                                .physical_output_mapping
                                .insert(output_id.clone(), PhysicalOutput::Any(instances.clone()));
                        }
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
                        for c_instance in &physical_instances {
                            c_instance
                                .borrow_mut()
                                .physical_ports()
                                .physical_output_mapping
                                .insert(output_id.clone(), PhysicalOutput::All(instances.clone()));
                        }
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
                    if materialized.physical_input_mapping != current.desired_mapping.physical_input_mapping
                        || materialized.physical_output_mapping != current.desired_mapping.physical_output_mapping
                    {
                        changes.push(RequiredChange::PatchFunction {
                            function_id: current.id.clone(),
                            function_name: f_name.clone(),
                            input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                            output_mapping: current.desired_mapping.physical_output_mapping.clone(),
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
                        input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                        output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                        annotations: function.annotations.clone(),
                    });
                    current.materialized = Some(PhysicalPorts {
                        physical_input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                        physical_output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                    });
                }
            }
        }

        for (r_name, resource) in &mut self.resources {
            let mut resource = resource.borrow_mut();
            for i in &resource.instances {
                let mut current = i.borrow_mut();
                if let Some(materialized) = &current.materialized {
                    if materialized.physical_input_mapping != current.desired_mapping.physical_input_mapping
                        || materialized.physical_output_mapping != current.desired_mapping.physical_output_mapping
                    {
                        changes.push(RequiredChange::PatchResource {
                            resource_id: current.id.clone(),
                            resource_name: r_name.clone(),
                            input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                            output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                        });
                    }
                } else {
                    changes.push(RequiredChange::StartResource {
                        resource_id: current.id.clone(),
                        resource_name: r_name.clone(),
                        class_type: resource.class.clone(),
                        input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                        output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                        configuration: resource.configurations.clone(),
                    });
                    current.materialized = Some(PhysicalPorts {
                        physical_input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                        physical_output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                    });
                }
            }
        }

        for (s_name, subflow) in &mut self.subflows {
            let mut subflow = subflow.borrow_mut();
            for i in &subflow.instances {
                let mut current = i.borrow_mut();
                if let Some(materialized) = &current.materialized {
                    if materialized.physical_input_mapping != current.desired_mapping.physical_input_mapping
                        || materialized.physical_output_mapping != current.desired_mapping.physical_output_mapping
                    {
                        changes.push(RequiredChange::PatchSubflow {
                            subflow_id: current.id.clone(),
                            input_mapping: current.desired_mapping.physical_input_mapping.clone(),
                            output_mapping: current.desired_mapping.physical_output_mapping.clone(),
                        });
                    }
                } else {
                    changes.push(RequiredChange::CreateSubflow {
                        subflow_id: current.id.clone(),
                        spawn_req: edgeless_api::workflow_instance::SpawnWorkflowRequest {
                            workflow_functions: Vec::new(),
                            workflow_resources: Vec::new(),
                            workflow_ingress_proxies: current
                                .desired_mapping
                                .physical_input_mapping
                                .iter()
                                .map(|(id, physical_port)| edgeless_api::workflow_instance::WorkflowIngressProxy {
                                    id: id.0.clone(),
                                    inner_output: subflow.logical_ports.logical_output_mapping.get(&id).unwrap().clone(),
                                    external_input: physical_port.clone(),
                                })
                                .collect(),
                            workflow_egress_proxies: current
                                .desired_mapping
                                .physical_output_mapping
                                .iter()
                                .map(|(id, physical_port)| edgeless_api::workflow_instance::WorkflowEgressProxy {
                                    id: id.0.clone(),
                                    inner_input: match subflow.logical_ports.logical_input_mapping.get(&id).unwrap().clone() {
                                        LogicalInput::Direct(vec) => edgeless_api::workflow_instance::PortMapping::AnyOfTargets(vec),
                                        LogicalInput::Topic(topic) => edgeless_api::workflow_instance::PortMapping::Topic(topic),
                                    },
                                    external_output: physical_port.clone(),
                                })
                                .collect(),
                            annotations: std::collections::HashMap::new(),
                        },
                    });
                }
            }
        }

        {
            let prx = self.proxy.borrow_mut();
            for i in &prx.instances {
                let current = i.borrow_mut();
                if let Some(materialized) = &current.materialized {
                    if materialized.physical_input_mapping != current.desired_mapping.physical_input_mapping
                        || materialized.physical_output_mapping != current.desired_mapping.physical_output_mapping
                    {
                        changes.push(RequiredChange::PatchProxy {
                            proxy_id: current.id.clone(),
                            internal_inputs: current.desired_mapping.physical_input_mapping.clone(),
                            internal_outputs: current.desired_mapping.physical_output_mapping.clone(),
                            external_inputs: prx.external_ports.external_input_mapping.clone(),
                            external_outputs: prx.external_ports.external_output_mapping.clone(),
                        })
                    } else {
                        changes.push(RequiredChange::CrateProxy {
                            proxy_id: current.id.clone(),
                            internal_inputs: current.desired_mapping.physical_input_mapping.clone(),
                            internal_outputs: current.desired_mapping.physical_output_mapping.clone(),
                            external_inputs: prx.external_ports.external_input_mapping.clone(),
                            external_outputs: prx.external_ports.external_output_mapping.clone(),
                        });
                    }
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
            for (out_port, mapping) in &fdesc.borrow_mut().logical_ports().logical_output_mapping {
                match mapping {
                    LogicalOutput::DirectTarget(target_fid, target_port) => inputs
                        .entry(target_fid.clone())
                        .or_default()
                        .entry(target_port.clone())
                        .or_default()
                        .push((out_cid.to_string(), out_port.clone())),
                    LogicalOutput::AnyOfTargets(targets) => {
                        for (target_fid, target_port) in targets {
                            inputs
                                .entry(target_fid.clone())
                                .or_default()
                                .entry(target_port.clone())
                                .or_default()
                                .push((out_cid.to_string(), out_port.clone()))
                        }
                    }
                    LogicalOutput::AllOfTargets(targets) => {
                        for (target_fid, target_port) in targets {
                            inputs
                                .entry(target_fid.clone())
                                .or_default()
                                .entry(target_port.clone())
                                .or_default()
                                .push((out_cid.to_string(), out_port.clone()))
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
                        .logical_ports
                        .logical_input_mapping
                        .insert(target_port.clone(), LogicalInput::Direct(sources.clone()));
                }
            } else if let Some(target) = self.resources.get_mut(targed_fid) {
                // Some(&mut target.borrow_mut().ports)
                for (target_port, sources) in links {
                    target
                        .borrow_mut()
                        .logical_ports
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
                .logical_ports()
                .logical_input_mapping
                .retain(|port_id, port_mapping| match port_mapping {
                    LogicalInput::Topic(topic) => {
                        targets
                            .entry(topic.clone())
                            .or_insert(Vec::new())
                            .push((cid.to_string(), port_id.clone()));
                        false
                    }
                    _ => true,
                })
        }

        // Create Outputs

        for (cid, component) in &mut self.components() {
            component
                .borrow_mut()
                .logical_ports()
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
            let ports = &mut f.logical_ports();
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
                if let Some(source_port) = source.logical_ports().logical_input_mapping.get_mut(target_port_id) {
                    if let LogicalInput::Direct(sources) = source_port {
                        sources.retain(|(s_id, s_p_id)| s_id != source_component_id && s_p_id != source_port_id);
                        if sources.len() == 0 {
                            remove = true;
                        }
                    }
                }
                if remove {
                    source.logical_ports().logical_input_mapping.remove(target_port_id);
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
            let f_ports = &mut f.logical_ports();
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
                if let Some(source_port) = source.logical_ports().logical_output_mapping.get_mut(source_port_id) {
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
                    source.logical_ports().logical_output_mapping.remove(source_port_id);
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
            let (logical_ports, physical_instances) = current.split_view();
            for i in &physical_instances {
                for (out_id, out) in &mut i.borrow_mut().physical_ports().physical_output_mapping {
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

                                let logical_port = logical_ports.logical_output_mapping.get(out_id).unwrap();
                                if let edgeless_api::workflow_instance::PortMapping::AllOfTargets(logical_targets) = logical_port {
                                    for (target_name, target_port_id) in logical_targets {
                                        self.get_component(&target_name).unwrap().borrow_mut().instances().iter().for_each(|i| {
                                            i.borrow_mut()
                                                .physical_ports()
                                                .physical_input_mapping
                                                .insert(target_port_id.clone(), PhysicalInput::Link(new_link.clone()));
                                        });
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

    fn components(&self) -> Vec<(&str, &std::cell::RefCell<dyn WorkflowComponent>)> {
        let mut components = self
            .functions
            .iter()
            .map(|(f_id, f)| (f_id.as_str(), f as &std::cell::RefCell<dyn WorkflowComponent>))
            .collect::<Vec<_>>();
        components.append(
            &mut self
                .resources
                .iter()
                .map(|(r_id, r)| (r_id.as_str(), r as &std::cell::RefCell<dyn WorkflowComponent>))
                .collect::<Vec<_>>(),
        );
        components.append(
            &mut self
                .subflows
                .iter()
                .map(|(s_id, s)| (s_id.as_str(), s as &std::cell::RefCell<dyn WorkflowComponent>))
                .collect::<Vec<_>>(),
        );
        components.push(("__proxy", &self.proxy as &std::cell::RefCell<dyn WorkflowComponent>));
        components
    }

    // fn instances_for_component(&mut self, component_name: &str) -> Option<Vec<edgeless_api::function_instance::InstanceId>> {
    //     if let Some(component) = self.get_component(component_name) {
    //         Some(component.borrow_mut().instance_ids())
    //     } else {
    //         None
    //     }
    // }

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

    fn select_node_for_proxy(
        _proxy: &WorkflowProxy,
        nodes: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::WorkerNode>,
    ) -> Option<edgeless_api::function_instance::NodeId> {
        for (node_id, node) in nodes {
            if node.is_proxy {
                return Some(node_id.clone());
            }
        }
        return None;
    }

    fn select_cluster_for_subflow(
        subflow: &SubFlow,
        clusters: &std::collections::HashMap<edgeless_api::function_instance::NodeId, crate::controller::server::PeerCluster>,
    ) -> Option<edgeless_api::function_instance::NodeId> {
        for (cluster_id, cluster) in clusters {
            // TODO Proper Selection
            return Some(cluster_id.clone());
        }
        return None;
    }
}
