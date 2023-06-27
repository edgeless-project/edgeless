use edgeless_api::AgentAPI;

pub struct EdgelessOrcNodeConfig {
    pub node_id: uuid::Uuid,
    pub api_addr: String,
}
pub struct EdgelessOrcSettings {
    pub nodes: Vec<EdgelessOrcNodeConfig>,
}

pub async fn edgeless_orc_main(settings: EdgelessOrcSettings) {
    log::info!("Starting Edgeless Orchestrator");
    let mut client = edgeless_api::grpc_impl::AgentAPIClient::new(&settings.nodes[0].api_addr).await;
    let new_fid = edgeless_api::FunctionId::new(settings.nodes[0].node_id.clone());
    let _ = client
        .start_function_instance(edgeless_api::SpawnFunctionRequest {
            function_id: Some(new_fid.clone()),
            code: edgeless_api::FunctionClassSpecification {
                function_class_id: "example_1".to_string(),
                function_class_type: "RUST_WASM".to_string(),
                function_class_version: "0.1".to_string(),
                function_class_inlude_code: vec![0, 1, 2, 3, 4],
                output_callback_declarations: vec!["cb1".to_string(), "cb2".to_string()],
            },
            output_callback_definitions: std::collections::HashMap::from([
                ("cb1".to_string(), new_fid.clone()),
                ("cb2".to_string(), new_fid.clone()),
            ]),
            return_continuation: new_fid.clone(),
            annotations: std::collections::HashMap::from([("foo".to_string(), "bar".to_string())]),
        })
        .await;
}
