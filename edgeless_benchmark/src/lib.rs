// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub mod redis_dumper;
pub mod resources;

pub async fn edgeless_metrics_collector_node_main(settings: edgeless_node::EdgelessNodeSettings) {
    log::info!("Starting EDGELESS metrics collector node");
    log::debug!("Settings: {:?}", settings);

    // Create the data plane.
    let data_plane = edgeless_dataplane::handle::DataplaneProvider::new(settings.node_id.clone(), settings.invocation_url.clone()).await;

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
                edgeless_api::function_instance::InstanceId::new(settings.node_id),
            )
            .await,
        ),
    );

    // Create the agent.
    let (mut agent, agent_task) =
        edgeless_node::agent::Agent::new(std::collections::HashMap::new(), resources, settings.node_id.clone(), data_plane.clone());
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.agent_url.clone());

    // Wait for all the tasks to complete.
    let _ = futures::join!(
        agent_task,
        agent_api_server,
        edgeless_node::register_node(
            settings,
            edgeless_api::node_registration::NodeCapabilities::empty(),
            resource_provider_specifications
        )
    );
}
