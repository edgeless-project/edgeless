use edgeless_api::function_instance::{FunctionClassSpecification, StatePolicy, StateSpecification};

use super::*;

enum MockFunctionInstanceEvent {
    Start(edgeless_api::function_instance::SpawnFunctionRequest),
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
        self.sender.send(MockFunctionInstanceEvent::Start(spawn_request)).await.unwrap();
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

// async fn start_resource(
//     &mut self,
//     spawn_request: edgeless_api::function_instance::StartResourceRequest,
// ) -> anyhow::Result<edgeless_api::common::StartComponentResponse> {
//     let new_id = edgeless_api::function_instance::InstanceId {
//         node_id: uuid::Uuid::nil(),
//         function_id: uuid::Uuid::new_v4(),
//     };
//     self.sender
//         .send(MockFunctionInstanceEvent::StartResource((new_id.clone(), spawn_request)))
//         .await
//         .unwrap();
//     Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
// }
// async fn stop_resource(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
//     self.sender.send(MockFunctionInstanceEvent::StopResource(id)).await.unwrap();
//     Ok(())
// }

async fn test_setup() -> (
    Box<dyn edgeless_api::function_instance::FunctionInstanceOrcAPI>,
    futures::channel::mpsc::UnboundedReceiver<MockFunctionInstanceEvent>,
    uuid::Uuid,
) {
    let (mock_node_sender, mock_node_receiver) = futures::channel::mpsc::unbounded::<MockFunctionInstanceEvent>();
    let node_id = uuid::Uuid::new_v4();
    let mock_node = MockNode {
        node_id: node_id.clone(),
        sender: mock_node_sender,
    };

    let clients = std::collections::HashMap::from([(
        node_id.clone(),
        ClientDesc {
            agent_url: "".to_string(),
            invocation_url: "".to_string(),
            api: Box::new(mock_node) as Box<dyn edgeless_api::agent::AgentAPI + Send>,
        },
    )]);

    let (mut orchestrator, orchestrator_task) = Orchestrator::new_with_clients(
        crate::EdgelessOrcSettings {
            domain_id: "".to_string(),        // unused
            orchestrator_url: "".to_string(), // unused
            orchestration_strategy: crate::OrchestrationStrategy::Random,
            keep_alive_interval_secs: 0 as u64, // unused
        },
        clients,
    )
    .await;
    tokio::spawn(orchestrator_task);

    (orchestrator.get_api_client().function_instance_api(), mock_node_receiver, node_id)
}

fn event_to_string(msg: Result<Option<MockFunctionInstanceEvent>, futures::channel::mpsc::TryRecvError>) -> &'static str {
    match msg {
        Ok(val) => match val {
            Some(val) => match val {
                MockFunctionInstanceEvent::Start(_) => "start",
                MockFunctionInstanceEvent::Stop(_) => "stop",
                MockFunctionInstanceEvent::Patch(_) => "patch",
                MockFunctionInstanceEvent::UpdatePeers(_) => "update_peers",
                MockFunctionInstanceEvent::KeepAlive() => "keep_alive",
            },
            None => "none",
        },
        Err(_) => "error",
    }
}

#[tokio::test]
async fn orc_function_start_stop() {
    let (mut client, mut mock_node_receiver, node_id) = test_setup().await;
    assert!(!node_id.is_nil());

    println!("{}", event_to_string(mock_node_receiver.try_next()));

    assert!(mock_node_receiver.try_next().is_err());

    let instance_id = match client
        .start_function(SpawnFunctionRequest {
            instance_id: None,
            code: FunctionClassSpecification {
                function_class_id: "fc-1".to_string(),
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
        })
        .await
        .unwrap()
    {
        StartComponentResponse::InstanceId(id) => id,
        StartComponentResponse::ResponseError(err) => panic!("{}", err),
    };

    // [TODO] continue
    println!("{:?}", instance_id);
}
