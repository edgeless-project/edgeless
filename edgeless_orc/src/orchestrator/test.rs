// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#![allow(clippy::all)]

use crate::affinity_level::AffinityLevel;
use crate::deployment_requirements::DeploymentRequirements;
use crate::domain_subscriber::DomainSubscriberRequest;
use edgeless_api::function_instance::{FunctionClassSpecification, StatePolicy, StateSpecification};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};

use super::*;

enum MockAgentEvent {
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

struct MockNode {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockAgentEvent>,
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
fn test_create_clients_resources(
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

async fn test_setup(
    num_nodes: u32,
    num_resources_per_node: u32,
) -> (
    Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
    Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::DomainManagedInstanceId>>,
    std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>,
    uuid::Uuid,
    UnboundedReceiver<DomainSubscriberRequest>,
    UnboundedSender<OrchestratorRequest>,
) {
    let (mut nodes, client_descs_resources, stable_node_id) = test_create_clients_resources(num_nodes, num_resources_per_node);
    let (subscriber_sender, subscriber_receiver) = futures::channel::mpsc::unbounded();

    let (mut orchestrator, orchestrator_task, _refresh_task) = Orchestrator::new(
        crate::EdgelessOrcBaselineSettings {
            orchestration_strategy: crate::OrchestrationStrategy::Random,
        },
        std::sync::Arc::new(tokio::sync::Mutex::new(crate::proxy_none::ProxyNone {})),
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

    (
        orchestrator.get_api_client().function_instance_api(),
        orchestrator.get_api_client().resource_configuration_api(),
        nodes,
        stable_node_id,
        subscriber_receiver,
        orchestrator.get_sender(),
    )
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
        instance_id: None,
        code: FunctionClassSpecification {
            function_class_id: class_id.to_string(),
            function_class_type: "RUST_WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_code: "function_code".as_bytes().to_vec(),
            function_class_outputs: vec![],
        },
        annotations: std::collections::HashMap::new(),
        state_specification: StateSpecification {
            state_id: uuid::Uuid::new_v4(),
            state_policy: StatePolicy::NodeLocal,
        },
    }
}

fn make_start_resource_request(class_type: &str) -> edgeless_api::resource_configuration::ResourceInstanceSpecification {
    edgeless_api::resource_configuration::ResourceInstanceSpecification {
        class_type: class_type.to_string(),
        output_mapping: std::collections::HashMap::new(),
        configuration: std::collections::HashMap::new(),
    }
}

#[tokio::test]
async fn test_orc_single_node_function_start_stop() {
    let (mut fun_client, mut _res_client, mut nodes, _, _, _orc_sender) = test_setup(1, 0).await;
    assert_eq!(1, nodes.len());
    let (node_id, mock_node_receiver) = nodes.iter_mut().next().unwrap();
    assert!(!node_id.is_nil());

    assert!(mock_node_receiver.try_next().is_err());

    // Start a function.

    let spawn_req = make_spawn_function_request("fc-1");
    let instance_id = match fun_client.start(spawn_req.clone()).await.unwrap() {
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

    match fun_client.stop(instance_id).await {
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
    match fun_client.stop(instance_id).await {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }
    no_function_event(&mut nodes).await;
}

#[tokio::test]
async fn test_orc_multiple_nodes_function_start_stop() {
    let (mut fun_client, mut _res_client, mut nodes, _, _, _orc_sender) = test_setup(3, 0).await;
    assert_eq!(3, nodes.len());

    // Start 100 functions.

    let mut ext_instance_ids = vec![];
    let mut int_instance_ids = vec![];
    let mut node_ids = vec![];
    for i in 0..100 {
        let spawn_req = make_spawn_function_request(format!("fc-{}", i).as_str());
        ext_instance_ids.push(match fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });

        if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
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
        match fun_client.stop(ext_instance_ids[i]).await {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }

        if let (node_id, MockAgentEvent::StopFunction(instance_id_rcvd)) = wait_for_event_multiple(&mut nodes).await {
            assert_eq!(node_ids[i], node_id);
            assert_eq!(int_instance_ids[i], instance_id_rcvd);
        } else {
            panic!("wrong event received");
        }
    }
}

#[tokio::test]
async fn test_orc_multiple_resources_start_stop() {
    let (mut _fun_client, mut res_client, mut nodes, _, _, _orc_sender) = test_setup(3, 3).await;
    assert_eq!(3, nodes.len());

    // Start 100 resources.

    let mut ext_instance_ids = vec![];
    let mut int_instance_ids = vec![];
    let mut node_ids = vec![];
    for _i in 0..100 {
        let start_req = make_start_resource_request("rc-1");
        ext_instance_ids.push(match res_client.start(start_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });

        if let (node_id, MockAgentEvent::StartResource((int_instance_id, resource_instance_spec))) = wait_for_event_multiple(&mut nodes).await {
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
        match res_client.stop(ext_instance_ids[i]).await {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }

        if let (node_id, MockAgentEvent::StopResource(instance_id_rcvd)) = wait_for_event_multiple(&mut nodes).await {
            assert_eq!(node_ids[i], node_id);
            assert_eq!(node_ids[i], instance_id_rcvd.node_id);
            assert_eq!(int_instance_ids[i], instance_id_rcvd.function_id);
        } else {
            panic!("wrong event received");
        }
    }

    // Start a resource with unknown class type.
    match res_client.start(make_start_resource_request("rc-666")).await.unwrap() {
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
    let (mut fun_client, mut res_client, mut nodes, _, _, _orc_sender) = test_setup(1, 1).await;
    assert_eq!(1, nodes.len());
    let client_node_id = *nodes.keys().next().unwrap();

    // Spawn a function instance.

    let spawn_req = make_spawn_function_request("fc-1");
    let ext_function_id = match fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };

    let mut int_function_id = None;
    assert!(int_function_id.is_none());
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
        assert_eq!(client_node_id, node_id);
        assert_eq!(client_node_id, new_instance_id.node_id);
        int_function_id = Some(new_instance_id);
        assert_eq!(spawn_req, spawn_req_rcvd);
    } else {
        panic!("wrong event received");
    }

    // Start a resource.

    let start_req = make_start_resource_request("rc-1");
    let ext_resource_id = match res_client.start(start_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };

    let mut int_resource_id = None;
    assert!(int_resource_id.is_none());
    if let (node_id, MockAgentEvent::StartResource((new_instance_id, resource_instance_spec))) = wait_for_event_multiple(&mut nodes).await {
        assert_eq!(client_node_id, node_id);
        assert_eq!(client_node_id, new_instance_id.node_id);
        int_resource_id = Some(new_instance_id);
        assert!(resource_instance_spec.configuration.is_empty());
    } else {
        panic!("wrong event received");
    }

    // Gotta patch 'em all.

    match fun_client
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

    if let (node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut nodes).await {
        assert_eq!(client_node_id, node_id);
        assert_eq!(int_function_id.unwrap().function_id, patch_request.function_id);
        assert_eq!(1, patch_request.output_mapping.len());
        let mapping = patch_request.output_mapping.get("out-1");
        assert!(mapping.is_some());
        assert_eq!(int_resource_id.unwrap(), mapping.unwrap().clone());
    } else {
        panic!("wrong event received");
    }

    match res_client
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

    if let (node_id, MockAgentEvent::PatchResource(patch_request)) = wait_for_event_multiple(&mut nodes).await {
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
    let (mut fun_client, mut _res_client, mut nodes, stable_node_id, _, mut orc_sender) = test_setup(10, 0).await;
    assert_eq!(10, nodes.len());

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
    spawn_req.annotations.insert("node_id_match_any".to_string(), stable_node_id.to_string());
    let ext_fid_1 = match fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut int_fid_1 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
        assert_eq!(node_id, stable_node_id);
        int_fid_1 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f2
    let mut spawn_req = make_spawn_function_request("f2");
    spawn_req.annotations.insert("label_match_all".to_string(), "unstable".to_string());
    let ext_fid_2 = match fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut unstable_node_id = uuid::Uuid::nil();
    let mut int_fid_2 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
        assert_ne!(node_id, stable_node_id);
        unstable_node_id = node_id;
        int_fid_2 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f3
    let mut spawn_req = make_spawn_function_request("f3");
    spawn_req.annotations.insert("node_id_match_any".to_string(), stable_node_id.to_string());
    let ext_fid_3 = match fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut int_fid_3 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
        assert_eq!(node_id, stable_node_id);
        int_fid_3 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f4
    let mut spawn_req = make_spawn_function_request("f4");
    spawn_req.annotations.insert("node_id_match_any".to_string(), stable_node_id.to_string());
    let ext_fid_4 = match fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut _int_fid_4 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
        assert_eq!(node_id, stable_node_id);
        _int_fid_4 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Patch f1->f2
    match fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: ext_fid_1,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_2,
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
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Patch f2->f3
    match fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: ext_fid_2,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_3,
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
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Patch f3->f4
    match fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: ext_fid_3,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_4,
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

    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Make sure there are no pending events around.
    no_function_event(&mut nodes).await;

    // Disconnect the unstable node
    let _ = orc_sender.send(OrchestratorRequest::DelNode(unstable_node_id)).await;

    let mut num_events = std::collections::HashMap::new();
    let mut new_node_id = uuid::Uuid::nil();
    let mut patch_request_1 = None;
    let mut patch_request_2 = None;
    loop {
        if let Some((node_id, event)) = wait_for_events_if_any(&mut nodes).await {
            if num_events.contains_key(event_to_string(&event)) {
                *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
            } else {
                num_events.insert(event_to_string(&event), 1);
            }
            match event {
                MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd)) => {
                    log::info!("start-function");
                    assert_ne!(node_id, stable_node_id);
                    assert_eq!(node_id, new_instance_id.node_id);
                    new_node_id = new_instance_id.node_id;
                    int_fid_2 = new_instance_id.function_id;
                    assert_eq!("f2", spawn_req_rcvd.code.function_class_id);
                }
                MockAgentEvent::PatchFunction(patch_request) => {
                    log::info!("patch-function");
                    assert!(patch_request.output_mapping.contains_key("out"));
                    if node_id == stable_node_id {
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
    assert_eq!(int_fid_1, patch_request_1.function_id);
    assert_eq!(int_fid_2, patch_request_1.output_mapping.get("out").unwrap().function_id);
    assert_eq!(int_fid_2, patch_request_2.function_id);
    assert_eq!(int_fid_3, patch_request_2.output_mapping.get("out").unwrap().function_id);

    no_function_event(&mut nodes).await;
}

#[tokio::test]
async fn orc_node_with_res_disconnects() {
    let (mut fun_client, mut res_client, mut nodes, stable_node_id, _, mut orc_sender) = test_setup(10, 1).await;
    assert_eq!(10, nodes.len());

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
    spawn_req.annotations.insert("node_id_match_any".to_string(), stable_node_id.to_string());
    let ext_fid_1 = match fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut int_fid_1 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
        assert_eq!(node_id, stable_node_id);
        int_fid_1 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start r1
    let start_req = make_start_resource_request("rc-1");

    let mut unstable_node_id = uuid::Uuid::nil();
    let mut int_fid_res = uuid::Uuid::nil();
    let mut ext_fid_res = uuid::Uuid::nil();
    assert!(ext_fid_res.is_nil());
    loop {
        ext_fid_res = match res_client.start(start_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        };

        if let (node_id, MockAgentEvent::StartResource((int_instance_id, resource_instance_spec))) = wait_for_event_multiple(&mut nodes).await {
            assert_eq!(node_id, int_instance_id.node_id);
            unstable_node_id = int_instance_id.node_id;
            int_fid_res = int_instance_id.function_id;
            assert!(resource_instance_spec.configuration.is_empty());
            if int_instance_id.node_id != stable_node_id {
                break;
            }
        }

        // If we reach this point then the stable node has been selected,
        // so we stop the resource and try again.
        match res_client.stop(ext_fid_res).await {
            Ok(_) => {}
            Err(err) => panic!("{}", err),
        }

        if let (node_id, MockAgentEvent::StopResource(instance_id_rcvd)) = wait_for_event_multiple(&mut nodes).await {
            assert_eq!(unstable_node_id, node_id);
            assert_eq!(unstable_node_id, instance_id_rcvd.node_id);
            assert_eq!(int_fid_res, instance_id_rcvd.function_id);
        }
    }
    assert!(!unstable_node_id.is_nil());
    assert!(!int_fid_res.is_nil());
    assert!(!ext_fid_res.is_nil());

    // Patch f1->res
    match fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: ext_fid_1,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_res,
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
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
        assert_eq!(unstable_node_id, patch_request.output_mapping.get("out").unwrap().node_id);
        assert_eq!(int_fid_res, patch_request.output_mapping.get("out").unwrap().function_id);
    }

    // Make sure there are no pending events around.
    no_function_event(&mut nodes).await;

    // Disconnect the unstable node
    let _ = orc_sender.send(OrchestratorRequest::DelNode(unstable_node_id)).await;

    let mut num_events = std::collections::HashMap::new();
    let mut new_node_id = uuid::Uuid::nil();
    let mut patch_request_rcv: Option<edgeless_api::common::PatchRequest> = None;
    loop {
        if let Some((node_id, event)) = wait_for_events_if_any(&mut nodes).await {
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
                    int_fid_res = new_instance_id.function_id;
                }
                MockAgentEvent::PatchFunction(patch_request) => {
                    log::info!("patch-function");
                    assert!(patch_request.output_mapping.contains_key("out"));
                    assert_eq!(stable_node_id, node_id);
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
    assert_eq!(int_fid_1, patch_request_rcv.function_id);
    assert_eq!(int_fid_res, patch_request_rcv.output_mapping.get("out").unwrap().function_id);

    no_function_event(&mut nodes).await;
}

#[tokio::test]
async fn test_patch_after_fun_stop() {
    let (mut fun_client, mut _res_client, mut nodes, _stable_node_id, _, _orc_sender) = test_setup(10, 0).await;
    assert_eq!(10, nodes.len());

    // Start this workflow
    //
    // f1 -> f3 -> f4 -> f6
    //     /   \        /
    // f2 /     \  f5 /
    //
    // then stop f3

    // Start functions
    let mut ext_fids = vec![];
    let mut int_fids = vec![];
    for i in 1..=6 {
        let spawn_req = make_spawn_function_request(format!("f{}", i).as_str());
        ext_fids.push(match fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });
        if let (_node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
            int_fids.push(new_instance_id.function_id);
            assert_eq!(spawn_req, spawn_req_rcvd);
        }
    }
    assert_eq!(6, ext_fids.len());
    assert_eq!(6, int_fids.len());

    // Patch functions
    let patch_instructions = [
        (ext_fids[0], vec![ext_fids[2]]),
        (ext_fids[1], vec![ext_fids[2]]),
        (ext_fids[2], vec![ext_fids[3], ext_fids[4]]),
        (ext_fids[3], vec![ext_fids[5]]),
        (ext_fids[4], vec![ext_fids[5]]),
    ];
    let patch_instructions_int = [
        (int_fids[0], vec![int_fids[2]]),
        (int_fids[1], vec![int_fids[2]]),
        (int_fids[2], vec![int_fids[3], int_fids[4]]),
        (int_fids[3], vec![int_fids[5]]),
        (int_fids[4], vec![int_fids[5]]),
    ];

    for j in 0..patch_instructions.len() {
        let ext_fid_pair = &patch_instructions[j];
        let mut output_mapping = std::collections::HashMap::new();
        for i in 0..ext_fid_pair.1.len() {
            output_mapping.insert(
                format!("out{}", i),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_pair.1[i],
                },
            );
        }
        match fun_client
            .patch(edgeless_api::common::PatchRequest {
                function_id: ext_fid_pair.0,
                output_mapping,
            })
            .await
        {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        };
        if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut nodes).await {
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
    no_function_event(&mut nodes).await;

    // Stop function f3
    match fun_client.stop(ext_fids[2]).await {
        Ok(_) => {}
        Err(err) => panic!("{}", err),
    }

    let mut num_events = std::collections::HashMap::new();
    loop {
        if let Some((_node_id, event)) = wait_for_events_if_any(&mut nodes).await {
            if num_events.contains_key(event_to_string(&event)) {
                *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
            } else {
                num_events.insert(event_to_string(&event), 1);
            }
            match event {
                MockAgentEvent::StopFunction(instance_id) => {
                    log::info!("stop-resource");
                    assert_eq!(int_fids[2], instance_id.function_id);
                }
                MockAgentEvent::PatchFunction(patch_request) => {
                    log::info!("patch-function");
                    assert!(patch_request.function_id == int_fids[0] || patch_request.function_id == int_fids[1]);
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
    no_function_event(&mut nodes).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_recreate_fun_after_disconnect() {
    let (mut fun_client, mut _res_client, mut nodes, stable_node_id, _, mut orc_sender) = test_setup(2, 0).await;
    assert_eq!(2, nodes.len());

    // Start this workflow
    //
    // f1 -> f2 -> f3
    //
    // f1, f3 -> stable node
    // f2 -> unstable node which disconnects, then reconnects
    //

    // Start f1
    let mut spawn_req = make_spawn_function_request("f1");
    spawn_req.annotations.insert("node_id_match_any".to_string(), stable_node_id.to_string());
    let ext_fid_1 = match fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut int_fid_1 = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
        assert_eq!(node_id, stable_node_id);
        int_fid_1 = new_instance_id.function_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f2
    let mut spawn_req = make_spawn_function_request("f2");
    spawn_req.annotations.insert("label_match_all".to_string(), "unstable".to_string());
    let ext_fid_2 = match fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    let mut unstable_node_id = uuid::Uuid::nil();
    if let (node_id, MockAgentEvent::StartFunction((_new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
        assert_ne!(node_id, stable_node_id);
        unstable_node_id = node_id;
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Start f3
    let mut spawn_req = make_spawn_function_request("f3");
    spawn_req.annotations.insert("node_id_match_any".to_string(), stable_node_id.to_string());
    let ext_fid_3 = match fun_client.start(spawn_req.clone()).await.unwrap() {
        edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
        edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    if let (node_id, MockAgentEvent::StartFunction((_new_instance_id, spawn_req_rcvd))) = wait_for_event_multiple(&mut nodes).await {
        assert_eq!(node_id, stable_node_id);
        assert_eq!(spawn_req, spawn_req_rcvd);
    }

    // Patch f1->f2
    match fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: ext_fid_1,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_2,
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
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Patch f2->f3
    match fun_client
        .patch(edgeless_api::common::PatchRequest {
            function_id: ext_fid_2,
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_3,
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
    if let (_node_id, MockAgentEvent::PatchFunction(patch_request)) = wait_for_event_multiple(&mut nodes).await {
        assert!(patch_request.output_mapping.contains_key("out"));
    }

    // Make sure there are no pending events around.
    no_function_event(&mut nodes).await;

    // Disconnect the unstable node
    let _ = orc_sender.send(OrchestratorRequest::DelNode(unstable_node_id)).await;

    let mut num_events = std::collections::HashMap::new();
    loop {
        if let Some((_node_id, event)) = wait_for_events_if_any(&mut nodes).await {
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
                    assert_eq!(int_fid_1, patch_request.function_id);
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
        let _ = orc_sender.send(OrchestratorRequest::Refresh(reply_sender)).await;
        let _ = reply_receiver.await;

        let mut num_events = std::collections::HashMap::new();
        loop {
            if let Some((_node_id, event)) = wait_for_events_if_any(&mut nodes).await {
                if num_events.contains_key(event_to_string(&event)) {
                    *num_events.get_mut(event_to_string(&event)).unwrap() += 1;
                } else {
                    num_events.insert(event_to_string(&event), 1);
                }
                match event {
                    MockAgentEvent::PatchFunction(patch_request) => {
                        assert_eq!(int_fid_1, patch_request.function_id);
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
    no_function_event(&mut nodes).await;

    // Re-create the unstable node.

    let (mock_node_sender, mock_node_receiver) = futures::channel::mpsc::unbounded::<MockAgentEvent>();
    let mut capabilities = edgeless_api::node_registration::NodeCapabilities::minimum();
    capabilities.labels.push("unstable".to_string());

    if let Some(val) = nodes.get_mut(&unstable_node_id) {
        *val = mock_node_receiver;
    }

    let _ = orc_sender
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
            },
            vec![],
        ))
        .await;

    if let Some(entry) = nodes.get_mut(&stable_node_id) {
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
    if let Some(entry) = nodes.get_mut(&unstable_node_id) {
        let mut num_update_peers = 0;
        let mut num_reset = 0;
        for _ in 0..5 {
            let event = wait_for_event_at_node(entry).await;
            match event {
                MockAgentEvent::StartFunction((_new_instance_id, spawn_req_rcvd)) => {
                    log::info!("{:?}", spawn_req_rcvd);
                    assert_eq!("f2", spawn_req_rcvd.code.function_class_id);
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

    no_function_event(&mut nodes).await;
}

#[tokio::test]
async fn orc_reset() {
    let num_nodes = 3;
    let num_workflows = 100;
    let (mut fun_client, mut res_client, mut nodes, _stable_node_id, _, mut orc_sender) = test_setup(num_nodes, 1).await;
    assert_eq!(num_nodes, nodes.len() as u32);

    // Start 10 workflows:
    //
    // f1 -> f2 -> res

    for _wf_id in 0..num_workflows {
        // Start f1
        let spawn_req = make_spawn_function_request("f1");
        let ext_fid_1 = match fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        };

        // Start f2
        let spawn_req = make_spawn_function_request("f2");
        let ext_fid_2 = match fun_client.start(spawn_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        };

        // Start r1
        let start_req = make_start_resource_request("rc-1");
        let ext_fid_res = match res_client.start(start_req.clone()).await.unwrap() {
            edgeless_api::common::StartComponentResponse::InstanceId(id) => id,
            edgeless_api::common::StartComponentResponse::ResponseError(err) => panic!("{}", err),
        };

        // Patch f1->f2
        let mut output_mapping = std::collections::HashMap::new();
        output_mapping.insert(
            "out".to_string(),
            edgeless_api::function_instance::InstanceId {
                node_id: uuid::Uuid::nil(),
                function_id: ext_fid_2,
            },
        );
        fun_client
            .patch(edgeless_api::common::PatchRequest {
                function_id: ext_fid_1,
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
                function_id: ext_fid_res,
            },
        );
        fun_client
            .patch(edgeless_api::common::PatchRequest {
                function_id: ext_fid_2,
                output_mapping,
            })
            .await
            .expect("Could not patch");
    }

    // Make sure there are no pending events around.
    clear_events(&mut nodes).await;

    // Send a Reset to the orchestrator.
    let _ = orc_sender.send(OrchestratorRequest::Reset()).await;

    // Disconnect the unstable
    let mut num_events = std::collections::HashMap::new();
    while let Some((_node_id, event)) = wait_for_events_if_any(&mut nodes).await {
        *num_events.entry(event_to_string(&event)).or_insert(0) += 1;
    }

    assert!(num_events.remove("patch-function").expect("No patch event found, that's very unlikely") <= num_workflows);

    let mut expected_events = std::collections::HashMap::new();
    expected_events.insert("stop-function", 2 * num_workflows);
    expected_events.insert("stop-resource", num_workflows);
    assert_eq!(expected_events, num_events);

    // Ensure that there's no pending event.
    no_function_event(&mut nodes).await;
}

#[tokio::test]
#[serial_test::serial]
async fn test_update_domain_capabilities() {
    let num_nodes = 10;
    let num_resources = 5;
    let (_fun_client, mut _res_client, _nodes, _stable_node_id, mut subscriber_receiver, _orc_sender) = test_setup(num_nodes, num_resources).await;

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
    while let Ok(event) = subscriber_receiver.try_next() {
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

    assert_eq!(10, num_events);
}

#[test]
fn test_deployment_requirements() {
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
fn test_orchestration_logic_is_node_feasible() {
    let node_id = uuid::Uuid::new_v4();
    let mut reqs = DeploymentRequirements::none();
    let mut caps = edgeless_api::node_registration::NodeCapabilities::minimum();
    let mut providers = std::collections::HashSet::new();
    let mut runtime = "RUST_WASM".to_string();

    // Empty requirements
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    // Match any node_id
    reqs.node_id_match_any.push(node_id);
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    reqs.node_id_match_any.push(uuid::Uuid::new_v4());
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    reqs.node_id_match_any.clear();
    reqs.node_id_match_any.push(uuid::Uuid::new_v4());
    assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));
    reqs.node_id_match_any.clear();

    // Match all labels
    reqs.label_match_all.push("red".to_string());
    caps.labels.push("green".to_string());
    assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    caps.labels.push("red".to_string());
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    reqs.label_match_all.push("blue".to_string());
    assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    caps.labels.push("blue".to_string());
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    // Match all providers
    reqs.resource_match_all.push("file-1".to_string());
    assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    providers.insert("file-1".to_string());
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    providers.insert("file-2".to_string());
    providers.insert("file-3".to_string());
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    reqs.resource_match_all.push("file-9".to_string());
    assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    providers.insert("file-9".to_string());
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    // Match TEE and TPM
    reqs.tee = AffinityLevel::Required;
    assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));
    caps.is_tee_running = true;
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    reqs.tpm = AffinityLevel::Required;
    assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));
    caps.has_tpm = true;
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));

    // Match runtime
    runtime = "CONTAINER".to_string();
    assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));
    runtime = "".to_string();
    assert!(!crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));
    runtime = "RUST_WASM".to_string();
    assert!(crate::orchestration_logic::OrchestrationLogic::is_node_feasible(
        &runtime, &reqs, &node_id, &caps, &providers
    ));
}

#[test]
fn test_orchestration_feasible_nodes() {
    let mut logic = crate::orchestration_logic::OrchestrationLogic::new(crate::OrchestrationStrategy::Random);

    // No nodes
    let mut fun1_req = make_spawn_function_request("fun");

    assert!(logic.feasible_nodes(&fun1_req, &vec![]).is_empty());
    assert!(logic
        .feasible_nodes(&fun1_req, &vec![uuid::Uuid::new_v4(), uuid::Uuid::new_v4(), uuid::Uuid::new_v4()])
        .is_empty());

    // Add nodes
    let (nodes, mut client_descs_resources, _stable_node_id) = test_create_clients_resources(5, 0);

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
    fun1_req.code.function_class_type = "non-existing-runtime".to_string();
    assert!(logic.feasible_nodes(&fun1_req, &all_nodes).is_empty());
}
