mod orchestration_logic;
mod orchestrator;

use futures::join;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcNodeConfig {
    pub node_id: uuid::Uuid,
    pub agent_url: String,
}
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcSettings {
    pub domain_id: String,
    pub orchestrator_url: String,
    pub nodes: Vec<EdgelessOrcNodeConfig>,
    pub orchestration_strategy: OrchestrationStrategy,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum OrchestrationStrategy {
    /// Random strategy utilizes a random number generator to select the worker
    /// node where a function instance is started. It is the default strategy.
    Random,
    /// RoundRobin traverses the list of available worker nodes in a fixed order
    /// and places new function instances according to this fixed order.
    RoundRobin,
}

pub async fn edgeless_orc_main(settings: EdgelessOrcSettings) {
    log::info!("Starting Edgeless Orchestrator");
    log::debug!("Settings: {:?}", settings);

    let (mut orchestrator, orchestrator_task) = orchestrator::Orchestrator::new(settings.clone());

    let orchestrator_server = edgeless_api::grpc_impl::orc::OrchestratorAPIServer::run(orchestrator.get_api_client(), settings.orchestrator_url);

    join!(orchestrator_task, orchestrator_server);
}

pub fn edgeless_orc_default_conf() -> String {
    String::from(
        r##"domain_id = "domain-1"
orchestrator_url = "http://127.0.0.1:7011"
orchestration_strategy = "Random"
nodes = [
        {node_id = "fda6ce79-46df-4f96-a0d2-456f720f606c", agent_url = "http://127.0.0.1:7001" }
]
"##,
    )
}
