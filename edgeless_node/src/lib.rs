use futures::join;

pub mod agent;
pub mod data_plane;
pub mod runner_api;
pub mod rust_runner;
pub mod state_management;

#[derive(Clone)]
pub struct EdgelessNodeSettings {
    pub node_id: uuid::Uuid,
    pub agent_grpc_api_addr: String,
}

pub async fn edgeless_node_main(settings: EdgelessNodeSettings) {
    log::info!("Starting Edgeless Node");
    let state_manager = state_management::StateManager::new().await;
    let data_plane = data_plane::DataPlaneChainProvider::new(settings.node_id).await;
    let (mut rust_runner, rust_runner_task) = rust_runner::Runner::new(settings.clone(), data_plane.clone(), state_manager.clone());
    let (mut agent, agent_task) = agent::Agent::new(rust_runner.get_api_client(), settings.clone());
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.agent_grpc_api_addr);

    join!(rust_runner_task, agent_task, agent_api_server);
}
