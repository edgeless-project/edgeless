use edgeless_agent_api::AgentAPI;

pub struct EdgelessOrcNodeConfig {
    pub node_id: uuid::Uuid,
    pub api_addr: String,
}
pub struct EdgelessOrcSettings {
    pub nodes: Vec<EdgelessOrcNodeConfig>,
}

pub async fn edgeless_orc_main(settings: EdgelessOrcSettings) {
    log::info!("Starting Edgeless Orchestrator");
    let mut client = edgeless_agent_api::grpc_impl::AgentAPIClient::new(&settings.nodes[0].api_addr).await;
    let _ = client
        .spawn(edgeless_agent_api::SpawnFunctionRequest {
            function_id: Some(edgeless_agent_api::FunctionId::new(settings.nodes[0].node_id.clone())),
            code: "foo".to_string(),
        })
        .await;
}
