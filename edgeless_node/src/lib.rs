use futures::join;

pub mod agent;
pub mod runner_api;
pub mod rust_runner;
pub mod state_management;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EdgelessNodeSettings {
    pub node_id: uuid::Uuid,
    pub agent_url: String,
    pub invocation_url: String,
    pub peers: Vec<edgeless_dataplane::EdgelessDataplaneSettingsPeer>,
}

pub async fn edgeless_node_main(settings: EdgelessNodeSettings) {
    log::info!("Starting Edgeless Node");
    log::debug!("Settings: {:?}", settings);
    let state_manager = state_management::StateManager::new().await;
    let data_plane =
        edgeless_dataplane::DataPlaneChainProvider::new(settings.node_id.clone(), settings.invocation_url.clone(), settings.peers.clone()).await;
    let (mut rust_runner, rust_runner_task) = rust_runner::Runner::new(settings.clone(), data_plane.clone(), state_manager.clone());
    let (mut agent, agent_task) = agent::Agent::new(rust_runner.get_api_client(), settings.clone());
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.agent_url);

    join!(rust_runner_task, agent_task, agent_api_server);
}
