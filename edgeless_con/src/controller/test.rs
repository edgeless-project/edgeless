use controller_task::OrchestratorDesc;
// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_api::{
    domain_registration::DomainCapabilities, workflow_instance::SpawnWorkflowResponse,
};

use super::*;

use futures::SinkExt;

enum MockFunctionInstanceEvent {
    StartFunction(
        (
            // this is the id passed from the orchestrator to the controller
            edgeless_api::function_instance::DomainManagedInstanceId,
            edgeless_api::function_instance::SpawnFunctionRequest,
        ),
    ),
    StopFunction(edgeless_api::function_instance::DomainManagedInstanceId),
    StartResource(
        (
            // this is the id passed from the orchestrator to the controller
            edgeless_api::function_instance::DomainManagedInstanceId,
            edgeless_api::resource_configuration::ResourceInstanceSpecification,
        ),
    ),
    StopResource(edgeless_api::function_instance::DomainManagedInstanceId),
    Patch(edgeless_api::common::PatchRequest),
}

struct MockOrchestrator {
    sender: futures::channel::mpsc::UnboundedSender<MockFunctionInstanceEvent>,
}

impl edgeless_api::outer::orc::OrchestratorAPI for MockOrchestrator {
    fn function_instance_api(
        &mut self,
    ) -> Box<
        dyn edgeless_api::function_instance::FunctionInstanceAPI<
            edgeless_api::function_instance::DomainManagedInstanceId,
        >,
    > {
        Box::new(MockFunctionInstanceAPI {
            sender: self.sender.clone(),
        })
    }

    fn resource_configuration_api(
        &mut self,
    ) -> Box<
        dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<
            edgeless_api::function_instance::DomainManagedInstanceId,
        >,
    > {
        Box::new(MockResourceConfigurationAPI {
            sender: self.sender.clone(),
        })
    }
}

#[derive(Clone)]
struct MockFunctionInstanceAPI {
    sender: futures::channel::mpsc::UnboundedSender<MockFunctionInstanceEvent>,
}

#[derive(Clone)]
struct MockResourceConfigurationAPI {
    sender: futures::channel::mpsc::UnboundedSender<MockFunctionInstanceEvent>,
}

#[async_trait::async_trait]
impl
    edgeless_api::function_instance::FunctionInstanceAPI<
        edgeless_api::function_instance::DomainManagedInstanceId,
    > for MockFunctionInstanceAPI
{
    async fn start(
        &mut self,
        spawn_request: edgeless_api::function_instance::SpawnFunctionRequest,
    ) -> anyhow::Result<
        edgeless_api::common::StartComponentResponse<
            edgeless_api::function_instance::DomainManagedInstanceId,
        >,
    > {
        let new_id = uuid::Uuid::new_v4();
        self.sender
            .send(MockFunctionInstanceEvent::StartFunction((
                new_id,
                spawn_request,
            )))
            .await
            .unwrap();
        Ok(edgeless_api::common::StartComponentResponse::InstanceId(
            new_id,
        ))
    }
    async fn stop(
        &mut self,
        id: edgeless_api::function_instance::DomainManagedInstanceId,
    ) -> anyhow::Result<()> {
        self.sender
            .send(MockFunctionInstanceEvent::StopFunction(id))
            .await
            .unwrap();
        Ok(())
    }

    async fn patch(&mut self, request: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        self.sender
            .send(MockFunctionInstanceEvent::Patch(request))
            .await
            .unwrap();
        Ok(())
    }
}
#[async_trait::async_trait]
impl
    edgeless_api::resource_configuration::ResourceConfigurationAPI<
        edgeless_api::function_instance::DomainManagedInstanceId,
    > for MockResourceConfigurationAPI
{
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<
        edgeless_api::common::StartComponentResponse<
            edgeless_api::function_instance::DomainManagedInstanceId,
        >,
    > {
        let new_id = uuid::Uuid::new_v4();
        self.sender
            .send(MockFunctionInstanceEvent::StartResource((
                new_id,
                instance_specification,
            )))
            .await
            .unwrap();
        Ok(edgeless_api::common::StartComponentResponse::InstanceId(
            new_id,
        ))
    }
    async fn stop(
        &mut self,
        resource_id: edgeless_api::function_instance::DomainManagedInstanceId,
    ) -> anyhow::Result<()> {
        self.sender
            .send(MockFunctionInstanceEvent::StopResource(resource_id))
            .await
            .unwrap();
        Ok(())
    }
    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        self.sender
            .send(MockFunctionInstanceEvent::Patch(update))
            .await
            .unwrap();
        Ok(())
    }
}

async fn test_setup() -> (
    Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>,
    futures::channel::mpsc::UnboundedReceiver<MockFunctionInstanceEvent>,
) {
    let (mock_orc_sender, mock_orc_receiver) =
        futures::channel::mpsc::unbounded::<MockFunctionInstanceEvent>();
    let mock_orc = MockOrchestrator {
        sender: mock_orc_sender,
    };

    let (workflow_instance_sender, workflow_instance_receiver) =
        futures::channel::mpsc::unbounded();
    let (_domain_registration_sender, domain_registration_receiver) =
        futures::channel::mpsc::unbounded();
    let (_internal_sender, internal_receiver) = futures::channel::mpsc::unbounded();

    let mut capabilities = DomainCapabilities::default();
    capabilities.runtimes.insert(String::from("RUST_WASM"));
    capabilities
        .resource_classes
        .insert(String::from("test-res"));
    let orchestrators = std::collections::HashMap::from([(
        String::from("domain-1"),
        OrchestratorDesc {
            client: Box::new(mock_orc) as Box<dyn edgeless_api::outer::orc::OrchestratorAPI>,
            orchestrator_url: String::default(),
            capabilities,
            refresh_deadline: std::time::SystemTime::now(),
            counter: 0,
            nonce: 42,
        },
    )]);

    let controller_task = Box::pin(async move {
        let mut controller_task = controller_task::ControllerTask::new_with_orchestrators(
            workflow_instance_receiver,
            domain_registration_receiver,
            internal_receiver,
            orchestrators,
        );
        controller_task.run().await;
    });
    tokio::spawn(controller_task);

    let wf_client = client::ControllerClient::new(workflow_instance_sender).workflow_instance_api();

    (wf_client, mock_orc_receiver)
}

#[tokio::test]
async fn single_function_start_stop() {
    let (mut wf_client, mut mock_orc_receiver) = test_setup().await;

    assert!(mock_orc_receiver.try_next().is_err());

    let function_class_specification =
        edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "fc1".to_string(),
            function_class_type: "RUST_WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_code: vec![],
            function_class_outputs: vec![],
        };
    let start_workflow_request = edgeless_api::workflow_instance::SpawnWorkflowRequest {
        workflow_functions: vec![edgeless_api::workflow_instance::WorkflowFunction {
            name: "f1".to_string(),
            function_class_specification: function_class_specification.clone(),
            output_mapping: std::collections::HashMap::new(),
            annotations: std::collections::HashMap::new(),
        }],
        workflow_resources: vec![],
        annotations: std::collections::HashMap::new(),
    };
    let response = wf_client.start(start_workflow_request).await.unwrap();

    let instance = match &response {
        SpawnWorkflowResponse::ResponseError(err) => panic!("{}", err),
        SpawnWorkflowResponse::WorkflowInstance(val) => val,
    };

    assert_eq!(instance.domain_mapping[0].name, "f1".to_string());
    assert_eq!(instance.domain_mapping[0].domain_id, "domain-1".to_string());

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut new_func_id = uuid::Uuid::nil();
    assert!(new_func_id.is_nil());
    if let MockFunctionInstanceEvent::StartFunction((id, spawn_req)) =
        mock_orc_receiver.try_next().unwrap().unwrap()
    {
        new_func_id = id;
        assert_eq!(function_class_specification, spawn_req.code);
        assert!(spawn_req.annotations.is_empty());
        // TODO check state specifications
    } else {
        panic!();
    }

    assert!(mock_orc_receiver.try_next().is_err());

    wf_client.stop(instance.workflow_id.clone()).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    if let MockFunctionInstanceEvent::StopFunction(id) =
        mock_orc_receiver.try_next().unwrap().unwrap()
    {
        assert_eq!(new_func_id, id);
    } else {
        panic!();
    }

    assert!(mock_orc_receiver.try_next().is_err());
}

#[tokio::test]
async fn resource_to_function_start_stop() {
    let (mut wf_client, mut mock_orc_receiver) = test_setup().await;

    assert!(mock_orc_receiver.try_next().is_err());

    let response = wf_client
        .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
            workflow_functions: vec![edgeless_api::workflow_instance::WorkflowFunction {
                name: "f1".to_string(),
                function_class_specification:
                    edgeless_api::function_instance::FunctionClassSpecification {
                        function_class_id: "fc1".to_string(),
                        function_class_type: "RUST_WASM".to_string(),
                        function_class_version: "0.1".to_string(),
                        function_class_code: vec![],
                        function_class_outputs: vec![],
                    },
                output_mapping: std::collections::HashMap::new(),
                annotations: std::collections::HashMap::new(),
            }],
            workflow_resources: vec![edgeless_api::workflow_instance::WorkflowResource {
                name: "r1".to_string(),
                class_type: "test-res".to_string(),
                output_mapping: std::collections::HashMap::from([(
                    "test_out".to_string(),
                    "f1".to_string(),
                )]),
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

    let comparison_names = std::collections::HashSet::from([
        instance.domain_mapping[0].name.clone(),
        instance.domain_mapping[1].name.clone(),
    ]);
    let comparison_domains = std::collections::HashSet::from([
        instance.domain_mapping[0].domain_id.clone(),
        instance.domain_mapping[1].domain_id.clone(),
    ]);

    assert_eq!(
        comparison_names,
        std::collections::HashSet::from(["r1".to_string(), "f1".to_string()])
    );
    assert_eq!(
        comparison_domains,
        std::collections::HashSet::from(["domain-1".to_string(), "domain-1".to_string()])
    );

    let mut new_func_id = uuid::Uuid::nil();
    assert!(new_func_id.is_nil());
    if let MockFunctionInstanceEvent::StartFunction((id, _spawn_req)) =
        mock_orc_receiver.try_next().unwrap().unwrap()
    {
        new_func_id = id;
    } else {
        panic!();
    }

    let mut new_res_id = uuid::Uuid::nil();
    assert!(new_res_id.is_nil());
    if let MockFunctionInstanceEvent::StartResource((id, spawn_req)) =
        mock_orc_receiver.try_next().unwrap().unwrap()
    {
        new_res_id = id;
        assert_eq!("test-res".to_string(), spawn_req.class_type);
        assert!(spawn_req.configuration.is_empty());
    } else {
        panic!();
    }

    if let MockFunctionInstanceEvent::Patch(patch_req) =
        mock_orc_receiver.try_next().unwrap().unwrap()
    {
        assert!(!patch_req.function_id.is_nil());
        assert_eq!(1, patch_req.output_mapping.len());
    } else {
        panic!();
    }

    assert!(mock_orc_receiver.try_next().is_err());

    wf_client.stop(instance.workflow_id.clone()).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut got_function_stop = false;
    let mut got_resource_stop = false;
    for _i in 0..2 {
        match mock_orc_receiver.try_next().unwrap().unwrap() {
            MockFunctionInstanceEvent::StopFunction(id) => {
                assert_eq!(new_func_id, id);
                assert!(!got_function_stop);
                got_function_stop = true;
            }
            MockFunctionInstanceEvent::StopResource(id) => {
                assert_eq!(new_res_id, id);
                assert!(!got_resource_stop);
                got_resource_stop = true;
            }
            _ => {
                panic!()
            }
        }
    }

    assert!(mock_orc_receiver.try_next().is_err());
}

#[tokio::test]
async fn function_link_loop_start_stop() {
    let (mut wf_client, mut mock_orc_receiver) = test_setup().await;

    assert!(mock_orc_receiver.try_next().is_err());

    let response = wf_client
        .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
            workflow_functions: vec![
                edgeless_api::workflow_instance::WorkflowFunction {
                    name: "f1".to_string(),
                    function_class_specification:
                        edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: "fc1".to_string(),
                            function_class_type: "RUST_WASM".to_string(),
                            function_class_version: "0.1".to_string(),
                            function_class_code: vec![],
                            function_class_outputs: vec!["output-1".to_string()],
                        },
                    output_mapping: std::collections::HashMap::from([(
                        "output-1".to_string(),
                        "f2".to_string(),
                    )]),
                    annotations: std::collections::HashMap::new(),
                },
                edgeless_api::workflow_instance::WorkflowFunction {
                    name: "f2".to_string(),
                    function_class_specification:
                        edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: "fc2".to_string(),
                            function_class_type: "RUST_WASM".to_string(),
                            function_class_version: "0.1".to_string(),
                            function_class_code: vec![],
                            function_class_outputs: vec!["output-2".to_string()],
                        },
                    output_mapping: std::collections::HashMap::from([(
                        "output-2".to_string(),
                        "f1".to_string(),
                    )]),
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

    let domain_ids: std::collections::HashSet<_> = instance
        .domain_mapping
        .iter()
        .map(|instances| instances.domain_id.clone())
        .collect();
    assert_eq!(domain_ids.len(), 1);
    assert!(domain_ids.contains("domain-1"));

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut new_func1_id = uuid::Uuid::nil();
    assert!(new_func1_id.is_nil());
    if let MockFunctionInstanceEvent::StartFunction((id, spawn_req)) =
        mock_orc_receiver.try_next().unwrap().unwrap()
    {
        new_func1_id = id;
        assert!(spawn_req.annotations.is_empty());
        // TODO check state specifications
    } else {
        panic!();
    }

    let mut new_func2_id = uuid::Uuid::nil();
    assert!(new_func2_id.is_nil());
    if let MockFunctionInstanceEvent::StartFunction((id, spawn_req)) =
        mock_orc_receiver.try_next().unwrap().unwrap()
    {
        new_func2_id = id;
        assert!(spawn_req.annotations.is_empty());
        // TODO check state specifications
    } else {
        panic!();
    }

    let mut label1 = "output-1".to_string();
    let mut label2 = "output-2".to_string();
    if let MockFunctionInstanceEvent::Patch(update_req) =
        mock_orc_receiver.try_next().unwrap().unwrap()
    {
        if new_func1_id != update_req.function_id {
            std::mem::swap(&mut new_func1_id, &mut new_func2_id);
            std::mem::swap(&mut label1, &mut label2);
        }
        assert_eq!(new_func1_id, update_req.function_id);
        assert_eq!(1, update_req.output_mapping.len());
        assert!(update_req.output_mapping.contains_key(&label1));
        let mapping = update_req.output_mapping.get(&label1).unwrap();
        assert!(mapping.node_id.is_nil());
        assert_eq!(new_func2_id, mapping.function_id);
    } else {
        panic!();
    }

    if let MockFunctionInstanceEvent::Patch(update_req) =
        mock_orc_receiver.try_next().unwrap().unwrap()
    {
        assert_eq!(new_func2_id, update_req.function_id);
        assert_eq!(1, update_req.output_mapping.len());
        assert!(update_req.output_mapping.contains_key(&label2));
        let mapping = update_req.output_mapping.get(&label2).unwrap();
        assert!(mapping.node_id.is_nil());
        assert_eq!(new_func1_id, mapping.function_id);
    } else {
        panic!();
    }

    wf_client.stop(instance.workflow_id.clone()).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut fids = std::collections::HashSet::from([new_func1_id, new_func2_id]);
    let stop_res = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::StopFunction(id) = stop_res {
        assert!(fids.remove(&id));
    } else {
        panic!();
    }

    let stop_res2 = mock_orc_receiver.try_next().unwrap().unwrap();
    if let MockFunctionInstanceEvent::StopFunction(id) = stop_res2 {
        assert!(fids.remove(&id));
    } else {
        panic!();
    }
    assert!(fids.is_empty());
}
