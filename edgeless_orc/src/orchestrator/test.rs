// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#![allow(clippy::all)]

use crate::deployment_requirements::DeploymentRequirements;
use crate::domain_subscriber::DomainSubscriberRequest;
use crate::proxy::Proxy;
use crate::{affinity_level::AffinityLevel, deploy_intent};
use edgeless_api::function_instance::{FunctionClassSpecification, StatePolicy, StateSpecification};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};

use super::*;

fn init_logger() {
    let _ = env_logger::builder().is_test(true).try_init();
}

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
struct MockAgentAPI {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockAgentEvent>,
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

type ClientDescsResources = std::collections::HashMap<
    uuid::Uuid,
    (
        crate::client_desc::ClientDesc,
        Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
    ),
>;

#[allow(clippy::type_complexity)]
fn create_clients_resources(
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

struct SetupResult {
    fun_client: Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
    res_client: Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
    nodes: std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>,
    stable_node_id: uuid::Uuid,
    subscriber_receiver: UnboundedReceiver<DomainSubscriberRequest>,
    orc_sender: UnboundedSender<OrchestratorRequest>,
    proxy: std::sync::Arc<tokio::sync::Mutex<proxy_test::ProxyTest>>,
}

async fn setup(num_nodes: u32, num_resources_per_node: u32) -> SetupResult {
    let (mut nodes, client_descs_resources, stable_node_id) = create_clients_resources(num_nodes, num_resources_per_node);
    let (subscriber_sender, subscriber_receiver) = futures::channel::mpsc::unbounded();

    let proxy = std::sync::Arc::new(tokio::sync::Mutex::new(proxy_test::ProxyTest::default()));
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
fn event_to_string(event: &MockAgentEvent) -> &'static str {
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
fn msg_to_string(msg: Result<Option<MockAgentEvent>, futures::channel::mpsc::TryRecvError>) -> &'static str {
    match msg {
        Ok(val) => match val {
            Some(val) => event_to_string(&val),
            None => "none",
        },
        Err(_) => "error",
    }
}

async fn wait_for_function_event(receiver: &mut futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>) -> MockAgentEvent {
    for _ in 0..100 {
        if let Ok(Some(event)) = receiver.try_next() {
            return event;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
    panic!("timeout while waiting for an event");
}

async fn wait_for_event_multiple(
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

async fn wait_for_event_at_node(receiver: &mut futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>) -> MockAgentEvent {
    for _ in 0..100 {
        if let Ok(Some(event)) = receiver.try_next() {
            return event;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
    panic!("timeout while waiting for an event");
}

async fn wait_for_events_if_any(
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

async fn no_function_event(receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    for (node_id, receiver) in receivers.iter_mut() {
        if let Ok(Some(event)) = receiver.try_next() {
            panic!("expecting no event, but received one on node {}: {}", node_id, event_to_string(&event));
        }
    }
}

#[allow(dead_code)]
async fn print_events(receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    for (node_id, receiver) in receivers.iter_mut() {
        if let Ok(Some(event)) = receiver.try_next() {
            println!("node_id {} event {}", node_id, event_to_string(&event));
        }
    }
}

async fn clear_events(receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    for (_node_id, receiver) in receivers.iter_mut() {
        while let Ok(Some(_event)) = receiver.try_next() {}
    }
}

fn make_spawn_function_request(class_id: &str) -> edgeless_api::function_instance::SpawnFunctionRequest {
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

fn make_start_resource_request(class_type: &str) -> edgeless_api::resource_configuration::ResourceInstanceSpecification {
    edgeless_api::resource_configuration::ResourceInstanceSpecification {
        class_type: class_type.to_string(),
        configuration: std::collections::HashMap::new(),
        workflow_id: "workflow_1".to_string(),
    }
}

#[tokio::test]
async fn test_orc_single_node_function_start_stop() {
    let mut setup = setup(1, 0).await;

    assert_eq!(1, setup.nodes.len());
    let (node_id, mock_node_receiver) = setup.nodes.iter_mut().next().unwrap();
    assert!(!node_id.is_nil());

    assert!(mock_node_receiver.try_next().is_err());

    // Start a function.

    let spawn_req = make_spawn_function_request("fc-1");
    let instance_id = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };

    let mut int_instance_id = None;
    if let MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd)) = wait_for_function_event(mock_node_receiver).await {
        assert!(int_instance_id.is_none());
        int_instance_id = Some(new_instance_id);
        assert_eq!(spawn_req, spawn_req_rcvd);
    } else {
        panic!("wrong event received");
    }

    // Stop the function previously started.

    match setup.fun_client.stop(instance_id).await {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }

    if let MockAgentEvent::StopFunction(instance_id_rcvd) = wait_for_function_event(mock_node_receiver).await {
        assert!(int_instance_id.is_some());
        assert_eq!(int_instance_id.unwrap(), instance_id_rcvd);
    } else {
        panic!("wrong event received");
    }

    // Stop the function again.
    match setup.fun_client.stop(instance_id).await {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }
    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
async fn test_orc_multiple_nodes_function_start_stop() {
    let mut setup = setup(3, 0).await;

    // Start 100 functions.

    let mut ext_instance_ids = vec![];
    let mut int_instance_ids = vec![];
    let mut node_ids = vec![];
    for i in 0..100 {
        let spawn_req = make_spawn_function_request(format!("fc-{}", i).as_str());
        ext_instance_ids.push(match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });

        if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
            node_ids.push(node_id);
            int_instance_ids.push(new_instance_id);
            assert_eq!(spawn_req, spawn_req_rcvd);
        } else {
            panic!("wrong event received");
        }
    }

    // Check that all nodes have been selected at least once.

    let mut nodes_selected = std::collections::HashSet::new();
    for node_id in node_ids.iter() {
        nodes_selected.insert(node_id);
    }
    assert_eq!(3, nodes_selected.len());

    // Stop the functions previously started.

    assert_eq!(100, ext_instance_ids.len());
    assert_eq!(100, int_instance_ids.len());
    assert_eq!(100, node_ids.len());
    for i in 0..100 {
        match setup.fun_client.stop(ext_instance_ids[i]).await {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }

        if let (node_id, MockAgentEvent::StopFunction(instance_id_rcvd)) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(node_ids[i], node_id);
            assert_eq!(int_instance_ids[i], instance_id_rcvd);
        } else {
            panic!("wrong event received");
        }
    }
}

#[tokio::test]
async fn test_orc_multiple_resources_start_stop() {
    let mut setup = setup(3, 3).await;

    // Start 100 resources.

    let mut ext_instance_ids = vec![];
    let mut int_instance_ids = vec![];
    let mut node_ids = vec![];
    for _i in 0..100 {
        let start_req = make_start_resource_request("rc-1");
        ext_instance_ids.push(match setup.res_client.start(start_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });

        if let (node_id, MockAgentEvent::StartResource((int_instance_id, resource_instance_spec))) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(node_id, int_instance_id.node_id);
            node_ids.push(node_id);
            int_instance_ids.push(int_instance_id.function_id);
            assert!(resource_instance_spec.configuration.is_empty());
        } else {
            panic!("wrong event received");
        }
    }

    // Check that all the nodes have been selected at least once.

    let mut nodes_selected = std::collections::HashSet::new();
    for node_id in node_ids.iter() {
        nodes_selected.insert(node_id);
    }
    assert_eq!(3, nodes_selected.len());

    // Stop the resources previously started.

    assert_eq!(100, ext_instance_ids.len());
    assert_eq!(100, int_instance_ids.len());
    assert_eq!(100, node_ids.len());
    for i in 0..100 {
        match setup.res_client.stop(ext_instance_ids[i]).await {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }

        if let (node_id, MockAgentEvent::StopResource(instance_id_rcvd)) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(node_ids[i], node_id);
            assert_eq!(node_ids[i], instance_id_rcvd.node_id);
            assert_eq!(int_instance_ids[i], instance_id_rcvd.function_id);
        } else {
            panic!("wrong event received");
        }
    }

    // Start a resource with unknown class type.
    match setup.res_client.start(make_start_resource_request("rc-666")).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(_) => {
            panic!("started a resource for a non-existing class type");
        }
        edgeless_api::common::StartComponentResponse::ResponseError(err) => {
            assert_eq!("class type not found".to_string(), err.summary);
        }
    }
}

#[tokio::test]
async fn test_orc_patch() {
    let mut setup = setup(1, 1).await;
    assert_eq!(1, setup.nodes.len());
    let client_node_id = *setup.nodes.keys().next().unwrap();

    // Spawn a function instance.

    let spawn_req = make_spawn_function_request("fc-1");
    let ext_function_id = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };

    let mut int_function_id = None;
    assert!(int_function_id.is_none());
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(client_node_id, node_id);
        assert_eq!(client_node_id, new_instance_id.node_id);
        int_function_id = Some(new_instance_id);
        assert_eq!(spawn_req, spawn_req_rcvd);
    } else {
        panic!("wrong event received");
    }

    // Start a resource.

    let start_req = make_start_resource_request("rc-1");
    let ext_resource_id = match setup.res_client.start(start_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };

    let mut int_resource_id = None;
    assert!(int_resource_id.is_none());
    if let (node_id, MockAgentEvent::StartResource((new_instance_id, resource_instance_spec))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(client_node_id, node_id);
        assert_eq!(client_node_id, new_instance_id.node_id);
        int_resource_id = Some(new_instance_id);
        assert!(resource_instance_spec.configuration.is_empty());
    } else {
        panic!("wrong event received");
    }

    // Gotta patch 'em all.

    match setup
        .fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: ext_function_id,
            output_mapping: std::collections::HashMap::from([(
                "out-1".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_resource_id,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };

    if let (node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(client_node_id, node_id);
        assert_eq!(int_function_id.unwrap().function_id, patch_request.function_id);
        assert_eq!(1, patch_request.output_mapping.len());
        let mapping = patch_request.output_mapping.get("out-1");
        assert!(mapping.is_some());
        assert_eq!(int_resource_id.unwrap(), mapping.unwrap().clone());
    } else {
        panic!("wrong event received");
    }

    match setup
        .res_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: ext_resource_id,
            output_mapping: std::collections::HashMap::from([(
                "out-2".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_function_id,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };

    if let (node_id, MockAgentEvent::PatchResource(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(client_node_id, node_id);
        assert_eq!(int_resource_id.unwrap().function_id, patch_request.function_id);
        assert_eq!(1, patch_request.output_mapping.len());
        let mapping = patch_request.output_mapping.get("out-2");
        assert!(mapping.is_some());
        assert_eq!(int_function_id.unwrap(), mapping.unwrap().clone());
    } else {
        panic!("wrong event received");
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_orc_node_with_fun_disconnects() {
    let mut setup = setup(10, 0).await;

    // Start this workflow
    //
    // f1 -> f2 -> f3 -> f4
    //
    // One node is "stable", the others can be disconnected
    //
    // f1 & f3 & f4 are forced to be allocated on the stable noe
    // f2 is forced to be allocated on a node that will disconnect
    //

    // Start f1
    let mut spawn_req = make_spawn_function_request("f1");
    spawn_req
        .annotations
        .insert("node_id_match_any".to_string(), setup.stable_node_id.to_string());
    let lid_1 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut pid_1 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(node_id, setup.stable_node_id);
        pid_1 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f2
    let mut spawn_req = make_spawn_function_request("f2");
    spawn_req.annotations.insert("label_match_all".to_string(), "unstable".to_string());
    let lid_2 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut unstable_node_id = uuid::Uuid::nil();
    let mut pid_2 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_ne!(node_id, setup.stable_node_id);
        unstable_node_id = node_id;
        pid_2 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f3
    let mut spawn_req = make_spawn_function_request("f3");
    spawn_req
        .annotations
        .insert("node_id_match_any".to_string(), setup.stable_node_id.to_string());
    let lid_3 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut pid_3 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(node_id, setup.stable_node_id);
        pid_3 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f4
    let mut spawn_req = make_spawn_function_request("f4");
    spawn_req
        .annotations
        .insert("node_id_match_any".to_string(), setup.stable_node_id.to_string());
    let lid_4 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut _pid_4 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(node_id, setup.stable_node_id);
        _pid_4 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Patch f1->f2
    match setup
        .fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: lid_1,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lid_2,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Patch f2->f3
    match setup
        .fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: lid_2,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lid_3,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Patch f3->f4
    match setup
        .fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: lid_3,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lid_4,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };

    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Make sure there are no pending events around.
    no_function_event(&mut setup.nodes).await;

    // Disconnect the unstable node
    let _ = setup.orc_sender.send(OrchestratorRequest::DelNode(unstable_node_id)).await;

    let mut num_events = std::collections::HashMap::new();
    let mut new_node_id = uuid::Uuid::nil();
    let mut patch_request_1 = None;
    let mut patch_request_2 = None;
    loop {
        if let Some((node_id, event)) = wait_for_events_if_any(&mut setup.nodes).await {
            if num_events.contains_key(event_to_string(&event)) {
                *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
            } else {
                num_events.insert(event_to_string(&event), 1);
            }
            match event {
                MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd)) => {
                    log::info!("start-function");
                    assert_ne!(node_id, setup.stable_node_id);
                    assert_eq!(node_id, new_instance_id.node_id);
                    new_node_id = new_instance_id.node_id;
                    pid_2 = new_instance_id.function_id;
                    assert_eq!("f2", spawn_req_rcvd.spec.id);
                }
                MockAgentEvent::PatchFunction(patch_request) => {
                    log::info!("patch-function");
                    assert!(patch_request.output_mapping.contains_key("out"));
                    if node_id == setup.stable_node_id {
                        patch_request_1 = Some(patch_request);
                    } else if node_id == new_node_id {
                        patch_request_2 = Some(patch_request);
                    }
                }
                MockAgentEvent::UpdatePeers(req) => {
                    log::info!("update-peers");
                    match req {
                        edgeless_api::node_management::UpdatePeersRequest::Del(del_node_id) => {
                            assert_eq!(unstable_node_id, del_node_id);
                        }
                        _ => panic!("wrong UpdatePeersRequest message"),
                    }
                }
                _ => panic!("unexpected event type: {}", event_to_string(&event)),
            };
        } else {
            break;
        }
    }
    assert_eq!(Some(&9), num_events.get("update-peers"));
    assert_eq!(Some(&2), num_events.get("patch-function"));
    assert_eq!(Some(&1), num_events.get("start-function"));

    let patch_request_1 = patch_request_1.unwrap();
    let patch_request_2 = patch_request_2.unwrap();
    assert_eq!(pid_1, patch_request_1.function_id);
    assert_eq!(pid_2, patch_request_1.output_mapping.get("out").unwrap().function_id);
    assert_eq!(pid_2, patch_request_2.function_id);
    assert_eq!(pid_3, patch_request_2.output_mapping.get("out").unwrap().function_id);

    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
async fn test_orc_node_with_res_disconnects() {
    let mut setup = setup(10, 1).await;

    // Start this workflow
    //
    // f1 -> res
    //
    // One node is "stable", the others can be disconnected
    //
    // f1 is forced to be allocated on the stable noe
    // res is forced to be allocated on a node that will disconnect
    //

    // Start f1
    let mut spawn_req = make_spawn_function_request("f1");
    spawn_req
        .annotations
        .insert("node_id_match_any".to_string(), setup.stable_node_id.to_string());
    let lid_1 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut pid_1 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(node_id, setup.stable_node_id);
        pid_1 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start r1
    let start_req = make_start_resource_request("rc-1");

    let mut unstable_node_id = uuid::Uuid::nil();
    let mut pid_res = uuid::Uuid::nil();
    let mut lid_res = uuid::Uuid::nil();
    assert!(lid_res.is_nil());
    loop {
        lid_res = match setup.res_client.start(start_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        };

        if let (node_id, MockAgentEvent::StartResource((int_instance_id, resource_instance_spec))) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(node_id, int_instance_id.node_id);
            unstable_node_id = int_instance_id.node_id;
            pid_res = int_instance_id.function_id;
            assert!(resource_instance_spec.configuration.is_empty());
            if int_instance_id.node_id != setup.stable_node_id {
                break;
            }
        }

        // If we reach this point then the stable node has been selected,
        // so we stop the resource and try again.
        match setup.res_client.stop(lid_res).await {
            Ok(_) => {}
            Err(err) => panic!("{}", err),
        }

        if let (node_id, MockAgentEvent::StopResource(instance_id_rcvd)) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(unstable_node_id, node_id);
            assert_eq!(unstable_node_id, instance_id_rcvd.node_id);
            assert_eq!(pid_res, instance_id_rcvd.function_id);
        }
    }
    assert!(!unstable_node_id.is_nil());
    assert!(!pid_res.is_nil());
    assert!(!lid_res.is_nil());

    // Patch f1->res
    match setup
        .fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: lid_1,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lid_res,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
        assert_eq!(unstable_node_id, patch_request.output_mapping.get("out").unwrap().node_id);
        assert_eq!(pid_res, patch_request.output_mapping.get("out").unwrap().function_id);
    }

    // Make sure there are no pending events around.
    no_function_event(&mut setup.nodes).await;

    // Disconnect the unstable node
    let _ = setup.orc_sender.send(OrchestratorRequest::DelNode(unstable_node_id)).await;

    let mut num_events = std::collections::HashMap::new();
    let mut new_node_id = uuid::Uuid::nil();
    let mut patch_request_rcv: Option<edgeless_api::common::PatchRequest> = None;
    loop {
        if let Some((node_id, event)) = wait_for_events_if_any(&mut setup.nodes).await {
            if num_events.contains_key(event_to_string(&event)) {
                *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
            } else {
                num_events.insert(event_to_string(&event), 1);
            }
            match event {
                MockAgentEvent::StartResource((new_instance_id, _resource_instance_spec)) => {
                    log::info!("start-resource");
                    assert_eq!(node_id, new_instance_id.node_id);
                    new_node_id = new_instance_id.node_id;
                    pid_res = new_instance_id.function_id;
                }
                MockAgentEvent::PatchFunction(patch_request) => {
                    log::info!("patch-function");
                    assert!(patch_request.output_mapping.contains_key("out"));
                    assert_eq!(setup.stable_node_id, node_id);
                    patch_request_rcv = Some(patch_request);
                }
                MockAgentEvent::UpdatePeers(req) => {
                    log::info!("update-peers");
                    match req {
                        edgeless_api::node_management::UpdatePeersRequest::Del(del_node_id) => {
                            assert_eq!(unstable_node_id, del_node_id);
                        }
                        _ => panic!("wrong UpdatePeersRequest message"),
                    }
                }
                _ => panic!("unexpected event type: {}", event_to_string(&event)),
            };
        } else {
            break;
        }
    }
    assert_eq!(Some(&9), num_events.get("update-peers"));
    assert_eq!(Some(&1), num_events.get("patch-function"));
    assert_eq!(Some(&1), num_events.get("start-resource"));

    assert!(!new_node_id.is_nil());
    let patch_request_rcv = patch_request_rcv.unwrap();
    assert_eq!(pid_1, patch_request_rcv.function_id);
    assert_eq!(pid_res, patch_request_rcv.output_mapping.get("out").unwrap().function_id);

    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
async fn test_orc_patch_after_fun_stop() {
    let mut setup = setup(10, 0).await;

    // Start this workflow
    //
    // f1 -> f3 -> f4 -> f6
    //     /   \        /
    // f2 /     \  f5 /
    //
    // then stop f3

    // Start functions
    let mut lids = vec![];
    let mut pids = vec![];
    for i in 1..=6 {
        let spawn_req = make_spawn_function_request(format!("f{}", i).as_str());
        lids.push(match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });
        if let (_node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
            pids.push(new_instance_id.function_id);
            assert_eq!(spawn_req, spawn_req_rcvd);
        }
    }
    assert_eq!(6, lids.len());
    assert_eq!(6, pids.len());

    // Patch functions
    let patch_instructions = [
        (lids[0], vec![lids[2]]),
        (lids[1], vec![lids[2]]),
        (lids[2], vec![lids[3], lids[4]]),
        (lids[3], vec![lids[5]]),
        (lids[4], vec![lids[5]]),
    ];
    let patch_instructions_int = [
        (pids[0], vec![pids[2]]),
        (pids[1], vec![pids[2]]),
        (pids[2], vec![pids[3], pids[4]]),
        (pids[3], vec![pids[5]]),
        (pids[4], vec![pids[5]]),
    ];

    for j in 0..patch_instructions.len() {
        let lid_pair = &patch_instructions[j];
        let mut output_mapping = std::collections::HashMap::new();
        for i in 0..lid_pair.1.len() {
            output_mapping.insert(
                format!("out{}", i),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lid_pair.1[i],
                },
            );
        }
        match setup
            .fun_client
            .patch(edgeless_api::common::PatchRequest {
                function_id: lid_pair.0,
                output_mapping,
            })
            .await
        {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        };
        if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(patch_instructions_int[j].0, patch_request.function_id);
            assert!(patch_request.output_mapping.contains_key("out0"));
            assert_eq!(
                patch_request.output_mapping.get("out0").unwrap().function_id,
                patch_instructions_int[j].1[0]
            );
            if let Some(val) = patch_request.output_mapping.get("out1") {
                assert_eq!(val.function_id, patch_instructions_int[j].1[1]);
            }
        }
    }

    // Make sure there are no pending events around.
    no_function_event(&mut setup.nodes).await;

    // Stop function f3
    match setup.fun_client.stop(lids[2]).await {
        Ok(_) => {}
        Err(err) => panic!("{}", err),
    }

    let mut num_events = std::collections::HashMap::new();
    loop {
        if let Some((_node_id, event)) = wait_for_events_if_any(&mut setup.nodes).await {
            if num_events.contains_key(event_to_string(&event)) {
                *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
            } else {
                num_events.insert(event_to_string(&event), 1);
            }
            match event {
                MockAgentEvent::StopFunction(instance_id) => {
                    log::info!("stop-resource");
                    assert_eq!(pids[2], instance_id.function_id);
                }
                MockAgentEvent::PatchFunction(patch_request) => {
                    log::info!("patch-function");
                    assert!(patch_request.function_id == pids[0] || patch_request.function_id == pids[1]);
                    assert!(patch_request.output_mapping.is_empty());
                }
                _ => panic!("unexpected event type: {}", event_to_string(&event)),
            };
        } else {
            break;
        }
    }
    assert_eq!(Some(&1), num_events.get("stop-function"));
    assert_eq!(Some(&2), num_events.get("patch-function"));

    // Make sure there are no pending events around.
    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_orc_recreate_fun_after_disconnect() {
    let mut setup = setup(2, 0).await;

    // Start this workflow
    //
    // f1 -> f2 -> f3
    //
    // f1, f3 -> stable node
    // f2 -> unstable node which disconnects, then reconnects
    //

    // Start f1
    let mut spawn_req = make_spawn_function_request("f1");
    spawn_req
        .annotations
        .insert("node_id_match_any".to_string(), setup.stable_node_id.to_string());
    let lid_1 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut pid_1 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(node_id, setup.stable_node_id);
        pid_1 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f2
    let mut spawn_req = make_spawn_function_request("f2");
    spawn_req.annotations.insert("label_match_all".to_string(), "unstable".to_string());
    let lid_2 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut unstable_node_id = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((_new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_ne!(node_id, setup.stable_node_id);
        unstable_node_id = node_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f3
    let mut spawn_req = make_spawn_function_request("f3");
    spawn_req
        .annotations
        .insert("node_id_match_any".to_string(), setup.stable_node_id.to_string());
    let lid_3 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    if let (node_id, MockAgentEvent::StartFunction((_new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
        assert_eq!(node_id, setup.stable_node_id);
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Patch f1->f2
    match setup
        .fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: lid_1,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lid_2,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Patch f2->f3
    match setup
        .fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: lid_2,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lid_3,
                },
            )]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Make sure there are no pending events around.
    no_function_event(&mut setup.nodes).await;

    // Disconnect the unstable node
    let _ = setup.orc_sender.send(OrchestratorRequest::DelNode(unstable_node_id)).await;

    let mut num_events = std::collections::HashMap::new();
    loop {
        if let Some((_node_id, event)) = wait_for_events_if_any(&mut setup.nodes).await {
            if num_events.contains_key(event_to_string(&event)) {
                *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
            } else {
                num_events.insert(event_to_string(&event), 1);
            }
            match event {
                MockAgentEvent::UpdatePeers(req) => {
                    log::info!("update-peers");
                    match req {
                        edgeless_api::node_management::UpdatePeersRequest::Del(del_node_id) => {
                            assert_eq!(unstable_node_id, del_node_id);
                        }
                        _ => panic!("wrong UpdatePeersRequest message"),
                    }
                }
                MockAgentEvent::PatchFunction(patch_request) => {
                    log::info!("patch-function");
                    assert_eq!(pid_1, patch_request.function_id);
                    assert!(patch_request.output_mapping.is_empty());
                }
                _ => panic!("unexpected event type: {}", event_to_string(&event)),
            };
        } else {
            break;
        }
    }
    assert_eq!(Some(&1), num_events.get("update-peers"));
    assert_eq!(Some(&1), num_events.get("patch-function"));

    for _ in 0..5 {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<()>();
        let _ = setup.orc_sender.send(OrchestratorRequest::Refresh(reply_sender)).await;
        let _ = reply_receiver.await;

        let mut num_events = std::collections::HashMap::new();
        loop {
            if let Some((_node_id, event)) = wait_for_events_if_any(&mut setup.nodes).await {
                if num_events.contains_key(event_to_string(&event)) {
                    *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
                } else {
                    num_events.insert(event_to_string(&event), 1);
                }
                match event {
                    MockAgentEvent::PatchFunction(patch_request) => {
                        assert_eq!(pid_1, patch_request.function_id);
                        assert!(patch_request.output_mapping.is_empty());
                    }
                    _ => panic!("unexpected event type: {}", event_to_string(&event)),
                };
            } else {
                break;
            }
        }
        assert_eq!(Some(&1), num_events.get("patch-function"));
    }

    // Make sure there are no pending events.
    no_function_event(&mut setup.nodes).await;

    // Re-create the unstable node.

    let (mock_node_sender, mock_node_receiver) = futures::channel::mpsc::unbounded::<MockAgentEvent>();
    let mut capabilities = edgeless_api::node_registration::NodeCapabilities::minimum();
    capabilities.labels.push("unstable".to_string());

    if let Some(val) = setup.nodes.get_mut(&unstable_node_id) {
        *val = mock_node_receiver;
    }

    let _ = setup
        .orc_sender
        .send(crate::orchestrator::OrchestratorRequest::AddNode(
            unstable_node_id,
            crate::client_desc::ClientDesc {
                agent_url: "".to_string(),
                invocation_url: "".to_string(),
                api: Box::new(MockNode {
                    node_id: unstable_node_id,
                    sender: mock_node_sender,
                }) as Box<dyn edgeless_api::outer::agent::AgentAPI + Send>,
                capabilities,
                cordoned: false,
            },
            vec![],
        ))
        .await;

    if let Some(entry) = setup.nodes.get_mut(&setup.stable_node_id) {
        let mut num_update_peers = 0;
        for _ in 0..2 {
            let event = wait_for_event_at_node(entry).await;
            match event {
                MockAgentEvent::PatchFunction(patch_request) => assert!(patch_request.output_mapping.contains_key("out")),
                MockAgentEvent::UpdatePeers(_update) => {
                    num_update_peers += 1;
                }
                _ => panic!("unexpected event"),
            }
        }
        assert_eq!(1, num_update_peers);
    }
    if let Some(entry) = setup.nodes.get_mut(&unstable_node_id) {
        let mut num_update_peers = 0;
        let mut num_reset = 0;
        for _ in 0..5 {
            let event = wait_for_event_at_node(entry).await;
            match event {
                MockAgentEvent::StartFunction((_new_instance_id, spawn_req_rcvd)) => {
                    log::info!("{:?}", spawn_req_rcvd);
                    assert_eq!("f2", spawn_req_rcvd.spec.id);
                }
                MockAgentEvent::PatchFunction(patch_request) => assert!(patch_request.output_mapping.contains_key("out")),
                MockAgentEvent::UpdatePeers(_update) => {
                    num_update_peers += 1;
                }
                MockAgentEvent::Reset() => {
                    num_reset += 1;
                }
                _ => panic!("unexpected event"),
            }
        }
        assert_eq!(2, num_update_peers);
        assert_eq!(1, num_reset);
    }

    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
async fn test_orc_migrate_function() {
    let mut setup = setup(2, 0).await;

    // Deploy the following workflow:
    //
    // f0 <--> f1 <--> f2

    // Spawn the function instances.
    let mut lids = vec![];
    let mut pids = vec![];
    let mut nodes_assigned = vec![];
    for i in 0..=2 {
        let spawn_req = make_spawn_function_request(format!("fc-{}", i).as_str());
        lids.push(match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });

        if let (node_id, MockAgentEvent::StartFunction((new_instance_id, _spawn_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(node_id, new_instance_id.node_id);
            nodes_assigned.push(node_id);
            pids.push(new_instance_id.function_id);
        } else {
            panic!("wrong event received");
        }
    }

    // Gotta patch 'em all.
    let patch_requests = vec![
        edgeless_api::common::PatchRequest {
            function_id: lids[0],
            output_mapping: std::collections::HashMap::from([(
                "out-1".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lids[1],
                },
            )]),
        },
        edgeless_api::common::PatchRequest {
            function_id: lids[1],
            output_mapping: std::collections::HashMap::from([
                (
                    "out-1".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: lids[0],
                    },
                ),
                (
                    "out-2".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: lids[2],
                    },
                ),
            ]),
        },
        edgeless_api::common::PatchRequest {
            function_id: lids[2],
            output_mapping: std::collections::HashMap::from([(
                "out-1".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lids[1],
                },
            )]),
        },
    ];

    for (i, patch_request) in patch_requests.iter().enumerate() {
        match setup.fun_client.patch(patch_request.clone()).await {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        };
        if let (node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(node_id, nodes_assigned[i]);
            assert_eq!(patch_request.function_id, pids[i]);
        } else {
            panic!("wrong event received");
        }
    }

    // Ask to migrate function f1 to node to the other node.
    let old_node = nodes_assigned[1];

    let mut another_node = old_node;
    for node_id in setup.nodes.keys() {
        if *node_id != old_node {
            another_node = *node_id
        }
    }
    assert_ne!(another_node, old_node);

    setup
        .proxy
        .lock()
        .await
        .add_deploy_intents(vec![deploy_intent::DeployIntent::Migrate(lids[1], vec![another_node])]);

    let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<()>();
    let _ = setup.orc_sender.send(OrchestratorRequest::Refresh(reply_sender)).await;
    let _ = reply_receiver.await;

    let mut num_patches = 0;
    for _ in 0..5 {
        let (node_id, event) = wait_for_event_multiple(&mut setup.nodes).await;
        match event {
            MockAgentEvent::StartFunction((_new_instance_id, spawn_req_rcvd)) => {
                assert_eq!("fc-1", spawn_req_rcvd.spec.id);
                assert_eq!(another_node, node_id);
            }
            MockAgentEvent::StopFunction(_new_instance_id) => {
                assert_eq!(old_node, node_id);
            }
            MockAgentEvent::PatchFunction(_patch_request) => {
                num_patches += 1;
            }
            _ => panic!("unexpected event"),
        }
    }
    assert_eq!(3, num_patches);

    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
async fn test_orc_migrate_resource() {
    let mut setup = setup(2, 1).await;

    // Deploy the following workflow:
    //
    // r0 <--> r1 <--> r2

    // Spawn the resource instances.
    let mut lids = vec![];
    let mut pids = vec![];
    let mut nodes_assigned = vec![];
    for _ in 0..=2 {
        let resource_req = make_start_resource_request("rc-1");
        lids.push(match setup.res_client.start(resource_req).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });

        if let (node_id, MockAgentEvent::StartResource((new_instance_id, _resource_req_rcvd))) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(node_id, new_instance_id.node_id);
            nodes_assigned.push(node_id);
            pids.push(new_instance_id.function_id);
        } else {
            panic!("wrong event received");
        }
    }

    // Gotta patch 'em all.
    let patch_requests = vec![
        edgeless_api::common::PatchRequest {
            function_id: lids[0],
            output_mapping: std::collections::HashMap::from([(
                "out-1".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lids[1],
                },
            )]),
        },
        edgeless_api::common::PatchRequest {
            function_id: lids[1],
            output_mapping: std::collections::HashMap::from([
                (
                    "out-1".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: lids[0],
                    },
                ),
                (
                    "out-2".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: lids[2],
                    },
                ),
            ]),
        },
        edgeless_api::common::PatchRequest {
            function_id: lids[2],
            output_mapping: std::collections::HashMap::from([(
                "out-1".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: lids[1],
                },
            )]),
        },
    ];

    for (i, patch_request) in patch_requests.iter().enumerate() {
        match setup.fun_client.patch(patch_request.clone()).await {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        };
        if let (node_id, MockAgentEvent::PatchResource(patch_request)) = wait_for_event_multiple(&mut setup.nodes).await {
            assert_eq!(node_id, nodes_assigned[i]);
            assert_eq!(patch_request.function_id, pids[i]);
        } else {
            panic!("wrong event received");
        }
    }

    // Ask to migrate resource r1 to the other node.
    let old_node = nodes_assigned[1];

    let mut another_node = old_node;
    for node_id in setup.nodes.keys() {
        if *node_id != old_node {
            another_node = *node_id
        }
    }
    assert_ne!(another_node, old_node);

    setup
        .proxy
        .lock()
        .await
        .add_deploy_intents(vec![deploy_intent::DeployIntent::Migrate(lids[1], vec![another_node])]);

    let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<()>();
    let _ = setup.orc_sender.send(OrchestratorRequest::Refresh(reply_sender)).await;
    let _ = reply_receiver.await;

    let mut num_patches = 0;
    for _ in 0..5 {
        let (node_id, event) = wait_for_event_multiple(&mut setup.nodes).await;
        match event {
            MockAgentEvent::StartResource((_new_instance_id, resource_req_rcvd)) => {
                assert_eq!("rc-1", resource_req_rcvd.class_type);
                assert_eq!(another_node, node_id);
            }
            MockAgentEvent::StopResource(_new_instance_id) => {
                assert_eq!(old_node, node_id);
            }
            MockAgentEvent::PatchResource(_patch_request) => {
                num_patches += 1;
            }
            _ => panic!("unexpected event"),
        }
    }
    assert_eq!(3, num_patches);

    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
async fn test_orc_invalid_migration() {
    // Deploy the following workflow:
    //
    // f0 <--> f1 <--> f2 <--> r0 <--> r1 <--> r2
    //
    // Two nodes:
    // - one node has a suitable function run-time and resource
    // - another node does not have them
    //
    // In this test we try to migrate to the node with insufficient resources

    // Setup nodes

    let mut nodes = std::collections::HashMap::new();
    let mut client_descs_resources = std::collections::HashMap::new();
    let mut node_ids = vec![];
    for i in 0..=1 {
        let (mock_node_sender, mock_node_receiver) = futures::channel::mpsc::unbounded::<MockAgentEvent>();
        let node_id = uuid::Uuid::new_v4();
        node_ids.push(node_id);
        let mut capabilities = edgeless_api::node_registration::NodeCapabilities::minimum();
        if i == 1 {
            capabilities.runtimes.clear();
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
        if i == 0 {
            resources.push(edgeless_api::node_registration::ResourceProviderSpecification {
                provider_id: "provider-1".to_string(),
                class_type: "rc-1".to_string(),
                outputs: vec![],
            });
        }

        client_descs_resources.insert(node_id, (client_desc, resources));
    }
    assert_eq!(2, nodes.len());
    assert_eq!(2, client_descs_resources.len());

    let good_node_id = *node_ids.first().unwrap();
    let bad_node_id = *node_ids.last().unwrap();

    let (subscriber_sender, _subscriber_receiver) = futures::channel::mpsc::unbounded();

    let proxy = std::sync::Arc::new(tokio::sync::Mutex::new(proxy_test::ProxyTest::default()));
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

    let mut fun_client = orchestrator.get_api_client().function_instance_api();
    let mut res_client = orchestrator.get_api_client().resource_configuration_api();
    let mut orc_sender = orchestrator.get_sender();

    clear_events(&mut nodes).await;

    // Spawn the function instances.
    let mut function_lids = vec![];
    let mut function_pids = vec![];
    let mut function_nodes = vec![];
    for i in 0..=2 {
        let spawn_req = make_spawn_function_request(format!("fc-{}", i).as_str());
        function_lids.push(match fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });

        if let (node_id, MockAgentEvent::StartFunction((new_instance_id, _spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
            assert_eq!(node_id, new_instance_id.node_id);
            function_nodes.push(node_id);
            function_pids.push(new_instance_id.function_id);
        } else {
            panic!("wrong event received");
        }
    }
    for assigned in function_nodes {
        assert_eq!(good_node_id, assigned);
    }

    // Spawn the resource instances.
    let mut resource_lids = vec![];
    let mut resource_pids = vec![];
    let mut resource_nodes = vec![];
    for _ in 0..=2 {
        let resource_req = make_start_resource_request("rc-1");
        resource_lids.push(match res_client.start(resource_req).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });

        if let (node_id, MockAgentEvent::StartResource((new_instance_id, _resource_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
            assert_eq!(node_id, new_instance_id.node_id);
            resource_nodes.push(node_id);
            resource_pids.push(new_instance_id.function_id);
        } else {
            panic!("wrong event received");
        }
    }
    for assigned in resource_nodes {
        assert_eq!(good_node_id, assigned);
    }

    // Gotta patch 'em all.
    let patch_requests = vec![
        edgeless_api::common::PatchRequest {
            function_id: function_lids[0],
            output_mapping: std::collections::HashMap::from([(
                "out-1".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: function_lids[1],
                },
            )]),
        },
        edgeless_api::common::PatchRequest {
            function_id: function_lids[1],
            output_mapping: std::collections::HashMap::from([
                (
                    "out-1".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: function_lids[0],
                    },
                ),
                (
                    "out-2".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: function_lids[2],
                    },
                ),
            ]),
        },
        edgeless_api::common::PatchRequest {
            function_id: function_lids[2],
            output_mapping: std::collections::HashMap::from([
                (
                    "out-1".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: function_lids[1],
                    },
                ),
                (
                    "out-2".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: resource_lids[0],
                    },
                ),
            ]),
        },
        edgeless_api::common::PatchRequest {
            function_id: resource_lids[0],
            output_mapping: std::collections::HashMap::from([
                (
                    "out-1".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: function_lids[2],
                    },
                ),
                (
                    "out-2".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: resource_lids[1],
                    },
                ),
            ]),
        },
        edgeless_api::common::PatchRequest {
            function_id: resource_lids[1],
            output_mapping: std::collections::HashMap::from([
                (
                    "out-1".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: resource_lids[0],
                    },
                ),
                (
                    "out-2".to_string(),
                    edgeless_api::function_instance::InstanceId {
                        node_id: uuid::Uuid::nil(),
                        function_id: resource_lids[2],
                    },
                ),
            ]),
        },
        edgeless_api::common::PatchRequest {
            function_id: resource_lids[2],
            output_mapping: std::collections::HashMap::from([(
                "out-1".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: resource_lids[1],
                },
            )]),
        },
    ];
    assert_eq!(6, patch_requests.len());

    for (i, patch_request) in patch_requests.iter().enumerate() {
        if i < 3 {
            match fun_client.patch(patch_request.clone()).await {
                Ok(_) => {}
                Err(err) => {
                    panic!("{}", err);
                }
            };
            if let (node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut nodes).await {
                assert_eq!(good_node_id, node_id);
                assert_eq!(patch_request.function_id, function_pids[i]);
            } else {
                panic!("wrong event received");
            }
        } else {
            match res_client.patch(patch_request.clone()).await {
                Ok(_) => {}
                Err(err) => {
                    panic!("{}", err);
                }
            };
            if let (node_id, MockAgentEvent::PatchResource(patch_request)) = wait_for_event_multiple(&mut nodes).await {
                assert_eq!(good_node_id, node_id);
                assert_eq!(patch_request.function_id, resource_pids[i - 3]);
            } else {
                panic!("wrong event received");
            }
        }
    }

    // Migrate functions to the good node.
    for function_lid in &function_lids {
        proxy
            .lock()
            .await
            .add_deploy_intents(vec![deploy_intent::DeployIntent::Migrate(*function_lid, vec![good_node_id])]);

        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<()>();
        let _ = orc_sender.send(OrchestratorRequest::Refresh(reply_sender)).await;
        let _ = reply_receiver.await;

        no_function_event(&mut nodes).await;
    }

    // Migrate resources to the good node.
    for resource_lid in &resource_lids {
        proxy
            .lock()
            .await
            .add_deploy_intents(vec![deploy_intent::DeployIntent::Migrate(*resource_lid, vec![good_node_id])]);

        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<()>();
        let _ = orc_sender.send(OrchestratorRequest::Refresh(reply_sender)).await;
        let _ = reply_receiver.await;
        no_function_event(&mut nodes).await;
    }

    // Migrate functions to the bad node.
    for function_lid in &function_lids {
        proxy
            .lock()
            .await
            .add_deploy_intents(vec![deploy_intent::DeployIntent::Migrate(*function_lid, vec![bad_node_id])]);

        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<()>();
        let _ = orc_sender.send(OrchestratorRequest::Refresh(reply_sender)).await;
        let _ = reply_receiver.await;

        no_function_event(&mut nodes).await;
    }

    // Migrate resources to the bad node.
    for resource_lid in &resource_lids {
        proxy
            .lock()
            .await
            .add_deploy_intents(vec![deploy_intent::DeployIntent::Migrate(*resource_lid, vec![bad_node_id])]);

        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<()>();
        let _ = orc_sender.send(OrchestratorRequest::Refresh(reply_sender)).await;
        let _ = reply_receiver.await;
        no_function_event(&mut nodes).await;
    }
}

#[tokio::test]
async fn test_orc_reset() {
    let num_nodes = 3;
    let num_workflows = 100;
    let mut setup = setup(num_nodes, 1).await;
    assert_eq!(num_nodes, setup.nodes.len() as u32);

    // Start 10 workflows:
    //
    // f1 -> f2 -> res

    for _wf_id in 0..num_workflows {
        // Start f1
        let spawn_req = make_spawn_function_request("f1");
        let lid_1 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        };

        // Start f2
        let spawn_req = make_spawn_function_request("f2");
        let lid_2 = match setup.fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        };

        // Start r1
        let start_req = make_start_resource_request("rc-1");
        let lid_res = match setup.res_client.start(start_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        };

        // Patch f1->f2
        let mut output_mapping = std::collections::HashMap::new();
        output_mapping.insert(
            "out".to_string(),
            edgeless_api::function_instance::InstanceId {
                node_id: uuid::Uuid::nil(),
                function_id: lid_2,
            },
        );
        setup
            .fun_client
            .patch(edgeless_api::common::PatchRequest {
                function_id: lid_1,
                output_mapping,
            })
            .await
            .expect("Could not patch");

        // Patch f2->res
        let mut output_mapping = std::collections::HashMap::new();
        output_mapping.insert(
            "out".to_string(),
            edgeless_api::function_instance::InstanceId {
                node_id: uuid::Uuid::nil(),
                function_id: lid_res,
            },
        );
        setup
            .fun_client
            .patch(edgeless_api::common::PatchRequest {
                function_id: lid_2,
                output_mapping,
            })
            .await
            .expect("Could not patch");
    }

    // Make sure there are no pending events around.
    clear_events(&mut setup.nodes).await;

    // Send a Reset to the orchestrator.
    let _ = setup.orc_sender.send(OrchestratorRequest::Reset()).await;

    // Disconnect the unstable
    let mut num_events = std::collections::HashMap::new();
    while let Some((_node_id, event)) = wait_for_events_if_any(&mut setup.nodes).await {
        *num_events.entry(event_to_string(&event)).or_insert(0) += 1;
    }

    assert!(num_events.remove("patch-function").expect("No patch event found, that's very unlikely") <= num_workflows);

    let mut expected_events = std::collections::HashMap::new();
    expected_events.insert("stop-function", 2 * num_workflows);
    expected_events.insert("stop-resource", num_workflows);
    assert_eq!(expected_events, num_events);

    // Ensure that there's no pending event.
    no_function_event(&mut setup.nodes).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_orc_update_domain_capabilities() {
    let num_nodes = 10;
    let num_resources = 5;
    let mut setup = setup(num_nodes, num_resources).await;

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut num_events = 0;
    let mut expected_caps = edgeless_api::domain_registration::DomainCapabilities::default();
    expected_caps.num_nodes = num_nodes;
    expected_caps.num_cpus = num_nodes;
    expected_caps.num_cores = num_nodes;
    expected_caps.labels.insert("stable".to_string());
    expected_caps.labels.insert("unstable".to_string());
    expected_caps.runtimes.insert("RUST_WASM".to_string());
    for node_id in 0..num_nodes {
        for res_id in 0..num_resources {
            expected_caps
                .resource_providers
                .insert(format!("node-{}-resource-{}-provider", node_id, res_id));
        }
    }
    expected_caps.resource_classes.insert("rc-1".to_string());
    let mut last_caps = edgeless_api::domain_registration::DomainCapabilities::default();
    while let Ok(event) = setup.subscriber_receiver.try_next() {
        match event {
            Some(event) => match event {
                DomainSubscriberRequest::Update(actual_caps) => {
                    last_caps = *actual_caps;
                    num_events += 1;
                }
                DomainSubscriberRequest::RegisterOrcSender(_) => {}
                DomainSubscriberRequest::Refresh() => {
                    panic!("unexpected refresh event received");
                }
            },
            None => break,
        }
    }
    assert_eq!(expected_caps, last_caps);

    assert_eq!(11, num_events);
}

#[test]
fn test_orc_deployment_requirements() {
    let no_reqs = DeploymentRequirements::none();

    let empty_annotations = std::collections::HashMap::new();
    assert_eq!(no_reqs, DeploymentRequirements::from_annotations(&empty_annotations));

    let irrelevant_annotations =
        std::collections::HashMap::from([("foo".to_string(), "bar".to_string()), ("mickey".to_string(), "mouse".to_string())]);
    assert_eq!(no_reqs, DeploymentRequirements::from_annotations(&irrelevant_annotations));

    let uuid1 = uuid::Uuid::new_v4();
    let uuid2 = uuid::Uuid::new_v4();
    let valid_annotations = std::collections::HashMap::from([
        ("max_instances".to_string(), "42".to_string()),
        ("node_id_match_any".to_string(), format!("{},{}", uuid1, uuid2)),
        ("label_match_all".to_string(), "red,blue".to_string()),
        ("resource_match_all".to_string(), "file,redis".to_string()),
        ("tee".to_string(), "REQuired".to_string()),
        ("tpm".to_string(), "required".to_string()),
    ]);
    let reqs = DeploymentRequirements::from_annotations(&valid_annotations);
    assert_eq!(42, reqs.max_instances);
    assert_eq!(vec![uuid1, uuid2], reqs.node_id_match_any);
    assert_eq!(vec!["red".to_string(), "blue".to_string()], reqs.label_match_all);
    assert_eq!(vec!["file".to_string(), "redis".to_string()], reqs.resource_match_all);
    assert!(std::mem::discriminant(&AffinityLevel::Required) == std::mem::discriminant(&reqs.tee));
    assert!(std::mem::discriminant(&AffinityLevel::Required) == std::mem::discriminant(&reqs.tpm));
}

#[test]
fn test_orc_feasible_nodes() {
    let mut logic = crate::orchestration_logic::OrchestrationLogic::new(crate::OrchestrationStrategy::Random);

    // No nodes
    let mut fun1_req = make_spawn_function_request("fun");

    assert!(logic.feasible_nodes(&fun1_req, &vec![]).is_empty());
    assert!(logic
        .feasible_nodes(&fun1_req, &vec![uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), uuid::Uuid::new_v4()])
        .is_empty());

    // Add nodes
    let (nodes, mut client_descs_resources, _stable_node_id) = create_clients_resources(5, 0);

    let mut client_descs = std::collections::HashMap::new();
    for node_id in nodes.keys() {
        let client_desc = client_descs_resources.remove(&node_id).unwrap().0;
        client_descs.insert(*node_id, client_desc);
    }

    logic.update_nodes(&client_descs, &std::collections::HashMap::new());
    let all_nodes = client_descs.keys().cloned().collect::<Vec<uuid::Uuid>>();
    assert!(client_descs.len() == 5);

    // No annotations, all nodes are good
    assert_eq!(5, logic.feasible_nodes(&fun1_req, &all_nodes).len());

    // Pin-point to a node.
    fun1_req
        .annotations
        .insert("node_id_match_any".to_string(), all_nodes.first().unwrap().to_string());

    // Wrong run-time
    fun1_req.spec.function_type = "non-existing-runtime".to_string();
    assert!(logic.feasible_nodes(&fun1_req, &all_nodes).is_empty());
}
