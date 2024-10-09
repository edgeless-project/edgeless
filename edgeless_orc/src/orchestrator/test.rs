// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_api::function_instance::{FunctionClassSpecification, StatePolicy, StateSpecification};

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
    KeepAlive(),
    StartResource(
        (
            edgeless_api::function_instance::InstanceId,
            edgeless_api::resource_configuration::ResourceInstanceSpecification,
        ),
    ),
    StopResource(edgeless_api::function_instance::InstanceId),
    PatchResource(edgeless_api::common::PatchRequest),
}

static FAILING_NODES: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<uuid::Uuid>>> = std::sync::OnceLock::new();

struct MockNode {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockAgentEvent>,
}

impl edgeless_api::agent::AgentAPI for MockNode {
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
            node_id: self.node_id.clone(),
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
    async fn keep_alive(&mut self) -> anyhow::Result<edgeless_api::node_management::KeepAliveResponse> {
        self.sender.send(MockAgentEvent::KeepAlive()).await.unwrap();

        if FAILING_NODES.get().unwrap().lock().unwrap().contains(&self.node_id) {
            Err(anyhow::anyhow!("node {} failed", self.node_id))
        } else {
            Ok(edgeless_api::node_management::KeepAliveResponse::empty())
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for MockAgentAPI {
    async fn start(
        &mut self,
        start_request: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let new_id = edgeless_api::function_instance::InstanceId {
            node_id: self.node_id.clone(),
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

fn test_create_clients_resources(
    num_nodes: u32,
    num_resources_per_node: u32,
) -> (
    std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>,
    std::collections::HashMap<uuid::Uuid, ClientDesc>,
    std::collections::HashMap<String, ResourceProvider>,
    uuid::Uuid,
) {
    assert!(num_nodes > 0);

    let mut nodes = std::collections::HashMap::new();
    let mut clients = std::collections::HashMap::new();
    let mut resource_providers = std::collections::HashMap::new();
    let mut stable_node_id = uuid::Uuid::nil();
    for node_i in 0..num_nodes {
        let (mock_node_sender, mock_node_receiver) = futures::channel::mpsc::unbounded::<MockAgentEvent>();
        let node_id = uuid::Uuid::new_v4();
        let mut capabilities = edgeless_api::node_registration::NodeCapabilities::minimum();
        if node_i == 0 {
            stable_node_id = node_id.clone();
            capabilities.labels.push("stable".to_string());
        } else {
            capabilities.labels.push("unstable".to_string());
        }
        nodes.insert(node_id.clone(), mock_node_receiver);
        clients.insert(
            node_id.clone(),
            ClientDesc {
                agent_url: "".to_string(),
                invocation_url: "".to_string(),
                api: Box::new(MockNode {
                    node_id: node_id.clone(),
                    sender: mock_node_sender,
                }) as Box<dyn edgeless_api::agent::AgentAPI + Send>,
                capabilities,
            },
        );
        for provider_i in 0..num_resources_per_node {
            resource_providers.insert(
                format!("node-{}-resource-{}-provider", node_i, provider_i),
                ResourceProvider {
                    class_type: "rc-1".to_string(),
                    node_id: node_id.clone(),
                    outputs: vec![],
                },
            );
        }
    }

    (nodes, clients, resource_providers, stable_node_id)
}

async fn test_setup(
    num_nodes: u32,
    num_resources_per_node: u32,
) -> (
    Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::orc::DomainManagedInstanceId>>,
    Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::orc::DomainManagedInstanceId>>,
    Box<dyn edgeless_api::node_registration::NodeRegistrationAPI>,
    std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>,
    uuid::Uuid,
) {
    let (nodes, clients, resource_providers, stable_node_id) = test_create_clients_resources(num_nodes, num_resources_per_node);

    let (mut orchestrator, orchestrator_task) = Orchestrator::new_with_clients(
        crate::EdgelessOrcBaselineSettings {
            orchestration_strategy: crate::OrchestrationStrategy::Random,
            keep_alive_interval_secs: 0 as u64, // unused
        },
        clients,
        resource_providers,
    )
    .await;
    tokio::spawn(orchestrator_task);

    (
        orchestrator.get_api_client().function_instance_api(),
        orchestrator.get_api_client().resource_configuration_api(),
        orchestrator.get_api_client().node_registration_api(),
        nodes,
        stable_node_id,
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
        MockAgentEvent::KeepAlive() => "keep-alive",
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
        match receiver.try_next() {
            Ok(val) => match val {
                Some(event) => {
                    return event;
                }
                None => {}
            },
            Err(_) => {}
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
            match receiver.try_next() {
                Ok(val) => match val {
                    Some(event) => {
                        return (node_id.clone(), event);
                    }
                    None => {}
                },
                Err(_) => {}
            }
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
            match receiver.try_next() {
                Ok(val) => match val {
                    Some(event) => {
                        return Some((node_id.clone(), event));
                    }
                    None => {}
                },
                Err(_) => {}
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
    None
}

async fn no_function_event(receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    for (node_id, receiver) in receivers.iter_mut() {
        match receiver.try_next() {
            Ok(val) => match val {
                Some(event) => {
                    panic!("expecting no event, but received one on node {}: {}", node_id, event_to_string(&event));
                }
                None => {}
            },
            Err(_) => {}
        }
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
    let (mut fun_client, mut _res_client, mut _mgt_client, mut nodes, _) = test_setup(1, 0).await;
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
    let (mut fun_client, mut _res_client, mut _mgt_client, mut nodes, _) = test_setup(3, 0).await;
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
    let (mut _fun_client, mut res_client, mut _mgt_client, mut nodes, _) = test_setup(3, 3).await;
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
            int_instance_ids.push(int_instance_id.function_id.clone());
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
    let (mut fun_client, mut res_client, mut _mgt_client, mut nodes, _) = test_setup(1, 1).await;
    assert_eq!(1, nodes.len());
    let client_node_id = nodes.keys().next().unwrap().clone();

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
            function_id: ext_function_id.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out-1".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_resource_id.clone(),
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
            function_id: ext_resource_id.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out-2".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_function_id.clone(),
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
    let _ = env_logger::try_init();

    let (mut fun_client, mut _res_client, mut mgt_client, mut nodes, stable_node_id) = test_setup(10, 0).await;
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
        unstable_node_id = node_id.clone();
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
            function_id: ext_fid_1.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_2.clone(),
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
            function_id: ext_fid_2.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_3.clone(),
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
            function_id: ext_fid_3.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_4.clone(),
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
    {
        let _ = FAILING_NODES.set(std::sync::Mutex::new(std::collections::HashSet::new()));

        let mut failing_nodes = FAILING_NODES.get().unwrap().lock().unwrap();
        failing_nodes.clear();
        failing_nodes.insert(unstable_node_id.clone());
    }
    let _ = mgt_client.keep_alive().await;

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
                MockAgentEvent::KeepAlive() => {
                    log::info!("keep-alive");
                }
                _ => panic!("unexpected event type: {}", event_to_string(&event)),
            };
        } else {
            break;
        }
    }
    assert_eq!(Some(&10), num_events.get("keep-alive"));
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
    let _ = env_logger::try_init();

    let (mut fun_client, mut res_client, mut mgt_client, mut nodes, stable_node_id) = test_setup(10, 1).await;
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
            unstable_node_id = int_instance_id.node_id.clone();
            int_fid_res = int_instance_id.function_id.clone();
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
            function_id: ext_fid_1.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_res.clone(),
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
    {
        let _ = FAILING_NODES.set(std::sync::Mutex::new(std::collections::HashSet::new()));

        let mut failing_nodes = FAILING_NODES.get().unwrap().lock().unwrap();
        failing_nodes.clear();
        failing_nodes.insert(unstable_node_id.clone());
    }
    let _ = mgt_client.keep_alive().await;

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
                MockAgentEvent::KeepAlive() => {
                    log::info!("keep-alive");
                }
                _ => panic!("unexpected event type: {}", event_to_string(&event)),
            };
        } else {
            break;
        }
    }
    assert_eq!(Some(&10), num_events.get("keep-alive"));
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
    let _ = env_logger::try_init();

    let (mut fun_client, mut _res_client, mut _mgt_client, mut nodes, _stable_node_id) = test_setup(10, 0).await;
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
        (ext_fids[0].clone(), vec![ext_fids[2].clone()]),
        (ext_fids[1].clone(), vec![ext_fids[2].clone()]),
        (ext_fids[2].clone(), vec![ext_fids[3].clone(), ext_fids[4].clone()]),
        (ext_fids[3].clone(), vec![ext_fids[5].clone()]),
        (ext_fids[4].clone(), vec![ext_fids[5].clone()]),
    ];
    let patch_instructions_int = [
        (int_fids[0].clone(), vec![int_fids[2].clone()]),
        (int_fids[1].clone(), vec![int_fids[2].clone()]),
        (int_fids[2].clone(), vec![int_fids[3].clone(), int_fids[4].clone()]),
        (int_fids[3].clone(), vec![int_fids[5].clone()]),
        (int_fids[4].clone(), vec![int_fids[5].clone()]),
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
    let _ = env_logger::try_init();

    let (mut fun_client, mut _res_client, mut mgt_client, mut nodes, stable_node_id) = test_setup(2, 0).await;
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
        unstable_node_id = node_id.clone();
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
            function_id: ext_fid_1.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_2.clone(),
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
            function_id: ext_fid_2.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out".to_string(),
                edgeless_api::function_instance::InstanceId {
                    node_id: uuid::Uuid::nil(),
                    function_id: ext_fid_3.clone(),
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
    {
        let _ = FAILING_NODES.set(std::sync::Mutex::new(std::collections::HashSet::new()));

        let mut failing_nodes = FAILING_NODES.get().unwrap().lock().unwrap();
        failing_nodes.clear();
        failing_nodes.insert(unstable_node_id.clone());
    }
    let _ = mgt_client.keep_alive().await;

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
                MockAgentEvent::KeepAlive() => {
                    log::info!("keep-alive");
                }
                _ => panic!("unexpected event type: {}", event_to_string(&event)),
            };
        } else {
            break;
        }
    }
    assert_eq!(Some(&2), num_events.get("keep-alive"));
    assert_eq!(Some(&1), num_events.get("update-peers"));
    assert_eq!(Some(&1), num_events.get("patch-function"));

    // Keep alive again.
    for _ in 0..5 {
        let _ = mgt_client.keep_alive().await;

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
                        log::info!("patch-function");
                        assert_eq!(int_fid_1, patch_request.function_id);
                        assert!(patch_request.output_mapping.is_empty());
                    }
                    MockAgentEvent::KeepAlive() => {
                        log::info!("keep-alive");
                    }
                    _ => panic!("unexpected event type: {}", event_to_string(&event)),
                };
            } else {
                break;
            }
        }
        assert_eq!(Some(&1), num_events.get("keep-alive"));
        assert_eq!(Some(&1), num_events.get("patch-function"));
    }

    //
    // Test incomplete: it is not possible to (easily) restore the failed node.
    //

    // Make sure there are no pending events.
    no_function_event(&mut nodes).await;
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
        ("node_id_match_any".to_string(), format!("{},{}", uuid1.to_string(), uuid2.to_string())),
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
    reqs.node_id_match_any.push(node_id.clone());
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
    let (_nodes, clients, resource_providers, _stable_node_id) = test_create_clients_resources(5, 0);
    logic.update_nodes(&clients, &resource_providers);
    let all_nodes = clients.keys().cloned().collect::<Vec<uuid::Uuid>>();
    assert!(clients.len() == 5);

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
