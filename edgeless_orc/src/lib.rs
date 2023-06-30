mod orchestrator;

use futures::join;

#[derive(Clone)]
pub struct EdgelessOrcNodeConfig {
    pub node_id: uuid::Uuid,
    pub api_addr: String,
}
#[derive(Clone)]
pub struct EdgelessOrcSettings {
    pub orchestrator_grpc_api_addr: String,
    pub nodes: Vec<EdgelessOrcNodeConfig>,
}

pub async fn edgeless_orc_main(settings: EdgelessOrcSettings) {
    log::info!("Starting Edgeless Orchestrator");

    let (mut orchestrator, orchestrator_task) = orchestrator::Orchestrator::new(settings.clone());

    let orchestrator_server =
        edgeless_api::grpc_impl::orc::OrchestratorAPIServer::run(orchestrator.get_api_client(), settings.orchestrator_grpc_api_addr);

    join!(orchestrator_task, orchestrator_server);
}
