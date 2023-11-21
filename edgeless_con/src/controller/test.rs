use edgeless_api::workflow_instance::SpawnWorkflowResponse;

use super::*;

enum MockFunctionInstanceEvent {
    Start(
        (
            // this is the id passed from the orchestrator to the controller
            edgeless_api::function_instance::InstanceId,
            edgeless_api::function_instance::SpawnFunctionRequest,
        ),
    ),
    Stop(edgeless_api::function_instance::InstanceId),
    Update(edgeless_api::function_instance::UpdateFunctionLinksRequest),
}

struct MockOrchestrator {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockFunctionInstanceEvent>,
}

impl edgeless_api::orc::OrchestratorAPI for MockOrchestrator {
    fn function_instance_api(&mut self) -> Box<dyn edgeless_api::function_instance::FunctionInstanceAPI> {
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
impl edgeless_api::function_instance::FunctionInstanceAPI for MockFunctionInstanceAPI {
    async fn start(
        &mut self,
        spawn_request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<edgeless_api::function_instance::SpawnFunctionResponse> {
        let new_id = edgeless_api::function_instance::InstanceId::new(self.node_id);
        self.sender
            .send(MockFunctionInstanceEvent::Start((new_id.clone(), spawn_request)))
            .await
            .unwrap();
        Ok(edgeless_api::function_instance::SpawnFunctionResponse::InstanceId(new_id))
    }
    async fn stop(&mut self, id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.sender.send(MockFunctionInstanceEvent::Stop(id)).await.unwrap();
        Ok(())
    }
    async fn update_links(&mut self, update: edgeless_api::function_instance::UpdateFunctionLinksRequest) -> anyhow::Result<()> {
        self.sender.send(MockFunctionInstanceEvent::Update(update)).await.unwrap();
        Ok(())
    }
}

enum MockResourceEvent {
    Start(
        (
            // this is the id passed from the orchestrator to the controller
            edgeless_api::function_instance::InstanceId,
            edgeless_api::resource_configuration::ResourceInstanceSpecification,
        ),
    ),
    Stop(edgeless_api::function_instance::InstanceId),
}

struct MockResourceProvider {
    node_id: uuid::Uuid,
    sender: futures::channel::mpsc::UnboundedSender<MockResourceEvent>,
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI for MockResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::resource_configuration::SpawnResourceResponse> {
        let new_id = edgeless_api::function_instance::InstanceId::new(self.node_id);
        self.sender
            .send(MockResourceEvent::Start((new_id.clone(), instance_specification)))
            .await
            .unwrap();
        Ok(edgeless_api::resource_configuration::SpawnResourceResponse::InstanceId(new_id))
    }
    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.sender.send(MockResourceEvent::Stop(resource_id)).await.unwrap();
        Ok(())
    }
}

async fn test_setup() -> (
    Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>,
    futures::channel::mpsc::UnboundedReceiver<MockFunctionInstanceEvent>,
    futures::channel::mpsc::UnboundedReceiver<MockResourceEvent>,
) {
    let (mock_orc_sender, mock_orc_receiver) = futures::channel::mpsc::unbounded::<MockFunctionInstanceEvent>();
    let mock_orc = MockOrchestrator {
        node_id: uuid::Uuid::new_v4(),
        sender: mock_orc_sender,
    };

    let (mock_res_sender, mock_res_receiver) = futures::channel::mpsc::unbounded::<MockResourceEvent>();
    let mock_res = MockResourceProvider {
        node_id: uuid::Uuid::new_v4(),
        sender: mock_res_sender,
    };

    let orc_clients = std::collections::HashMap::<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>::from([(
        "domain-1".to_string(),
        Box::new(mock_orc) as Box<dyn edgeless_api::orc::OrchestratorAPI>,
    )]);
    let resources = std::collections::HashMap::<String, ResourceHandle>::from([(
        "resource-1".to_string(),
        ResourceHandle {
            resource_type: "test-res".to_string(),
            _outputs: vec!["test_out".to_string()],
            config_api: Box::new(mock_res) as Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI + Send>,
        },
    )]);

    let (mut controller, controller_task) = Controller::new(orc_clients, resources);
    tokio::spawn(controller_task);
    let mut client = controller.get_api_client();
    let wf_client = client.workflow_instance_api();

    (wf_client, mock_orc_receiver, mock_res_receiver)
}

#[tokio::test]
async fn single_function_start_stop() {
    let (mut wf_client, mut mock_orc_receiver, mut mock_res_receiver) = test_setup().await;

    assert!(mock_orc_receiver.try_next().is_err());
    assert!(mock_res_receiver.try_next().is_err());

    let wf_id = edgeless_api::workflow_instance::WorkflowId {
        workflow_id: uuid::Uuid::new_v4(),
    };

    let response = wf_client
        .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
            workflow_id: wf_id.clone(),
            workflow_functions: vec![edgeless_api::workflow_instance::WorkflowFunction {
                name: "f1".to_string(),
                function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                    function_class_id: "fc1".to_string(),
                    function_class_type: "RUST_WASM".to_string(),
                    function_class_version: "0.1".to_string(),
                    function_class_inlude_code: vec![],
                    outputs: vec![],
                },
                output_mapping: std::collections::HashMap::new(),
                annotations: std::collections::HashMap::new(),
            }],
            workflow_resources: vec![],
            annotations: std::collections::HashMap::new(),
        })
        .await
        .unwrap();

    let instance = match &response {
        SpawnWorkflowResponse::ResponseError(err) => panic!("{}", err),
        SpawnWorkflowResponse::WorkflowInstance(val) => val,
    };

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let start_res = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::Start((id, _spawn_req)) = start_res {
        assert_eq!(instance.functions[0].instances[0], id);
    } else {
        panic!();
    }
    assert!(mock_res_receiver.try_next().is_err());

    assert!(mock_orc_receiver.try_next().is_err());

    wf_client.stop(wf_id).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let stop_res = mock_orc_receiver.try_next().unwrap().unwrap();

    if let MockFunctionInstanceEvent::Stop(id) = stop_res {
        assert_eq!(instance.functions[0].instances[0], id);
    } else {
        panic!();
    }

    assert!(mock_orc_receiver.try_next().is_err());
    assert!(mock_res_receiver.try_next().is_err());
}

#[tokio::test]
async fn resource_to_function_start_stop() {
    let (mut wf_client, mut mock_orc_receiver, mut mock_res_receiver) = test_setup().await;

    assert!(mock_orc_receiver.try_next().is_err());
    assert!(mock_res_receiver.try_next().is_err());

    let wf_id = edgeless_api::workflow_instance::WorkflowId {
        workflow_id: uuid::Uuid::new_v4(),
    };

    let response = wf_client
        .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
            workflow_id: wf_id.clone(),
            workflow_functions: vec![edgeless_api::workflow_instance::WorkflowFunction {
                name: "f1".to_string(),
                function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                    function_class_id: "fc1".to_string(),
                    function_class_type: "RUST_WASM".to_string(),
                    function_class_version: "0.1".to_string(),
                    function_class_inlude_code: vec![],
                    outputs: vec![],
                },
                output_mapping: std::collections::HashMap::new(),
                annotations: std::collections::HashMap::new(),
            }],
            workflow_resources: vec![edgeless_api::workflow_instance::WorkflowResource {
                name: "r1".to_string(),
                class_type: "test-res".to_string(),
                output_mapping: std::collections::HashMap::from([("test_out".to_string(), "f1".to_string())]),
                configurations: std::collections::HashMap::new(),
            }],
            annotations: std::collections::HashMap::new(),
        })
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let instance = match &response {
        SpawnWorkflowResponse::ResponseError(err) => panic!("{}", err),
        SpawnWorkflowResponse::WorkflowInstance(val) => val,
    };

    let res = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::Start((id, _spawn_req)) = res {
        assert_eq!(instance.functions[0].instances[0], id);
    } else {
        panic!();
    }

    let resource_res = mock_res_receiver.try_next().unwrap().unwrap();
    if let MockResourceEvent::Start((_id, spawn_req)) = resource_res {
        assert_eq!(*spawn_req.output_mapping.get("test_out").unwrap(), instance.functions[0].instances[0]);
    } else {
        panic!();
    }

    assert!(mock_orc_receiver.try_next().is_err());
    assert!(mock_res_receiver.try_next().is_err());

    wf_client.stop(wf_id).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let stop_res = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::Stop(id) = stop_res {
        assert_eq!(instance.functions[0].instances[0], id);
    } else {
        panic!();
    }

    let stop_resource_res = mock_res_receiver.try_next().unwrap().unwrap();
    if let MockResourceEvent::Stop(_id) = stop_resource_res {
    } else {
        panic!();
    }

    assert!(mock_orc_receiver.try_next().is_err());
    assert!(mock_res_receiver.try_next().is_err());
}

#[tokio::test]
async fn function_link_loop_start_stop() {
    let (mut wf_client, mut mock_orc_receiver, mut mock_res_receiver) = test_setup().await;

    assert!(mock_orc_receiver.try_next().is_err());
    assert!(mock_res_receiver.try_next().is_err());

    let wf_id = edgeless_api::workflow_instance::WorkflowId {
        workflow_id: uuid::Uuid::new_v4(),
    };

    let response = wf_client
        .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
            workflow_id: wf_id.clone(),
            workflow_functions: vec![
                edgeless_api::workflow_instance::WorkflowFunction {
                    name: "f1".to_string(),
                    function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                        function_class_id: "fc1".to_string(),
                        function_class_type: "RUST_WASM".to_string(),
                        function_class_version: "0.1".to_string(),
                        function_class_inlude_code: vec![],
                        outputs: vec!["output-1".to_string()],
                    },
                    output_mapping: std::collections::HashMap::from([("output-1".to_string(), "f2".to_string())]),
                    annotations: std::collections::HashMap::new(),
                },
                edgeless_api::workflow_instance::WorkflowFunction {
                    name: "f2".to_string(),
                    function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                        function_class_id: "fc2".to_string(),
                        function_class_type: "RUST_WASM".to_string(),
                        function_class_version: "0.1".to_string(),
                        function_class_inlude_code: vec![],
                        outputs: vec!["output-2".to_string()],
                    },
                    output_mapping: std::collections::HashMap::from([("output-2".to_string(), "f1".to_string())]),
                    annotations: std::collections::HashMap::new(),
                },
            ],
            workflow_resources: vec![],
            annotations: std::collections::HashMap::new(),
        })
        .await
        .unwrap();

    let instance = match &response {
        SpawnWorkflowResponse::ResponseError(err) => panic!("{}", err),
        SpawnWorkflowResponse::WorkflowInstance(val) => val,
    };

    let fids: std::collections::HashSet<_> = instance.functions.iter().flat_map(|instances| instances.instances.clone()).collect();
    let to_patch: Option<edgeless_api::function_instance::InstanceId>;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let res = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::Start((id, spawn_req)) = res {
        assert!(fids.contains(&id));
        assert_eq!(spawn_req.output_mapping.len(), 0);
        to_patch = Some(id);
    } else {
        panic!();
    }
    let res2 = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::Start((id, spawn_req)) = res2 {
        assert!(fids.contains(&id));
        assert_eq!(spawn_req.output_mapping.len(), 1);
    } else {
        panic!();
    }
    let res3 = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::Update(update_req) = res3 {
        assert_eq!(update_req.instance_id.unwrap(), to_patch.unwrap());
        assert_eq!(update_req.output_mapping.len(), 1);
    } else {
        panic!();
    }

    assert!(mock_res_receiver.try_next().is_err());
    assert!(mock_orc_receiver.try_next().is_err());

    wf_client.stop(wf_id).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let stop_res = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::Stop(id) = stop_res {
        assert!(fids.contains(&id));
    } else {
        panic!();
    }

    let stop_res2 = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::Stop(id) = stop_res2 {
        assert!(fids.contains(&id));
    } else {
        panic!();
    }

    assert!(mock_res_receiver.try_next().is_err());
}
