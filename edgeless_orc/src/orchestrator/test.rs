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
    UpdatePeers(UpdatePeersRequest),
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

struct MockNode {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockAgentEvent>,
}

impl edgeless_api::agent::AgentAPI for MockNode {
    fn function_instance_api(
        &mut self,
    ) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::function_instance::InstanceId>> {
        Box::new(MockAgentAPI {
            node_id: self.node_id.clone(),
            sender: self.sender.clone(),
        })
    }
    fn node_management_api(&mut self) -> Box<dyn edgeless_api::node_managment::NodeManagementAPI> {
        Box::new(MockAgentAPI {
            node_id: self.node_id.clone(),
            sender: self.sender.clone(),
        })
    }
    fn resource_configuration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>> {
        Box::new(MockAgentAPI {
            node_id: self.node_id.clone(),
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
impl edgeless_api::node_managment::NodeManagementAPI for MockAgentAPI {
    async fn update_peers(&mut self, request: edgeless_api::node_managment::UpdatePeersRequest) -> anyhow::Result<()> {
        self.sender.send(MockAgentEvent::UpdatePeers(request)).await.unwrap();
        Ok(())
    }
    async fn keep_alive(&mut self) -> anyhow::Result<()> {
        self.sender.send(MockAgentEvent::KeepAlive()).await.unwrap();
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

async fn test_setup(
    num_nodes: u32,
    num_resources_per_node: u32,
) -> (
    Box<dyn edgeless_api::function_instance::FunctionInstanceAPI<edgeless_api::orc::DomainManagedInstanceId>>,
    Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::orc::DomainManagedInstanceId>>,
    Box<dyn edgeless_api::node_registration::NodeRegistrationAPI>,
    std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>>,
) {
    assert!(num_nodes > 0);

    let mut nodes = std::collections::HashMap::new();
    let mut clients = std::collections::HashMap::new();
    let mut resource_providers = std::collections::HashMap::new();
    for node_i in 0..num_nodes {
        let (mock_node_sender, mock_node_receiver) = futures::channel::mpsc::unbounded::<MockAgentEvent>();
        let node_id = uuid::Uuid::new_v4();
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

    let (mut orchestrator, orchestrator_task) = Orchestrator::new_with_clients(
        crate::EdgelessOrcSettings {
            domain_id: "".to_string(),        // unused
            orchestrator_url: "".to_string(), // unused
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
    )
}

#[allow(dead_code)]
fn event_to_string(event: MockAgentEvent) -> &'static str {
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
            Some(val) => event_to_string(val),
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

async fn no_function_event(receiver: &mut futures::channel::mpsc::UnboundedReceiver<MockAgentEvent>) {
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    match receiver.try_next() {
        Ok(val) => match val {
            Some(event) => {
                panic!("expecting no event, but received one: {}", event_to_string(event));
            }
            None => {}
        },
        Err(_) => {}
    }
}

fn make_spawn_function_request(class_id: &str) -> SpawnFunctionRequest {
    SpawnFunctionRequest {
        instance_id: None,
        code: FunctionClassSpecification {
            function_class_id: class_id.to_string(),
            function_class_type: "ft-1".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_inlude_code: "function_code".as_bytes().to_vec(),
            outputs: vec![],
        },
        annotations: std::collections::HashMap::new(),
        state_specification: StateSpecification {
            state_id: uuid::Uuid::new_v4(),
            state_policy: StatePolicy::NodeLocal,
        },
    }
}

fn make_start_resource_request(class_type: &str) -> ResourceInstanceSpecification {
    ResourceInstanceSpecification {
        class_type: class_type.to_string(),
        output_mapping: std::collections::HashMap::new(),
        configuration: std::collections::HashMap::new(),
    }
}

#[tokio::test]
async fn orc_single_node_function_start_stop() {
    let (mut fun_client, mut _res_client, mut _mgt_client, mut nodes) = test_setup(1, 0).await;
    assert_eq!(1, nodes.len());
    let (node_id, mock_node_receiver) = nodes.iter_mut().next().unwrap();
    assert!(!node_id.is_nil());

    assert!(mock_node_receiver.try_next().is_err());

    // Start a function.

    let spawn_req = make_spawn_function_request("fc-1");
    let instance_id = match fun_client.start(spawn_req.clone()).await.unwrap() {
        StartComponentResponse::InstanceId(id) => id,
        StartComponentResponse::ResponseError(err) => panic!("{}", err),
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
    no_function_event(mock_node_receiver).await;
}

#[tokio::test]
async fn orc_multiple_nodes_function_start_stop() {
    let (mut fun_client, mut _res_client, mut _mgt_client, mut nodes) = test_setup(3, 0).await;
    assert_eq!(3, nodes.len());

    // Start 100 functions.

    let mut ext_instance_ids = vec![];
    let mut int_instance_ids = vec![];
    let mut node_ids = vec![];
    for i in 0..100 {
        let spawn_req = make_spawn_function_request(format!("fc-{}", i).as_str());
        ext_instance_ids.push(match fun_client.start(spawn_req.clone()).await.unwrap() {
            StartComponentResponse::InstanceId(id) => id,
            StartComponentResponse::ResponseError(err) => panic!("{}", err),
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
async fn orc_multiple_resources_start_stop() {
    let (mut _fun_client, mut res_client, mut _mgt_client, mut nodes) = test_setup(3, 3).await;
    assert_eq!(3, nodes.len());

    // Start 100 resources.

    let mut ext_instance_ids = vec![];
    let mut int_instance_ids = vec![];
    let mut node_ids = vec![];
    for _i in 0..100 {
        let start_req = make_start_resource_request("rc-1");
        ext_instance_ids.push(match res_client.start(start_req.clone()).await.unwrap() {
            StartComponentResponse::InstanceId(id) => id,
            StartComponentResponse::ResponseError(err) => panic!("{}", err),
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
        StartComponentResponse::InstanceId(_) => {
            panic!("started a resource for a non-existing class type");
        }
        StartComponentResponse::ResponseError(err) => {
            assert_eq!("class type not found".to_string(), err.summary);
        }
    }
}

#[tokio::test]
async fn orc_patch() {
    let (mut fun_client, mut res_client, mut _mgt_client, mut nodes) = test_setup(1, 1).await;
    assert_eq!(1, nodes.len());
    let client_node_id = nodes.keys().next().unwrap().clone();

    // Spawn a function instance.

    let spawn_req = make_spawn_function_request("fc-1");
    let ext_function_id = match fun_client.start(spawn_req.clone()).await.unwrap() {
        StartComponentResponse::InstanceId(id) => id,
        StartComponentResponse::ResponseError(err) => panic!("{}", err),
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
        StartComponentResponse::InstanceId(id) => id,
        StartComponentResponse::ResponseError(err) => panic!("{}", err),
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
        .patch(PatchRequest {
            function_id: ext_function_id.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out-1".to_string(),
                InstanceId {
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
        .patch(PatchRequest {
            function_id: ext_resource_id.clone(),
            output_mapping: std::collections::HashMap::from([(
                "out-2".to_string(),
                InstanceId {
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
