use edgeless_api::function_instance::{FunctionClassSpecification, StatePolicy, StateSpecification};

use super::*;

enum MockFunctionInstanceEvent {
    Start(
        (
            edgeless_api::function_instance::InstanceId,
            edgeless_api::function_instance::SpawnFunctionRequest,
        ),
    ),
    Stop(edgeless_api::function_instance::InstanceId),
    Patch(edgeless_api::common::PatchRequest),
    UpdatePeers(UpdatePeersRequest),
    KeepAlive(),
}

struct MockNode {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockFunctionInstanceEvent>,
}

impl edgeless_api::agent::AgentAPI for MockNode {
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceNodeAPI> {
        Box::new(MockFunctionInstanceAPI {
            node_id: self.node_id.clone(),
            sender: self.sender.clone(),
        })
    }
}

#[derive(Clone)]
struct MockFunctionInstanceAPI {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockFunctionInstanceEvent>,
}

#[async_trait::async_trait]
impl edgeless_api::function_instance::FunctionInstanceNodeAPI for MockFunctionInstanceAPI {
    async fn start(
        &mut self,
        spawn_request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse> {
        let new_id = edgeless_api::function_instance::InstanceId {
            node_id: self.node_id.clone(),
            function_id: uuid::Uuid::new_v4(),
        };
        self.sender.send(MockFunctionInstanceEvent::Start((new_id, spawn_request))).await.unwrap();
        Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.sender.send(MockFunctionInstanceEvent::Stop(id)).await.unwrap();
        Ok(())
    }
    async fn patch(&mut self, request: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        self.sender.send(MockFunctionInstanceEvent::Patch(request)).await.unwrap();
        Ok(())
    }
    async fn update_peers(&mut self, request: edgeless_api::function_instance::UpdatePeersRequest) -> anyhow::Result<()> {
        self.sender.send(MockFunctionInstanceEvent::UpdatePeers(request)).await.unwrap();
        Ok(())
    }
    async fn keep_alive(&mut self) -> anyhow::Result<()> {
        self.sender.send(MockFunctionInstanceEvent::KeepAlive()).await.unwrap();
        Ok(())
    }
}

enum MockResourceEvent {
    Start(
        (
            edgeless_api::function_instance::InstanceId,
            edgeless_api::resource_configuration::ResourceInstanceSpecification,
        ),
    ),
    Stop(edgeless_api::function_instance::InstanceId),
    Patch(edgeless_api::common::PatchRequest),
}

struct MockResourceProvider {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockResourceEvent>,
}

impl edgeless_api::resource_provider::ResourceProviderAPI for MockResourceProvider {
    fn resource_configuration_api(&mut self) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI> {
        Box::new(MockResourceConfigurationAPI {
            node_id: self.node_id.clone(),
            sender: self.sender.clone(),
        })
    }
}

#[derive(Clone)]
struct MockResourceConfigurationAPI {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockResourceEvent>,
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI for MockResourceConfigurationAPI {
    async fn start(
        &mut self,
        start_request: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse> {
        let new_id = edgeless_api::function_instance::InstanceId {
            node_id: self.node_id.clone(),
            function_id: uuid::Uuid::new_v4(),
        };
        self.sender.send(MockResourceEvent::Start((new_id.clone(), start_request))).await.unwrap();
        Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.sender.send(MockResourceEvent::Stop(id)).await.unwrap();
        Ok(())
    }
    async fn patch(&mut self, request: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        self.sender.send(MockResourceEvent::Patch(request)).await.unwrap();
        Ok(())
    }
}

async fn test_setup(
    num_nodes: u32,
    provider_names: Vec<String>,
) -> (
    Box<dyn edgeless_api::function_instance::FunctionInstanceOrcAPI>,
    std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockFunctionInstanceEvent>>,
    std::collections::HashMap<String, futures::channel::mpsc::UnboundedReceiver<MockResourceEvent>>,
) {
    let mut nodes = std::collections::HashMap::new();
    let mut clients = std::collections::HashMap::new();
    for _ in 0..num_nodes {
        let (mock_node_sender, mock_node_receiver) = futures::channel::mpsc::unbounded::<MockFunctionInstanceEvent>();
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
    }

    let mut providers = std::collections::HashMap::new();
    let mut resource_providers = std::collections::HashMap::new();
    for provider in provider_names.iter() {
        let (mock_resource_provider_sender, mock_resource_provider_receiver) = futures::channel::mpsc::unbounded::<MockResourceEvent>();
        providers.insert(provider.clone(), mock_resource_provider_receiver);
        let node_id = uuid::Uuid::new_v4();
        resource_providers.insert(
            provider.clone(),
            ResourceProvider {
                class_type: "rc-1".to_string(),
                node_id: node_id.clone(),
                outputs: vec![],
                config_api: Box::new(MockResourceProvider {
                    node_id: node_id.clone(),
                    sender: mock_resource_provider_sender,
                }) as Box<dyn edgeless_api::resource_provider::ResourceProviderAPI + Send>,
            },
        );
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

    (orchestrator.get_api_client().function_instance_api(), nodes, providers)
}

#[allow(dead_code)]
fn event_to_string(event: MockFunctionInstanceEvent) -> &'static str {
    match event {
        MockFunctionInstanceEvent::Start(_) => "start",
        MockFunctionInstanceEvent::Stop(_) => "stop",
        MockFunctionInstanceEvent::Patch(_) => "patch",
        MockFunctionInstanceEvent::UpdatePeers(_) => "update_peers",
        MockFunctionInstanceEvent::KeepAlive() => "keep_alive",
    }
}

#[allow(dead_code)]
fn msg_to_string(msg: Result<Option<MockFunctionInstanceEvent>, futures::channel::mpsc::TryRecvError>) -> &'static str {
    match msg {
        Ok(val) => match val {
            Some(val) => event_to_string(val),
            None => "none",
        },
        Err(_) => "error",
    }
}

async fn wait_for_function_event(receiver: &mut futures::channel::mpsc::UnboundedReceiver<MockFunctionInstanceEvent>) -> MockFunctionInstanceEvent {
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

async fn wait_for_function_event_multiple(
    receivers: &mut std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedReceiver<MockFunctionInstanceEvent>>,
) -> (uuid::Uuid, MockFunctionInstanceEvent) {
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

async fn no_function_event(receiver: &mut futures::channel::mpsc::UnboundedReceiver<MockFunctionInstanceEvent>) {
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

async fn wait_for_resource_event_multiple(
    receivers: &mut std::collections::HashMap<String, futures::channel::mpsc::UnboundedReceiver<MockResourceEvent>>,
) -> (String, MockResourceEvent) {
    for _ in 0..100 {
        for (provider_id, receiver) in receivers.iter_mut() {
            match receiver.try_next() {
                Ok(val) => match val {
                    Some(event) => {
                        return (provider_id.clone(), event);
                    }
                    None => {}
                },
                Err(_) => {}
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
    panic!("timeout while waiting for a resource");
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

fn make_start_resource_request(class_type: &str) -> StartResourceRequest {
    StartResourceRequest {
        class_type: class_type.to_string(),
        configurations: std::collections::HashMap::new(),
    }
}

#[tokio::test]
async fn orc_single_node_function_start_stop() {
    let (mut client, mut nodes, _) = test_setup(1, vec![]).await;
    assert_eq!(1, nodes.len());
    let (node_id, mock_node_receiver) = nodes.iter_mut().next().unwrap();
    assert!(!node_id.is_nil());

    assert!(mock_node_receiver.try_next().is_err());

    // Start a function.

    let spawn_req = make_spawn_function_request("fc-1");
    let instance_id = match client.start_function(spawn_req.clone()).await.unwrap() {
        StartComponentResponse::InstanceId(id) => id,
        StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    assert!(instance_id.node_id.is_nil());

    let mut int_instance_id = None;
    if let MockFunctionInstanceEvent::Start((new_instance_id, spawn_req_rcvd)) = wait_for_function_event(mock_node_receiver).await {
        assert!(int_instance_id.is_none());
        int_instance_id = Some(new_instance_id);
        assert_eq!(spawn_req, spawn_req_rcvd);
    } else {
        panic!("wrong event received");
    }

    // Stop the function previously started.

    match client.stop_function(instance_id).await {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }

    if let MockFunctionInstanceEvent::Stop(instance_id_rcvd) = wait_for_function_event(mock_node_receiver).await {
        assert!(int_instance_id.is_some());
        assert_eq!(int_instance_id.unwrap(), instance_id_rcvd);
    } else {
        panic!("wrong event received");
    }

    // Stop the function again.
    match client.stop_function(instance_id).await {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    }
    no_function_event(mock_node_receiver).await;
}

#[tokio::test]
async fn orc_multiple_nodes_function_start_stop() {
    let (mut client, mut nodes, _) = test_setup(3, vec![]).await;
    assert_eq!(3, nodes.len());

    // Start 100 functions.

    let mut ext_instance_ids = vec![];
    let mut int_instance_ids = vec![];
    let mut node_ids = vec![];
    for i in 0..100 {
        let spawn_req = make_spawn_function_request(format!("fc-{}", i).as_str());
        ext_instance_ids.push(match client.start_function(spawn_req.clone()).await.unwrap() {
            StartComponentResponse::InstanceId(id) => id,
            StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });
        assert!(ext_instance_ids.last().unwrap().node_id.is_nil());

        if let (node_id, MockFunctionInstanceEvent::Start((new_instance_id, spawn_req_rcvd))) = wait_for_function_event_multiple(&mut nodes).await {
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
        match client.stop_function(ext_instance_ids[i]).await {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }

        if let (node_id, MockFunctionInstanceEvent::Stop(instance_id_rcvd)) = wait_for_function_event_multiple(&mut nodes).await {
            assert_eq!(node_ids[i], node_id);
            assert_eq!(int_instance_ids[i], instance_id_rcvd);
        } else {
            panic!("wrong event received");
        }
    }
}

#[tokio::test]
async fn orc_multiple_resources_start_stop() {
    let (mut client, nodes, mut providers) = test_setup(0, vec!["provider-1".to_string(), "provider-2".to_string(), "provider-3".to_string()]).await;
    assert_eq!(0, nodes.len());
    assert_eq!(3, providers.len());

    // Start 100 resources.

    let mut ext_instance_ids = vec![];
    let mut int_instance_ids = vec![];
    let mut provider_ids = vec![];
    for _i in 0..100 {
        let start_req = make_start_resource_request("rc-1");
        ext_instance_ids.push(match client.start_resource(start_req.clone()).await.unwrap() {
            StartComponentResponse::InstanceId(id) => id,
            StartComponentResponse::ResponseError(err) => panic!("{}", err),
        });
        assert!(ext_instance_ids.last().unwrap().node_id.is_nil());

        if let (provider_id, MockResourceEvent::Start((new_instance_id, resource_instance_spec))) =
            wait_for_resource_event_multiple(&mut providers).await
        {
            int_instance_ids.push(new_instance_id);
            assert!(resource_instance_spec.configuration.is_empty());
            assert_eq!(provider_id, resource_instance_spec.provider_id);
            provider_ids.push(provider_id);
        } else {
            panic!("wrong event received");
        }
    }

    // Check that all resource providers have been selected at least once.

    let mut providers_selected = std::collections::HashSet::new();
    for provider_id in provider_ids.iter() {
        providers_selected.insert(provider_id);
    }
    assert_eq!(3, providers_selected.len());

    // Stop the resources previously started.

    assert_eq!(100, ext_instance_ids.len());
    assert_eq!(100, int_instance_ids.len());
    assert_eq!(100, provider_ids.len());
    for i in 0..100 {
        match client.stop_resource(ext_instance_ids[i]).await {
            Ok(_) => {}
            Err(err) => {
                panic!("{}", err);
            }
        }

        if let (provider_id, MockResourceEvent::Stop(instance_id_rcvd)) = wait_for_resource_event_multiple(&mut providers).await {
            assert_eq!(provider_ids[i], provider_id);
            assert_eq!(int_instance_ids[i], instance_id_rcvd);
        } else {
            panic!("wrong event received");
        }
    }

    // Start a resource with unknown class type.
    match client.start_resource(make_start_resource_request("rc-666")).await.unwrap() {
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
    let (mut client, mut nodes, mut providers) = test_setup(1, vec!["provider-1".to_string()]).await;
    assert_eq!(1, nodes.len());
    assert_eq!(1, providers.len());
    let client_node_id = nodes.keys().next().unwrap().clone();

    // Spawn a function instance.

    let spawn_req = make_spawn_function_request("fc-1");
    let ext_function_id = match client.start_function(spawn_req.clone()).await.unwrap() {
        StartComponentResponse::InstanceId(id) => id,
        StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    assert!(ext_function_id.node_id.is_nil());

    let mut int_function_id = None;
    assert!(int_function_id.is_none());
    if let (node_id, MockFunctionInstanceEvent::Start((new_instance_id, spawn_req_rcvd))) = wait_for_function_event_multiple(&mut nodes).await {
        assert_eq!(client_node_id, node_id);
        int_function_id = Some(new_instance_id);
        assert_eq!(spawn_req, spawn_req_rcvd);
    } else {
        panic!("wrong event received");
    }

    // Start a resource.

    let start_req = make_start_resource_request("rc-1");
    let ext_resource_id = match client.start_resource(start_req.clone()).await.unwrap() {
        StartComponentResponse::InstanceId(id) => id,
        StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };
    assert!(ext_resource_id.node_id.is_nil());

    let mut int_resource_id = None;
    assert!(int_resource_id.is_none());
    if let (provider_id, MockResourceEvent::Start((new_instance_id, resource_instance_spec))) = wait_for_resource_event_multiple(&mut providers).await
    {
        int_resource_id = Some(new_instance_id);
        assert!(resource_instance_spec.configuration.is_empty());
        assert_eq!("provider-1".to_string(), provider_id);
        assert_eq!(provider_id, resource_instance_spec.provider_id);
    } else {
        panic!("wrong event received");
    }

    // Gotta patch 'em all.

    match client
        .patch(PatchRequest {
            function_id: ext_function_id.function_id.clone(),
            output_mapping: std::collections::HashMap::from([("out-1".to_string(), ext_resource_id.clone())]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };

    if let (node_id, MockFunctionInstanceEvent::Patch(patch_request)) = wait_for_function_event_multiple(&mut nodes).await {
        assert_eq!(client_node_id, node_id);
        assert_eq!(int_function_id.unwrap().function_id, patch_request.function_id);
        assert_eq!(1, patch_request.output_mapping.len());
        let mapping = patch_request.output_mapping.get("out-1");
        assert!(mapping.is_some());
        assert_eq!(int_resource_id.unwrap(), mapping.unwrap().clone());
    } else {
        panic!("wrong event received");
    }

    match client
        .patch(PatchRequest {
            function_id: ext_resource_id.function_id.clone(),
            output_mapping: std::collections::HashMap::from([("out-2".to_string(), ext_function_id.clone())]),
        })
        .await
    {
        Ok(_) => {}
        Err(err) => {
            panic!("{}", err);
        }
    };

    if let (provider_id, MockResourceEvent::Patch(patch_request)) = wait_for_resource_event_multiple(&mut providers).await {
        assert_eq!("provider-1".to_string(), provider_id);
        assert_eq!(int_resource_id.unwrap().function_id, patch_request.function_id);
        assert_eq!(1, patch_request.output_mapping.len());
        let mapping = patch_request.output_mapping.get("out-2");
        assert!(mapping.is_some());
        assert_eq!(int_function_id.unwrap(), mapping.unwrap().clone());
    } else {
        panic!("wrong event received");
    }
}
