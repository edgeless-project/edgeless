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
        let res = con_wf_client
            .start_workflow_instance(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                workflow_id: edgeless_api::workflow_instance::WorkflowId {
                    workflow_id: uuid::Uuid::new_v4(),
                },
                workflow_functions: vec![edgeless_api::workflow_instance::WorkflowFunction {
                    function_alias: "test_fun_1".to_string(),
                    function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                        function_class_id: "example_1".to_string(),
                        function_class_type: "RUST_WASM".to_string(),
                        function_class_version: "0.1".to_string(),
                        function_class_inlude_code: vec![0, 1, 2, 3, 4],
                        output_callback_declarations: vec!["cb1".to_string(), "cb2".to_string()],
                    },
                    output_callback_definitions: std::collections::HashMap::from([
                        ("cb1".to_string(), "test_fun_1".to_string()),
                        ("cb2".to_string(), "test_fun_1".to_string()),
                    ]),
                    return_continuation: "test_fun_1".to_string(),
                    function_annotations: std::collections::HashMap::from([("foo".to_string(), "bar".to_string())]),
                }],
                workflow_annotations: std::collections::HashMap::from([("bar".to_string(), "baz".to_string())]),
            })
            .await;
        log::debug!("{:?}", res.unwrap());
    };
    futures::join!(controller_task, server_task, test_task);
}
