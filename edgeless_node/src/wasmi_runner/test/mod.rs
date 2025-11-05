// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use futures::SinkExt;
use std::time::Duration;

use crate::base_runtime::RuntimeAPI;
use edgeless_api::common::PatchRequest;
use edgeless_api::function_instance::InstanceId;
use edgeless_dataplane::core::CallRet;
use edgeless_dataplane::handle::DataplaneHandle;
use edgeless_telemetry::telemetry_events::TelemetryEvent;

struct MockTelemetryHandle {
    sender: std::sync::mpsc::Sender<(
        edgeless_telemetry::telemetry_events::TelemetryEvent,
        std::collections::BTreeMap<String, String>,
    )>,
}

impl edgeless_telemetry::telemetry_events::TelemetryHandleAPI for MockTelemetryHandle {
    fn observe(&mut self, event: edgeless_telemetry::telemetry_events::TelemetryEvent, event_tags: std::collections::BTreeMap<String, String>) {
        self.sender.send((event, event_tags)).unwrap();
    }
    fn fork(&mut self, _child_tags: std::collections::BTreeMap<String, String>) -> Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI> {
        Box::new(MockTelemetryHandle { sender: self.sender.clone() })
    }
}

struct MockStateMananger {
    output_mocks: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<uuid::Uuid, String>>>,
    state_set_sender: futures::channel::mpsc::UnboundedSender<(uuid::Uuid, String)>,
}

#[async_trait::async_trait]
impl crate::state_management::StateManagerAPI for MockStateMananger {
    async fn get_handle(
        &mut self,
        _state_policy: edgeless_api::function_instance::StatePolicy,
        state_id: uuid::Uuid,
    ) -> Box<dyn crate::state_management::StateHandleAPI> {
        Box::new(MockStateHandle {
            state_id: state_id,
            output_mocks: self.output_mocks.clone(),
            state_set_sender: self.state_set_sender.clone(),
        })
    }
}

struct MockStateHandle {
    state_id: uuid::Uuid,
    output_mocks: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<uuid::Uuid, String>>>,
    state_set_sender: futures::channel::mpsc::UnboundedSender<(uuid::Uuid, String)>,
}

#[async_trait::async_trait]
impl crate::state_management::StateHandleAPI for MockStateHandle {
    async fn get(&mut self) -> Option<String> {
        self.output_mocks.lock().await.get(&self.state_id).cloned()
    }

    async fn set(&mut self, serialized_state: String) {
        self.state_set_sender.send((self.state_id.clone(), serialized_state)).await.unwrap();
    }
}

fn mock_runtime() -> std::sync::Arc<tokio::sync::Mutex<Box<dyn crate::base_runtime::runtime::GuestAPIHostRegister + Send>>> {
    std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(super::runtime::WasmiRuntime::new())))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn basic_lifecycle() {
    let node_id = uuid::Uuid::new_v4();
    let instance_id = edgeless_api::function_instance::InstanceId::new(node_id);

    let state_manager = Box::new(crate::state_management::StateManager::new().await);
    let dataplane_provider = edgeless_dataplane::handle::DataplaneProvider::new(node_id, "http://127.0.0.1:7002".to_string(), None).await;

    let (telemetry_mock_sender, telemetry_mock_receiver) = std::sync::mpsc::channel::<(
        edgeless_telemetry::telemetry_events::TelemetryEvent,
        std::collections::BTreeMap<String, String>,
    )>();
    let telemetry_handle = Box::new(MockTelemetryHandle {
        sender: telemetry_mock_sender,
    });

    let (mut client, mut rt_task) =
        crate::base_runtime::runtime::create::<super::WASMIFunctionInstance>(dataplane_provider, state_manager, telemetry_handle, mock_runtime());

    tokio::spawn(async move { rt_task.run().await });

    let spawn_req = edgeless_api::function_instance::SpawnFunctionRequest {
        spec: edgeless_api::function_instance::FunctionClassSpecification {
            id: "EXAMPLE_1".to_string(),
            function_type: "RUST_WASM".to_string(),
            version: "0.1".to_string(),
            binary: Some(include_bytes!("../../../../functions/messaging_test/messaging_test.wasm").to_vec()),
            code: None,
            outputs: vec![],
        },
        annotations: std::collections::HashMap::new(),
        state_specification: edgeless_api::function_instance::StateSpecification {
            state_id: instance_id.function_id.clone(),
            state_policy: edgeless_api::function_instance::StatePolicy::Transient,
        },
        workflow_id: "workflow_1".to_string(),
    };

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let _res = client.start(instance_id, spawn_req).await;

    // wait for lifetime events created during spawn
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let res = telemetry_mock_receiver.recv();
    assert!(res.is_ok());
    let (event, _tags) = res.unwrap();
    assert_eq!(
        std::mem::discriminant(&event),
        std::mem::discriminant(&edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInstantiate(
            Duration::from_secs(1)
        ))
    );

    let res2 = telemetry_mock_receiver.recv();
    assert!(res2.is_ok());
    let (event2, _tags2) = res2.unwrap();
    assert_eq!(
        std::mem::discriminant(&event2),
        std::mem::discriminant(&edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionLogEntry(
            edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info,
            "".to_string(),
            "".to_string()
        ))
    );

    let res3 = telemetry_mock_receiver.recv();
    assert!(res3.is_ok());
    let (event3, _tags3) = res3.unwrap();
    assert_eq!(
        std::mem::discriminant(&event3),
        std::mem::discriminant(&edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionInit(Duration::from_secs(
            1
        )))
    );

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let stop_res = client.stop(instance_id.clone()).await;
    assert!(stop_res.is_ok());

    // wait for lifetime events created after stoping it
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let stop_res_1 = telemetry_mock_receiver.recv();
    assert!(stop_res_1.is_ok());
    let (stop_event_1, _stop_tags_1) = stop_res_1.unwrap();
    assert_eq!(
        std::mem::discriminant(&stop_event_1),
        std::mem::discriminant(&edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionLogEntry(
            edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info,
            "".to_string(),
            "".to_string()
        ))
    );

    let stop_res_2 = telemetry_mock_receiver.recv();
    assert!(stop_res_2.is_ok());
    let (stop_event_2, _stop_tags_2) = stop_res_2.unwrap();
    assert_eq!(
        std::mem::discriminant(&stop_event_2),
        std::mem::discriminant(&edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionStop(Duration::from_secs(
            1
        )))
    );

    let stop_res_3 = telemetry_mock_receiver.recv();
    assert!(stop_res_3.is_ok());
    let (stop_event_3, _stop_tags_3) = stop_res_3.unwrap();
    assert_eq!(
        std::mem::discriminant(&stop_event_3),
        std::mem::discriminant(&edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionExit(
            edgeless_telemetry::telemetry_events::FunctionExitStatus::Ok
        ))
    );
}

type TelemetryReceiver = std::sync::mpsc::Receiver<(
    edgeless_telemetry::telemetry_events::TelemetryEvent,
    std::collections::BTreeMap<String, String>,
)>;

async fn messaging_test_setup() -> (InstanceId, DataplaneHandle, InstanceId, DataplaneHandle, InstanceId, TelemetryReceiver) {
    // shared?
    let node_id = uuid::Uuid::new_v4();
    let instance_id = edgeless_api::function_instance::InstanceId::new(node_id);

    let state_manager = Box::new(crate::state_management::StateManager::new().await);
    let mut dataplane_provider = edgeless_dataplane::handle::DataplaneProvider::new(node_id, "http://127.0.0.1:7002".to_string(), None).await;

    // shared insert
    let test_peer_fid = edgeless_api::function_instance::InstanceId::new(node_id);
    let test_peer_handle = dataplane_provider.get_handle_for(test_peer_fid.clone()).await;

    let next_fid = edgeless_api::function_instance::InstanceId::new(node_id);
    let next_handle = dataplane_provider.get_handle_for(next_fid.clone()).await;
    // end shared insert

    let (telemetry_mock_sender, telemetry_mock_receiver) = std::sync::mpsc::channel::<(
        edgeless_telemetry::telemetry_events::TelemetryEvent,
        std::collections::BTreeMap<String, String>,
    )>();
    let telemetry_handle = Box::new(MockTelemetryHandle {
        sender: telemetry_mock_sender,
    });

    let (mut client, mut rt_task) =
        crate::base_runtime::runtime::create::<super::WASMIFunctionInstance>(dataplane_provider, state_manager, telemetry_handle, mock_runtime());

    tokio::spawn(async move { rt_task.run().await });

    let spawn_req = edgeless_api::function_instance::SpawnFunctionRequest {
        spec: edgeless_api::function_instance::FunctionClassSpecification {
            id: "EXAMPLE_1".to_string(),
            function_type: "RUST_WASM".to_string(),
            version: "0.1".to_string(),
            binary: Some(include_bytes!("../../../../functions/messaging_test/messaging_test.wasm").to_vec()),
            code: None,
            outputs: vec!["test".to_string()],
        },
        annotations: std::collections::HashMap::new(),
        state_specification: edgeless_api::function_instance::StateSpecification {
            state_id: instance_id.function_id.clone(),
            state_policy: edgeless_api::function_instance::StatePolicy::Transient,
        },
        workflow_id: "workflow_1".to_string(),
    };

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let res = client.start(instance_id, spawn_req).await;
    assert!(res.is_ok());

    let res = client
        .patch(PatchRequest {
            function_id: instance_id.function_id.clone(),
            output_mapping: std::collections::HashMap::from([("test".to_string(), next_fid.clone())]),
        })
        .await;

    assert!(res.is_ok());

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    (
        instance_id,
        test_peer_handle,
        test_peer_fid,
        next_handle,
        next_fid,
        telemetry_mock_receiver,
    )
}

async fn is_telemetry_event_transfer(receiver: &mut TelemetryReceiver) -> bool {
    let telemetry_event = receiver.try_recv();
    assert!(telemetry_event.is_ok());
    let (telemetry_event, _tags) = telemetry_event.unwrap();
    std::mem::discriminant(&telemetry_event) == std::mem::discriminant(&TelemetryEvent::FunctionTransfer(tokio::time::Duration::ZERO))
}

async fn is_telemetry_event_invocation_complete(receiver: &mut TelemetryReceiver) -> bool {
    let telemetry_event = receiver.try_recv();
    assert!(telemetry_event.is_ok());
    let (telemetry_event, _tags) = telemetry_event.unwrap();
    std::mem::discriminant(&telemetry_event) == std::mem::discriminant(&TelemetryEvent::FunctionInvocationCompleted(tokio::time::Duration::ZERO))
}

// test input (host-> function): cast
// We assume this works after this test and trigger the different outputs using casts.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn messaging_cast_raw_input() {
    let (instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf0002fu128, 0x42a42bdecaf0003cu64);

    test_peer_handle.send(instance_id.clone(), "some_message".to_string(), &metad_1).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test output (i.e. the method available to the function): cast
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn messaging_cast_raw_output() {
    let (instance_id, mut test_peer_handle, test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf0003du128, 0x42a42bdecaf0003eu64);

    println!("Expected: {}", test_peer_fid.node_id);

    test_peer_handle
        .send(instance_id.clone(), "test_cast_raw_output".to_string(), &metad_1)
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);
    assert!(is_telemetry_event_invocation_complete(&mut telemetry_mock_receiver).await);
    assert!(telemetry_mock_receiver.try_recv().is_err());

    println!("Wait");
    let test_message = test_peer_handle.receive_next().await;
    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(
        test_message.message,
        edgeless_dataplane::core::Message::Cast("cast_raw_output".to_string())
    );
    assert_eq!(test_message.metadata, metad_1);
}

// test output: call
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn messaging_call_raw_output() {
    let (instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf0003fu128, 0x42a42bdecaf00040u64);

    test_peer_handle
        .send(instance_id.clone(), "test_call_raw_output".to_string(), &metad_1)
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);

    // This won't have completed here.
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = test_peer_handle.receive_next().await;
    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(
        test_message.message,
        edgeless_dataplane::core::Message::Call("call_raw_output".to_string())
    );
    assert_eq!(&test_message.metadata, &metad_1);

    test_peer_handle
        .reply(test_message.source_id, test_message.channel_id, CallRet::NoReply, &test_message.metadata)
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(is_telemetry_event_invocation_complete(&mut telemetry_mock_receiver).await);
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test output: delayed_cast
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn messaging_delayed_cast_output() {
    let (instance_id, mut test_peer_handle, _test_peer_fid, mut next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00041u128, 0x42a42bdecaf00042u64);

    test_peer_handle
        .send(instance_id.clone(), "test_delayed_cast_output".to_string(), &metad_1)
        .await;
    let start = tokio::time::Instant::now();

    let test_message = next_handle.receive_next().await;
    assert!(start.elapsed() >= Duration::from_millis(100));

    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(
        test_message.message,
        edgeless_dataplane::core::Message::Cast("delayed_cast_output".to_string())
    );
    assert_eq!(&test_message.metadata, &metad_1);

    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);
    assert!(is_telemetry_event_invocation_complete(&mut telemetry_mock_receiver).await);
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test output: cast
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn messaging_cast_output() {
    let (instance_id, mut test_peer_handle, _test_peer_fid, mut next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00043u128, 0x42a42bdecaf00044u64);

    test_peer_handle.send(instance_id.clone(), "test_cast_output".to_string(), &metad_1).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);
    assert!(is_telemetry_event_invocation_complete(&mut telemetry_mock_receiver).await);
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = next_handle.receive_next().await;
    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(test_message.message, edgeless_dataplane::core::Message::Cast("cast_output".to_string()));
    assert_eq!(&test_message.metadata, &metad_1);
}

// test output: call
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn messaging_call_output() {
    let (instance_id, mut test_peer_handle, _test_peer_fid, mut next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00045u128, 0x42a42bdecaf00046u64);

    test_peer_handle.send(instance_id.clone(), "test_call_output".to_string(), &metad_1).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);

    // This won't have completed here.
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = next_handle.receive_next().await;
    assert_eq!(test_message.source_id, instance_id);
    assert_eq!(test_message.message, edgeless_dataplane::core::Message::Call("call_output".to_string()));
    assert_eq!(&test_message.metadata, &metad_1);

    next_handle
        .reply(test_message.source_id, test_message.channel_id, CallRet::NoReply, &metad_1)
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(is_telemetry_event_invocation_complete(&mut telemetry_mock_receiver).await);
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test call-interaction: Noreply
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn messaging_call_raw_input_noreply() {
    let (instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00047u128, 0x42a42bdecaf00048u64);

    let ret = test_peer_handle.call(instance_id.clone(), "some_cast".to_string(), &metad_1).await;
    assert_eq!(ret, CallRet::NoReply);

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);
    assert!(is_telemetry_event_invocation_complete(&mut telemetry_mock_receiver).await);
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test call-interaction: Reply
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn messaging_call_raw_input_reply() {
    let (instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00049u128, 0x42a42bdecaf0004au64);

    let ret = test_peer_handle.call(instance_id.clone(), "test_ret".to_string(), &metad_1).await;
    assert_eq!(ret, CallRet::Reply("test_reply".to_string()));

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);
    assert!(is_telemetry_event_invocation_complete(&mut telemetry_mock_receiver).await);
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test call-interaction: Error
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn messaging_call_raw_input_err() {
    let (instance_id, mut test_peer_handle, _test_peer_fid, _next_handle, _next_fid, mut telemetry_mock_receiver) = messaging_test_setup().await;
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf0004bu128, 0x42a42bdecaf0004cu64);

    let ret = test_peer_handle.call(instance_id.clone(), "test_err".to_string(), &metad_1).await;
    assert_eq!(ret, CallRet::Err);

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);
    assert!(is_telemetry_event_invocation_complete(&mut telemetry_mock_receiver).await);
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn state_management() {
    let node_id = uuid::Uuid::new_v4();
    let instance_id = edgeless_api::function_instance::InstanceId::new(node_id);
    let instance_id_another = edgeless_api::function_instance::InstanceId::new(node_id);
    let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf0004du128, 0x42a42bdecaf0004eu64);

    let output_mocks = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let (state_mock_sender, mut state_mock_receiver) = futures::channel::mpsc::unbounded::<(uuid::Uuid, String)>();
    let mock_state_manager = Box::new(MockStateMananger {
        state_set_sender: state_mock_sender,
        output_mocks: output_mocks.clone(),
    });

    let mut dataplane_provider = edgeless_dataplane::handle::DataplaneProvider::new(node_id, "http://127.0.0.1:7002".to_string(), None).await;

    let (telemetry_mock_sender, mut telemetry_mock_receiver) = std::sync::mpsc::channel::<(
        edgeless_telemetry::telemetry_events::TelemetryEvent,
        std::collections::BTreeMap<String, String>,
    )>();
    let telemetry_handle = Box::new(MockTelemetryHandle {
        sender: telemetry_mock_sender,
    });

    let test_peer_fid = edgeless_api::function_instance::InstanceId::new(node_id);
    let mut test_peer_handle = dataplane_provider.get_handle_for(test_peer_fid.clone()).await;

    let (mut client, mut rt_task) = crate::base_runtime::runtime::create::<super::WASMIFunctionInstance>(
        dataplane_provider,
        mock_state_manager,
        telemetry_handle,
        mock_runtime(),
    );

    tokio::spawn(async move { rt_task.run().await });

    let spawn_req = edgeless_api::function_instance::SpawnFunctionRequest {
        spec: edgeless_api::function_instance::FunctionClassSpecification {
            id: "EXAMPLE_1".to_string(),
            function_type: "RUST_WASM".to_string(),
            version: "0.1".to_string(),
            binary: Some(include_bytes!("../../../../functions/state_test/state_test.wasm").to_vec()),
            code: None,
            outputs: Vec::new(),
        },
        annotations: std::collections::HashMap::new(),
        state_specification: edgeless_api::function_instance::StateSpecification {
            state_id: instance_id.function_id.clone(),
            state_policy: edgeless_api::function_instance::StatePolicy::Transient,
        },
        workflow_id: "workflow_1".to_string(),
    };

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let res = client.start(instance_id, spawn_req.clone()).await;
    assert!(res.is_ok());

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());

    let (init_log_event, _init_log_tags) = telemetry_mock_receiver.try_recv().unwrap();
    assert_eq!(
        init_log_event,
        TelemetryEvent::FunctionLogEntry(
            edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info,
            "state_test".to_string(),
            "no_state".to_string()
        )
    );

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    // trigger sync
    test_peer_handle
        .send(instance_id.clone(), "test_cast_raw_output".to_string(), &metad_1)
        .await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let (state_set_id, state_set_value) = state_mock_receiver.try_next().unwrap().unwrap();

    assert_eq!(state_set_id, instance_id.function_id.clone());
    assert_eq!(state_set_value, "new_state".to_string());

    assert!(is_telemetry_event_transfer(&mut telemetry_mock_receiver).await);
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let res = client.stop(instance_id.clone()).await;
    assert!(res.is_ok());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    // now we try starting with state

    output_mocks
        .lock()
        .await
        .insert(instance_id.function_id.clone(), "existing_state".to_string());

    // TODO(raphaelhetzel) InstanceId reuse leads to problems that need to be fixed.
    let res2 = client.start(instance_id_another, spawn_req).await;
    assert!(res2.is_ok());

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());

    let (init_log_event2, _init_log_tags2) = telemetry_mock_receiver.try_recv().unwrap();
    assert_eq!(
        init_log_event2,
        TelemetryEvent::FunctionLogEntry(
            edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info,
            "edgeless_test_state".to_string(),
            "existing_state".to_string()
        )
    );

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}
