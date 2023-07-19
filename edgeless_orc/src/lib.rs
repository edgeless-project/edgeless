mod orchestrator;

use futures::join;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EdgelessOrcNodeConfig {
    pub node_id: uuid::Uuid,
    pub agent_url: String,
}
#[derive(Debug, Clone, serde::Deserialize)]
pub struct EdgelessOrcSettings {
    pub domain_id: String,
    pub orchestrator_url: String,
    pub nodes: Vec<EdgelessOrcNodeConfig>,
}

pub async fn edgeless_orc_main(settings: EdgelessOrcSettings) {
    log::info!("Starting Edgeless Orchestrator");
    log::debug!("Settings: {:?}", settings);

    let (mut orchestrator, orchestrator_task) = orchestrator::Orchestrator::new(settings.clone());

    let orchestrator_server = edgeless_api::grpc_impl::orc::OrchestratorAPIServer::run(orchestrator.get_api_client(), settings.orchestrator_url);

    join!(orchestrator_task, orchestrator_server);
}
