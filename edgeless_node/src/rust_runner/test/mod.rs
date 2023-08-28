use std::time::Duration;

use edgeless_api::function_instance::FunctionId;
use edgeless_dataplane::handle::DataplaneHandle;
use edgeless_telemetry::telemetry_events::TelemetryEvent;

use crate::rust_runner::*;

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
impl state_management::StateManagerAPI for MockStateMananger {
    async fn get_handle(
        &mut self,
        _state_policy: edgeless_api::function_instance::StatePolicy,
        state_id: uuid::Uuid,
    ) -> Box<dyn state_management::StateHandleAPI> {
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
impl state_management::StateHandleAPI for MockStateHandle {
    async fn get(&mut self) -> Option<String> {
        self.output_mocks.lock().await.get(&self.state_id).cloned()
    }

    async fn set(&mut self, serialized_state: String) {
        self.state_set_sender.send((self.state_id.clone(), serialized_state)).await.unwrap();
    }
}

#[tokio::test]
async fn basic_lifecycle() {
    let node_id = uuid::Uuid::new_v4();
    let fid = edgeless_api::function_instance::FunctionId::new(node_id);

    let state_manager = Box::new(crate::state_management::StateManager::new().await);
    let dataplane_provider = edgeless_dataplane::handle::DataplaneProvider::new(node_id, "http://127.0.0.1:7002".to_string(), vec![]).await;

    let (telemetry_mock_sender, telemetry_mock_receiver) = std::sync::mpsc::channel::<(
        edgeless_telemetry::telemetry_events::TelemetryEvent,
        std::collections::BTreeMap<String, String>,
    )>();
    let telemetry_handle = Box::new(MockTelemetryHandle {
        sender: telemetry_mock_sender,
    });

    let (mut rt, rt_task) = Runner::new(dataplane_provider, state_manager, telemetry_handle);

    tokio::spawn(rt_task);

    let mut client = rt.get_api_client();

    let spawn_req = edgeless_api::function_instance::SpawnFunctionRequest {
        function_id: Some(fid.clone()),
        code: edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "EXAMPLE_1".to_string(),
            function_class_type: "RUST_WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_inlude_code: include_bytes!("fixtures/messaging_test.wasm").to_vec(),
            output_callback_declarations: vec![],
        },
        output_callback_definitions: std::collections::HashMap::new(),
        return_continuation: fid.clone(),
        annotations: std::collections::HashMap::new(),
        state_specification: edgeless_api::function_instance::StateSpecification {
            state_id: fid.function_id.clone(),
            state_policy: edgeless_api::function_instance::StatePolicy::Transient,
        },
    };

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let _res = client.start(spawn_req).await;

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

    let stop_res = client.stop(fid.clone()).await;
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
        std::mem::discriminant(&edgeless_telemetry::telemetry_events::TelemetryEvent::FunctionExit)
    );
}

async fn messaging_test_setup() -> (
    FunctionId,
    DataplaneHandle,
    FunctionId,
    DataplaneHandle,
    FunctionId,
    std::sync::mpsc::Receiver<(
        edgeless_telemetry::telemetry_events::TelemetryEvent,
        std::collections::BTreeMap<String, String>,
    )>,
) {
    // shared?
    let node_id = uuid::Uuid::new_v4();
    let fid = edgeless_api::function_instance::FunctionId::new(node_id);

    let state_manager = Box::new(crate::state_management::StateManager::new().await);
    let mut dataplane_provider = edgeless_dataplane::handle::DataplaneProvider::new(node_id, "http://127.0.0.1:7002".to_string(), vec![]).await;

    // shared insert
    let test_peer_fid = edgeless_api::function_instance::FunctionId::new(node_id);
    let test_peer_handle = dataplane_provider.get_handle_for(test_peer_fid.clone()).await;

    let alias_fid = edgeless_api::function_instance::FunctionId::new(node_id);
    let alias_handle = dataplane_provider.get_handle_for(alias_fid.clone()).await;
    // end shared insert

    let (telemetry_mock_sender, telemetry_mock_receiver) = std::sync::mpsc::channel::<(
        edgeless_telemetry::telemetry_events::TelemetryEvent,
        std::collections::BTreeMap<String, String>,
    )>();
    let telemetry_handle = Box::new(MockTelemetryHandle {
        sender: telemetry_mock_sender,
    });

    let (mut rt, rt_task) = Runner::new(dataplane_provider, state_manager, telemetry_handle);

    tokio::spawn(rt_task);

    let mut client = rt.get_api_client();

    let spawn_req = edgeless_api::function_instance::SpawnFunctionRequest {
        function_id: Some(fid.clone()),
        code: edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "EXAMPLE_1".to_string(),
            function_class_type: "RUST_WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_inlude_code: include_bytes!("fixtures/messaging_test.wasm").to_vec(),
            output_callback_declarations: vec!["test_alias".to_string()],
        },
        output_callback_definitions: std::collections::HashMap::from([("test_alias".to_string(), alias_fid.clone())]),
        return_continuation: fid.clone(),
        annotations: std::collections::HashMap::new(),
        state_specification: edgeless_api::function_instance::StateSpecification {
            state_id: fid.function_id.clone(),
            state_policy: edgeless_api::function_instance::StatePolicy::Transient,
        },
    };

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let res = client.start(spawn_req).await;
    assert!(res.is_ok());

    tokio::time::sleep(Duration::from_millis(100)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    (fid, test_peer_handle, test_peer_fid, alias_handle, alias_fid, telemetry_mock_receiver)
}

// test input (host-> function): cast
// We assume this works after this test and trigger the different outputs using casts.
#[tokio::test]
async fn messaging_cast_input() {
    let (fid, mut test_peer_handle, _test_peer_fid, _alias_handle, _alias_fid, telemetry_mock_receiver) = messaging_test_setup().await;
    test_peer_handle.send(fid.clone(), "some_message".to_string()).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test output (i.e. the method available to the function): cast
#[tokio::test]
async fn messaging_cast_output() {
    let (fid, mut test_peer_handle, _test_peer_fid, _alias_handle, _alias_fid, telemetry_mock_receiver) = messaging_test_setup().await;

    test_peer_handle.send(fid.clone(), "test_cast_output".to_string()).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = test_peer_handle.receive_next().await;
    assert_eq!(test_message.source_id, fid);
    assert_eq!(test_message.message, edgeless_dataplane::core::Message::Cast("cast_output".to_string()));
}

// test output: call
#[tokio::test]
async fn messaging_call_output() {
    let (fid, mut test_peer_handle, _test_peer_fid, _alias_handle, _alias_fid, telemetry_mock_receiver) = messaging_test_setup().await;

    test_peer_handle.send(fid.clone(), "test_call_output".to_string()).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // This won't have completed here.
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = test_peer_handle.receive_next().await;
    assert_eq!(test_message.source_id, fid);
    assert_eq!(test_message.message, edgeless_dataplane::core::Message::Call("call_output".to_string()));

    test_peer_handle
        .reply(test_message.source_id, test_message.channel_id, CallRet::NoReply)
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test output: delayed_cast
#[tokio::test]
async fn messaging_delayed_cast_output() {
    let (fid, mut test_peer_handle, _test_peer_fid, _alias_handle, _alias_fid, telemetry_mock_receiver) = messaging_test_setup().await;

    test_peer_handle.send(fid.clone(), "test_delayed_cast_output".to_string()).await;
    let start = tokio::time::Instant::now();

    let test_message = test_peer_handle.receive_next().await;
    assert!(start.elapsed() >= Duration::from_millis(100));

    assert_eq!(test_message.source_id, fid);
    assert_eq!(
        test_message.message,
        edgeless_dataplane::core::Message::Cast("delayed_cast_output".to_string())
    );

    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test output: cast_alias
#[tokio::test]
async fn messaging_cast_alias_output() {
    let (fid, mut test_peer_handle, _test_peer_fid, mut alias_handle, _alias_fid, telemetry_mock_receiver) = messaging_test_setup().await;

    test_peer_handle.send(fid.clone(), "test_cast_alias_output".to_string()).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = alias_handle.receive_next().await;
    assert_eq!(test_message.source_id, fid);
    assert_eq!(
        test_message.message,
        edgeless_dataplane::core::Message::Cast("cast_alias_output".to_string())
    );
}

// test output: call alias
#[tokio::test]
async fn messaging_call_alias_output() {
    let (fid, mut test_peer_handle, _test_peer_fid, mut alias_handle, _alias_fid, telemetry_mock_receiver) = messaging_test_setup().await;

    test_peer_handle.send(fid.clone(), "test_call_alias_output".to_string()).await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // This won't have completed here.
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let test_message = alias_handle.receive_next().await;
    assert_eq!(test_message.source_id, fid);
    assert_eq!(
        test_message.message,
        edgeless_dataplane::core::Message::Call("call_alias_output".to_string())
    );

    alias_handle
        .reply(test_message.source_id, test_message.channel_id, CallRet::NoReply)
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test call-interaction: Noreply
#[tokio::test]
async fn messaging_call_input_noreply() {
    let (fid, mut test_peer_handle, _test_peer_fid, _alias_handle, _alias_fid, telemetry_mock_receiver) = messaging_test_setup().await;

    let ret = test_peer_handle.call(fid.clone(), "some_cast".to_string()).await;
    assert_eq!(ret, CallRet::NoReply);

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test call-interaction: Reply
#[tokio::test]
async fn messaging_call_input_reply() {
    let (fid, mut test_peer_handle, _test_peer_fid, _alias_handle, _alias_fid, telemetry_mock_receiver) = messaging_test_setup().await;

    let ret = test_peer_handle.call(fid.clone(), "test_ret".to_string()).await;
    assert_eq!(ret, CallRet::Reply("test_reply".to_string()));

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

// test call-interaction: Error
#[tokio::test]
async fn messaging_call_input_err() {
    let (fid, mut test_peer_handle, _test_peer_fid, _alias_handle, _alias_fid, telemetry_mock_receiver) = messaging_test_setup().await;

    let ret = test_peer_handle.call(fid.clone(), "test_err".to_string()).await;
    assert_eq!(ret, CallRet::Err);

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());
}

#[tokio::test]
async fn state_management() {
    env_logger::init();

    let node_id = uuid::Uuid::new_v4();
    let fid = edgeless_api::function_instance::FunctionId::new(node_id);
    let fid2 = edgeless_api::function_instance::FunctionId::new(node_id);

    let output_mocks = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let (state_mock_sender, mut state_mock_receiver) = futures::channel::mpsc::unbounded::<(uuid::Uuid, String)>();
    let mock_state_manager = Box::new(MockStateMananger {
        state_set_sender: state_mock_sender,
        output_mocks: output_mocks.clone(),
    });

    let mut dataplane_provider = edgeless_dataplane::handle::DataplaneProvider::new(node_id, "http://127.0.0.1:7002".to_string(), vec![]).await;

    let (telemetry_mock_sender, telemetry_mock_receiver) = std::sync::mpsc::channel::<(
        edgeless_telemetry::telemetry_events::TelemetryEvent,
        std::collections::BTreeMap<String, String>,
    )>();
    let telemetry_handle = Box::new(MockTelemetryHandle {
        sender: telemetry_mock_sender,
    });

    let test_peer_fid = edgeless_api::function_instance::FunctionId::new(node_id);
    let mut test_peer_handle = dataplane_provider.get_handle_for(test_peer_fid.clone()).await;

    let (mut rt, rt_task) = Runner::new(dataplane_provider, mock_state_manager, telemetry_handle);

    tokio::spawn(rt_task);

    let mut client = rt.get_api_client();

    let mut spawn_req = edgeless_api::function_instance::SpawnFunctionRequest {
        function_id: Some(fid.clone()),
        code: edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "EXAMPLE_1".to_string(),
            function_class_type: "RUST_WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_inlude_code: include_bytes!("fixtures/state_test.wasm").to_vec(),
            output_callback_declarations: Vec::new(),
        },
        output_callback_definitions: std::collections::HashMap::new(),
        return_continuation: fid.clone(),
        annotations: std::collections::HashMap::new(),
        state_specification: edgeless_api::function_instance::StateSpecification {
            state_id: fid.function_id.clone(),
            state_policy: edgeless_api::function_instance::StatePolicy::Transient,
        },
    };

    assert!(telemetry_mock_receiver.try_recv().is_err());

    let res = client.start(spawn_req.clone()).await;
    assert!(res.is_ok());

    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(telemetry_mock_receiver.try_recv().is_ok());

    let (init_log_event, _init_log_tags) = telemetry_mock_receiver.try_recv().unwrap();
    assert_eq!(
        init_log_event,
        TelemetryEvent::FunctionLogEntry(
            edgeless_telemetry::telemetry_events::TelemetryLogLevel::Info,
            "edgeless_test_state".to_string(),
            "no_state".to_string()
        )
    );

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    // trigger sync
    test_peer_handle.send(fid.clone(), "test_cast_output".to_string()).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let (state_set_id, state_set_value) = state_mock_receiver.try_next().unwrap().unwrap();

    assert_eq!(state_set_id, fid.function_id.clone());
    assert_eq!(state_set_value, "new_state".to_string());

    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    let res = client.stop(fid.clone()).await;
    assert!(res.is_ok());

    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_ok());
    assert!(telemetry_mock_receiver.try_recv().is_err());

    // now we try starting with state

    output_mocks.lock().await.insert(fid.function_id.clone(), "existing_state".to_string());

    // TODO(raphaelhetzel) FunctionId reuse leads to problems that need to be fixed.
    spawn_req.function_id = Some(fid2);

    let res2 = client.start(spawn_req).await;
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
