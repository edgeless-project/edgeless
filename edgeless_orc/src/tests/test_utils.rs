#![allow(clippy::all)]

use crate::domain_subscriber::DomainSubscriberRequest;
use crate::proxy::Proxy;
use crate::orchestrator::*;
use edgeless_api::function_instance::{FunctionClassSpecification, StatePolicy, StateSpecification};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::SinkExt;

pub fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[allow(dead_code)]
pub enum MockAgentEvent {
    StartFunction(
        (
            edgeless_api::function_instance::InstanceId,
            edgeless_api::function_instance::SpawnFunctionRequest,
        ),
    ),
    StopFunction(edgeless_api::function_instance::InstanceId),
    PatchFunction(edgeless_api::common::PatchRequest),
    #[allow(dead_code)]
    UpdatePeers(edgeless_api::node_management::UpdatePeersRequest),
    StartResource(
        (
            edgeless_api::function_instance::InstanceId,
            edgeless_api::resource_configuration::ResourceInstanceSpecification,
        ),
    ),
    StopResource(edgeless_api::function_instance::InstanceId),
    PatchResource(edgeless_api::common::PatchRequest),
    Reset(),
}

pub struct MockNode {
    pub node_id: uuid::Uuid,
    pub sender: futures::channel::mpsc::UnboundedSender<MockAgentEvent>,
}

impl edgeless_api::outer::agent::AgentAPI for MockNode {
    fn function_instance_api(
        &mut self,
    ) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>> {
        Box::new(MockAgentAPI {
            node_id: self.node_id,
            sender: self.sender.clone(),
        })
    }
    fn node_management_api(&mut self) -> Box<dyn edgeless_api::node_management::NodeManagementAPI> {
        Box::new(MockAgentAPI {
            node_id: self.node_id,
            sender: self.sender.clone(),
        })
    }
    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>> {
        Box::new(MockAgentAPI {
            node_id: self.node_id,
            sender: self.sender.clone(),
        })
    }
}

#[derive(Clone)]
pub struct MockAgentAPI {
    pub node_id: uuid::Uuid,
    pub sender: futures::channel::mpsc::UnboundedSender<MockAgentEvent>,
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId> for MockAgentAPI {
    async fn start(
        &mut self,
        spawn_request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let new_id = edgeless_api::function_instance::InstanceId {
            node_id: self.node_id,
            function_id: uuid::Uuid::new_v4(),
        };
        self.sender.send(MockAgentEvent::StartFunction((new_id, spawn_request))).await.unwrap();
        Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.sender.send(MockAgentEvent::StopFunction(id)).await.unwrap();
        Ok(())
    }
    async fn patch(&mut self, request: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        self.sender.send(MockAgentEvent::PatchFunction(request)).await.unwrap();
        Ok(())
    }
}

#[async_trait::async_trait]
impl edgeless_api::node_management::NodeManagementAPI for MockAgentAPI {
    async fn update_peers(&mut self, request: edgeless_api::node_management::UpdatePeersRequest) -> anyhow::Result<()> {
        self.sender.send(MockAgentEvent::UpdatePeers(request)).await.unwrap();
        Ok(())
    }
    async fn reset(&mut self) -> anyhow::Result<()> {
        self.sender.send(MockAgentEvent::Reset()).await.unwrap();
        Ok(())
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for MockAgentAPI {
    async fn start(
        &mut self,
        start_request: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let new_id = edgeless_api::function_instance::InstanceId {
            node_id: self.node_id,
            function_id: uuid::Uuid::new_v4(),
        };
        self.sender.send(MockAgentEvent::StartResource((new_id, start_request))).await.unwrap();
        Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.sender.send(MockAgentEvent::StopResource(id)).await.unwrap();
        Ok(())
    }
    async fn patch(&mut self, request: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        self.sender.send(MockAgentEvent::PatchResource(request)).await.unwrap();
        Ok(())
    }
}

pub type ClientDescsResources = std::collections::HashMap<
    uuid::Uuid,
    (
        crate::client_desc::ClientDesc,
        Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
    ),
>;

#[allow(clippy::type_complexity)]
pub fn create_clients_resources(
    num_nodes: u32,
    num_resources_per_node: u32,
) -> (
    std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>,
    ClientDescsResources,
    uuid::Uuid,
) {
    assert!(num_nodes > 0);

    let mut nodes = std::collections::HashMap::new();
    let mut client_descs_resources = std::collections::HashMap::new();
    let mut stable_node_id = uuid::Uuid::nil();
    for node_i in 0..num_nodes {
        let (mock_node_sender, mock_node_receiver) = futures::channel::mpsc::unbounded::<MockAgentEvent>();
        let node_id = uuid::Uuid::new_v4();
        let mut capabilities = edgeless_api::node_registration::NodeCapabilities::minimum();
        if node_i == 0 {
            stable_node_id = node_id;
            capabilities.labels.push("stable".to_string());
        } else {
            capabilities.labels.push("unstable".to_string());
        }
        nodes.insert(node_id, mock_node_receiver);

        let client_desc = crate::client_desc::ClientDesc {
            agent_url: "".to_string(),
            invocation_url: "".to_string(),
            api: Box::new(MockNode {
                node_id,
                sender: mock_node_sender,
            }) as Box<dyn edgeless_api::outer::agent::AgentAPI + Send>,
            capabilities,
            cordoned: false,
        };

        let mut resources = vec![];
        for provider_i in 0..num_resources_per_node {
            resources.push(edgeless_api::node_registration::ResourceProviderSpecification {
                provider_id: format!("node-{}-resource-{}-provider", node_i, provider_i),
                class_type: "rc-1".to_string(),
                outputs: vec![],
            });
        }

        client_descs_resources.insert(node_id, (client_desc, resources));
    }

    (nodes, client_descs_resources, stable_node_id)
}

pub struct SetupResult {
    pub fun_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
    pub res_client: Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
    pub nodes: std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>,
    pub stable_node_id: uuid::Uuid,
    pub subscriber_receiver: UnboundedReceiver<DomainSubscriberRequest>,
    pub orc_sender: UnboundedSender<OrchestratorRequest>,
    pub proxy: std::sync::Arc<tokio::sync::Mutex<dyn Proxy>>,
}

pub async fn setup(num_nodes: u32, num_resources_per_node: u32) -> SetupResult {
    let (mut nodes, client_descs_resources, stable_node_id) = create_clients_resources(num_nodes, num_resources_per_node);
    let (subscriber_sender, subscriber_receiver) = futures::channel::mpsc::unbounded();

    let proxy = std::sync::Arc::new(tokio::sync::Mutex::new(crate::orchestrator::proxy_local::ProxyLocal::default()));
    let (mut orchestrator, orchestrator_task, _refresh_task) = Orchestrator::new(
        crate::EdgelessOrcBaselineSettings {
            orchestration_strategy: crate::OrchestrationStrategy::Random,
        },
        proxy.clone(),
        subscriber_sender,
    )
    .await;
    tokio::spawn(orchestrator_task);

    let mut orchestrator_sender = orchestrator.get_sender();
    for (node_id, (client_desc, resources)) in client_descs_resources {
        let _ = orchestrator_sender
            .send(crate::orchestrator::OrchestratorRequest::AddNode(node_id, client_desc, resources))
            .await;
    }

    clear_events(&mut nodes).await;

    SetupResult {
        fun_client: orchestrator.get_api_client().function_instance_api(),
        res_client: orchestrator.get_api_client().resource_configuration_api(),
        nodes,
        stable_node_id,
        subscriber_receiver,
        orc_sender: orchestrator.get_sender(),
        proxy,
    }
}

#[allow(dead_code)]
pub fn event_to_string(event: &MockAgentEvent) -> &'static str {
    match event {
        MockAgentEvent::StartFunction(_) => "start-function",
        MockAgentEvent::StopFunction(_) => "stop-function",
        MockAgentEvent::PatchFunction(_) => "patch-function",
        MockAgentEvent::StartResource(_) => "start-resource",
        MockAgentEvent::StopResource(_) => "stop-resource",
        MockAgentEvent::PatchResource(_) => "patch-resource",
        MockAgentEvent::UpdatePeers(_) => "update-peers",
        MockAgentEvent::Reset() => "reset",
    }
}

#[allow(dead_code)]
pub fn msg_to_string(msg: Result<Option<MockAgentEvent>, futures::channel::mpsc::TryRecvError>) -> &'static str {
    match msg {
        Ok(val) => match val {
            Some(val) => event_to_string(&val),
            None => "none",
        },
        Err(_) => "error",
    }
}

pub async fn wait_for_function_event(receiver: &mut futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>) -> MockAgentEvent {
    for _ in 0..100 {
        if let Ok(Some(event)) = receiver.try_next() {
            return event;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
    panic!("timeout while waiting for an event");
}

pub async fn wait_for_event_multiple(
    receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>,
) -> (uuid::Uuid, MockAgentEvent) {
    for _ in 0..100 {
        for (node_id, receiver) in receivers.iter_mut() {
            if let Ok(Some(event)) = receiver.try_next() {
                return (*node_id, event);
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
    panic!("timeout while waiting for an event");
}

pub async fn wait_for_event_at_node(receiver: &mut futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>) -> MockAgentEvent {
    for _ in 0..100 {
        if let Ok(Some(event)) = receiver.try_next() {
            return event;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
    panic!("timeout while waiting for an event");
}

pub async fn wait_for_events_if_any(
    receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>,
) -> Option<(uuid::Uuid, MockAgentEvent)> {
    for _ in 0..100 {
        for (node_id, receiver) in receivers.iter_mut() {
            if let Ok(Some(event)) = receiver.try_next() {
                return Some((*node_id, event));
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
    None
}

pub async fn no_function_event(receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    for (node_id, receiver) in receivers.iter_mut() {
        if let Ok(Some(event)) = receiver.try_next() {
            panic!("expecting no event, but received one on node {}: {}", node_id, event_to_string(&event));
        }
    }
}

#[allow(dead_code)]
pub async fn print_events(receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    for (node_id, receiver) in receivers.iter_mut() {
        if let Ok(Some(event)) = receiver.try_next() {
            println!("node_id {} event {}", node_id, event_to_string(&event));
        }
    }
}

pub async fn clear_events(receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    for (_node_id, receiver) in receivers.iter_mut() {
        while let Ok(Some(_event)) = receiver.try_next() {}
    }
}

pub fn make_spawn_function_request(class_id: &str) -> edgeless_api::function_instance::SpawnFunctionRequest {
    edgeless_api::function_instance::SpawnFunctionRequest {
        spec: FunctionClassSpecification {
            id: class_id.to_string(),
            function_type: "RUST_WASM".to_string(),
            version: "0.1".to_string(),
            binary: Some("function_code".as_bytes().to_vec()),
            code: None,
            outputs: vec![],
        },
        annotations: std::collections::HashMap::new(),
        state_specification: StateSpecification {
            state_id: uuid::Uuid::new_v4(),
            state_policy: StatePolicy::NodeLocal,
        },
        workflow_id: "workflow_1".to_string(),
        replication_factor: Some(1),
    }
}

pub fn make_start_resource_request(class_type: &str) -> edgeless_api::resource_configuration::ResourceInstanceSpecification {
    edgeless_api::resource_configuration::ResourceInstanceSpecification {
        class_type: class_type.to_string(),
        configuration: std::collections::HashMap::new(),
        workflow_id: "workflow_1".to_string(),
    }
}