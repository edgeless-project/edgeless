use edgeless_api::con::ControllerAPI;

mod controller;

#[derive(Clone)]
pub struct EdgelessConOrcConfig {
    pub domain_id: String,
    pub api_addr: String,
}
#[derive(Clone)]
pub struct EdgelessConSettings {
    pub controller_grpc_api_addr: String,
    pub orchestrators: Vec<EdgelessConOrcConfig>,
}

pub async fn edgeless_con_main(settings: EdgelessConSettings) {
    log::info!("Starting Edgeless Controller");

    let (mut controller, controller_task) = controller::Controller::new(settings.clone());

    let server_task =
        edgeless_api::grpc_impl::con::WorkflowInstanceAPIServer::run(controller.get_api_client(), settings.controller_grpc_api_addr.clone());

    let test_task = async {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let mut con_client = edgeless_api::grpc_impl::con::ControllerAPIClient::new(&settings.controller_grpc_api_addr).await;
        let mut con_wf_client = con_client.workflow_instance_api();
        let my_wf_id = edgeless_api::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::new_v4(),
        };
        let res = con_wf_client
            .start_workflow_instance(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                workflow_id: my_wf_id.clone(),
                workflow_functions: vec![
                    edgeless_api::workflow_instance::WorkflowFunction {
                        function_alias: "ponger".to_string(),
                        function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: "ponger".to_string(),
                            function_class_type: "RUST_WASM".to_string(),
                            function_class_version: "0.1".to_string(),
                            function_class_inlude_code: std::fs::read("examples/ping_pong/pong/pong.wasm").unwrap(),
                            output_callback_declarations: vec!["pinger".to_string()],
                        },
                        output_callback_definitions: std::collections::HashMap::from([("pinger".to_string(), "pinger".to_string())]),
                        return_continuation: "unused".to_string(),
                        function_annotations: std::collections::HashMap::new(),
                    },
                    edgeless_api::workflow_instance::WorkflowFunction {
                        function_alias: "pinger".to_string(),
                        function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: "pinger".to_string(),
                            function_class_type: "RUST_WASM".to_string(),
                            function_class_version: "0.1".to_string(),
                            function_class_inlude_code: std::fs::read("examples/ping_pong/ping/ping.wasm").unwrap(),
                            output_callback_declarations: vec!["ponger".to_string()],
                        },
                        output_callback_definitions: std::collections::HashMap::from([("ponger".to_string(), "ponger".to_string())]),
                        return_continuation: "unused".to_string(),
                        function_annotations: std::collections::HashMap::new(),
                    },
                ],
                workflow_annotations: std::collections::HashMap::new(),
            })
            .await;
        log::debug!("{:?}", res);
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        con_wf_client.stop_workflow_instance(my_wf_id).await.unwrap();
    };
    futures::join!(controller_task, server_task, test_task);
}
