use edgeless_api::orc::OrchestratorAPI;
use futures::join;

pub mod agent;
pub mod runner_api;
pub mod state_management;
pub mod wasm_runner;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeSettings {
    pub node_id: uuid::Uuid,
    pub agent_url: String,
    pub invocation_url: String,
    pub metrics_url: String,
    pub orchestrator_url: String,
}

async fn register_node(settings: &EdgelessNodeSettings) {
    log::info!("Registering this node '{}' on e-ORC {}", &settings.node_id, &settings.orchestrator_url);
    let mut orc_client = edgeless_api::grpc_impl::orc::OrchestratorAPIClient::new(&settings.orchestrator_url).await;
    match orc_client
        .function_instance_api()
        .update_node(edgeless_api::function_instance::UpdateNodeRequest::Registration(
            settings.node_id.clone(),
            settings.agent_url.clone(),
            settings.invocation_url.clone(),
        ))
        .await
    {
        Ok(res) => match res {
            edgeless_api::function_instance::UpdateNodeResponse::ResponseError(err) => panic!("could not register to e-ORC: {}", err),
            edgeless_api::function_instance::UpdateNodeResponse::Accepted => {
                log::info!("this node '{}' registered to e-ORC '{}'", &settings.node_id, &settings.orchestrator_url)
            }
        },
        Err(err) => panic!("channel error when registering to e-ORC: {}", err),
    }
}

pub async fn edgeless_node_main(settings: EdgelessNodeSettings) {
    log::info!("Starting Edgeless Node");
    log::debug!("Settings: {:?}", settings);
    let state_manager = Box::new(state_management::StateManager::new().await);
    let data_plane = edgeless_dataplane::handle::DataplaneProvider::new(settings.node_id.clone(), settings.invocation_url.clone()).await;
    let telemetry_provider = edgeless_telemetry::telemetry_events::TelemetryProcessor::new(settings.metrics_url.clone())
        .await
        .expect(&format!("could not build the telemetry provider at URL {}", &settings.metrics_url));
    let (rust_runner_client, rust_runner_task) = wasm_runner::runner::Runner::new(
        data_plane.clone(),
        state_manager.clone(),
        Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
            ("FUNCTION_TYPE".to_string(), "RUST_WASM".to_string()),
            ("NODE_ID".to_string(), settings.node_id.to_string()),
        ]))),
    );
    let (mut agent, agent_task) = agent::Agent::new(Box::new(rust_runner_client.clone()), settings.clone(), data_plane.clone());
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.agent_url.clone());

    join!(rust_runner_task, agent_task, agent_api_server, register_node(&settings));
}

pub fn edgeless_node_default_conf() -> String {
    String::from(
        r##"node_id = "fda6ce79-46df-4f96-a0d2-456f720f606c"
agent_url = "http://127.0.0.1:7001"
invocation_url = "http://127.0.0.1:7002"
metrics_url = "http://127.0.0.1:7003"
orchestrator_url = "http://127.0.0.1:7011"
"##,
    )
}
