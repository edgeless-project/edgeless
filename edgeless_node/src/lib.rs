use edgeless_api::orc::OrchestratorAPI;
pub mod agent;
pub mod base_runtime;
pub mod resources;
pub mod state_management;
pub mod wasm_runner;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeSettings {
    pub node_id: uuid::Uuid,
    pub agent_url: String,
    pub invocation_url: String,
    pub metrics_url: String,
    pub orchestrator_url: String,
    pub http_ingress_url: String,
}

async fn register_node(
    settings: &EdgelessNodeSettings,
    resource_provider_specifications: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
) {
    log::info!("Registering this node '{}' on e-ORC {}", &settings.node_id, &settings.orchestrator_url);
    match edgeless_api::grpc_impl::orc::OrchestratorAPIClient::new(&settings.orchestrator_url, None).await {
        Ok(mut orc_client) => match orc_client
            .node_registration_api()
            .update_node(edgeless_api::node_registration::UpdateNodeRequest::Registration(
                settings.node_id.clone(),
                settings.agent_url.clone(),
                settings.invocation_url.clone(),
                resource_provider_specifications,
            ))
            .await
        {
            Ok(res) => match res {
                edgeless_api::node_registration::UpdateNodeResponse::ResponseError(err) => {
                    panic!("could not register to e-ORC {}: {}", &settings.orchestrator_url, err)
                }
                edgeless_api::node_registration::UpdateNodeResponse::Accepted => {
                    log::info!("this node '{}' registered to e-ORC '{}'", &settings.node_id, &settings.orchestrator_url)
                }
            },
            Err(err) => panic!("channel error when registering to e-ORC {}: {}", &settings.orchestrator_url, err),
        },
        Err(err) => panic!("could not connect to e-ORC {}: {}", &settings.orchestrator_url, err),
    }
}

async fn fill_resources(
    data_plane: edgeless_dataplane::handle::DataplaneProvider,
    settings: &EdgelessNodeSettings,
    provider_specifications: &mut Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
) -> std::collections::HashMap<
    String,
    Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
> {
    let mut ret = std::collections::HashMap::<
        String,
        Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
    >::new();

    if !settings.http_ingress_url.is_empty() {
        log::info!("Creating resource 'http-ingress-1' at {}", &settings.http_ingress_url);
        ret.insert(
            "http-ingress-1".to_string(),
            resources::http_ingress::ingress_task(
                data_plane.clone(),
                edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
                settings.http_ingress_url.clone(),
            )
            .await,
        );
        provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
            provider_id: "http-ingress-1".to_string(),
            class_type: "http-ingress".to_string(),
            outputs: vec!["new_request".to_string()],
        });
    }

    log::info!("Creating resource 'http-egress-1'");
    ret.insert(
        "http-egress-1".to_string(),
        Box::new(
            resources::http_egress::EgressResourceProvider::new(
                data_plane.clone(),
                edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
            )
            .await,
        ),
    );
    provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
        provider_id: "http-egress-1".to_string(),
        class_type: "http-egress".to_string(),
        outputs: vec![],
    });

    log::info!("Creating resource 'file-log-1'");
    ret.insert(
        "file-log-1".to_string(),
        Box::new(
            resources::file_log::FileLogResourceProvider::new(
                data_plane.clone(),
                edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
            )
            .await,
        ),
    );
    provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
        provider_id: "file-log-1".to_string(),
        class_type: "file-log".to_string(),
        outputs: vec![],
    });

    log::info!("Creating resource 'redis-1'");
    ret.insert(
        "redis-1".to_string(),
        Box::new(
            resources::redis::RedisResourceProvider::new(
                data_plane.clone(),
                edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
            )
            .await,
        ),
    );
    provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
        provider_id: "redis-1".to_string(),
        class_type: "redis".to_string(),
        outputs: vec![],
    });

    ret
}

pub async fn edgeless_node_main(settings: EdgelessNodeSettings) {
    log::info!("Starting Edgeless Node");
    log::debug!("Settings: {:?}", settings);

    // Create the state manager.
    let state_manager = Box::new(state_management::StateManager::new().await);

    // Create the data plane.
    let data_plane = edgeless_dataplane::handle::DataplaneProvider::new(settings.node_id.clone(), settings.invocation_url.clone()).await;

    // Create the telemetry provider.
    let telemetry_provider = edgeless_telemetry::telemetry_events::TelemetryProcessor::new(settings.metrics_url.clone())
        .await
        .expect(&format!("could not build the telemetry provider at URL {}", &settings.metrics_url));

    // Create the WebAssembly runner.
    let (rust_runtime_client, mut rust_runtime_task_s) = base_runtime::runtime::create::<wasm_runner::function_instance::WASMFunctionInstance>(
        data_plane.clone(),
        state_manager.clone(),
        Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
            ("FUNCTION_TYPE".to_string(), "RUST_WASM".to_string()),
            ("NODE_ID".to_string(), settings.node_id.to_string()),
        ]))),
    );
    let rust_runtime_task = tokio::spawn(async move {
        rust_runtime_task_s.run().await;
    });

    // Create the resources.
    let mut resource_provider_specifications = vec![];
    let resources = fill_resources(data_plane.clone(), &settings, &mut resource_provider_specifications).await;

    // Create the agent.
    let (mut agent, agent_task) = agent::Agent::new(Box::new(rust_runtime_client.clone()), resources, settings.clone(), data_plane.clone());
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.agent_url.clone());

    // Wait for all the tasks to complete.
    let _ = futures::join!(
        rust_runtime_task,
        agent_task,
        agent_api_server,
        register_node(&settings, resource_provider_specifications)
    );
}

pub fn edgeless_node_default_conf() -> String {
    String::from(
        r##"node_id = "fda6ce79-46df-4f96-a0d2-456f720f606c"
agent_url = "http://127.0.0.1:7021"
invocation_url = "http://127.0.0.1:7002"
metrics_url = "http://127.0.0.1:7003"
orchestrator_url = "http://127.0.0.1:7011"
resource_configuration_url = "http://127.0.0.1:7033"
http_ingress_url = "http://127.0.0.1:7035"
"##,
    )
}
