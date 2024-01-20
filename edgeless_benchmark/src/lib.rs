// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub mod resources;

pub struct DummyFunctionInstance {}

#[async_trait::async_trait]
impl edgeless_node::base_runtime::FunctionInstance for DummyFunctionInstance {
    async fn instantiate(
        _guest_api_host: edgeless_node::base_runtime::guest_api::GuestAPIHost,
        _code: &[u8],
    ) -> Result<Box<Self>, edgeless_node::base_runtime::FunctionInstanceError> {
        Ok(Box::new(Self {}))
    }

    async fn init(
        &mut self,
        _init_payload: Option<&str>,
        _serialized_state: Option<&str>,
    ) -> Result<(), edgeless_node::base_runtime::FunctionInstanceError> {
        Ok(())
    }

    async fn cast(
        &mut self,
        _src: &edgeless_api::function_instance::InstanceId,
        _msg: &str,
    ) -> Result<(), edgeless_node::base_runtime::FunctionInstanceError> {
        Ok(())
    }

    async fn call(
        &mut self,
        _src: &edgeless_api::function_instance::InstanceId,
        _msg: &str,
    ) -> Result<edgeless_dataplane::core::CallRet, edgeless_node::base_runtime::FunctionInstanceError> {
        Ok(edgeless_dataplane::core::CallRet::NoReply)
    }

    async fn stop(&mut self) -> Result<(), edgeless_node::base_runtime::FunctionInstanceError> {
        Ok(())
    }
}

pub async fn edgeless_metrics_collector_node_main(settings: edgeless_node::EdgelessNodeSettings) {
    log::info!("Starting EDGELESS metrics collector node");
    log::debug!("Settings: {:?}", settings);

    // Create the state manager.
    let state_manager = Box::new(edgeless_node::state_management::StateManager::new().await);

    // Create the data plane.
    let data_plane = edgeless_dataplane::handle::DataplaneProvider::new(settings.node_id.clone(), settings.invocation_url.clone()).await;

    // Create the telemetry provider.
    let telemetry_provider = edgeless_telemetry::telemetry_events::TelemetryProcessor::new(settings.metrics_url.clone())
        .await
        .expect(&format!("could not build the telemetry provider at URL {}", &settings.metrics_url));

    // Create the WebAssembly runner.
    let (rust_runtime_client, mut rust_runtime_task_s) = edgeless_node::base_runtime::runtime::create::<DummyFunctionInstance>(
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

    // Create the metrics collector resource.
    let resource_provider_specifications = vec![edgeless_api::node_registration::ResourceProviderSpecification {
        provider_id: "metrics-collector".to_string(),
        class_type: "metrics-collector".to_string(),
        outputs: vec![],
    }];
    let mut resources: std::collections::HashMap<
        String,
        Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
    > = std::collections::HashMap::new();
    resources.insert(
        "metrics-collector".to_string(),
        Box::new(
            crate::resources::metrics_collector::MetricsCollectorResourceProvider::new(
                data_plane.clone(),
                edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
            )
            .await,
        ),
    );

    // Create the agent.
    let (mut agent, agent_task) =
        edgeless_node::agent::Agent::new(Box::new(rust_runtime_client.clone()), resources, settings.clone(), data_plane.clone());
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.agent_url.clone());

    // Wait for all the tasks to complete.
    let _ = futures::join!(
        rust_runtime_task,
        agent_task,
        agent_api_server,
        edgeless_node::register_node(
            &settings,
            edgeless_api::node_registration::NodeCapabilities::default(),
            resource_provider_specifications
        )
    );
}
