// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessBalSettings {
    /// General settins.
    pub general: EdgelessBalGeneralSettings,
    /// Node settings for local domain.
    pub local: edgeless_node::EdgelessNodeGeneralSettings,
    /// Node settings for portal domain.
    pub portal: edgeless_node::EdgelessNodeGeneralSettings,
    /// Node telemetry settings, for both local and portal domains.
    pub telemetry: edgeless_node::EdgelessNodeTelemetrySettings,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessBalGeneralSettings {
    /// Local domain identifier.
    pub domain: String,
}

impl Default for EdgelessBalGeneralSettings {
    fn default() -> Self {
        Self {
            domain: String::from("domain-7000"),
        }
    }
}

pub async fn edgeless_bal_main(settings: EdgelessBalSettings) {
    log::info!("Starting EDGELESS Balancer");
    log::debug!("Settings: {:?}", settings);

    // Create the local and portal data planes.
    let local_data_plane = edgeless_dataplane::handle::DataplaneProvider::new(
        settings.local.node_id,
        settings.local.invocation_url.clone(),
        settings.local.invocation_url_coap.clone(),
    )
    .await;
    let portal_data_plane = edgeless_dataplane::handle::DataplaneProvider::new(
        settings.portal.node_id,
        settings.portal.invocation_url.clone(),
        settings.portal.invocation_url_coap.clone(),
    )
    .await;

    // Create the performance target.
    let telemetry_performance_target = edgeless_telemetry::performance_target::PerformanceTargetInner::new();

    // Create the telemetry provider.
    let telemetry_provider = match edgeless_telemetry::telemetry_events::TelemetryProcessor::new(
        settings.telemetry.metrics_url.clone(),
        if settings.telemetry.performance_samples {
            Some(telemetry_performance_target.clone())
        } else {
            None
        },
    )
    .await
    {
        Ok(telemetry_provider) => telemetry_provider,
        Err(err) => panic!("could not build the telemetry provider: {}", err),
    };

    // Create the resources.
    let local_resource_provider_specifications = vec![];
    let local_resources = std::collections::HashMap::new();
    let portal_resource_provider_specifications = vec![];
    let portal_resources = std::collections::HashMap::new();

    // Create the local and portal agent.
    let (mut local_agent, local_agent_task) = edgeless_node::agent::Agent::new(
        std::collections::HashMap::new(),
        local_resources,
        settings.local.node_id,
        local_data_plane.clone(),
    );
    let local_agent_api_server =
        edgeless_api::grpc_impl::outer::agent::AgentAPIServer::run(local_agent.get_api_client(), settings.local.agent_url.clone());

    let (mut portal_agent, portal_agent_task) = edgeless_node::agent::Agent::new(
        std::collections::HashMap::new(),
        portal_resources,
        settings.portal.node_id,
        portal_data_plane.clone(),
    );
    let portal_agent_api_server =
        edgeless_api::grpc_impl::outer::agent::AgentAPIServer::run(portal_agent.get_api_client(), settings.portal.agent_url.clone());

    // Create the component that subscribes to the node register to
    // notify updates (periodically refreshed), for the local and portal parts.
    let (_local_subscriber, local_subscriber_task, local_refresh_task) = edgeless_node::node_subscriber::NodeSubscriber::new(
        settings.local,
        local_resource_provider_specifications.clone(),
        edgeless_api::node_registration::NodeCapabilities::default(),
        None,
        telemetry_performance_target.clone(),
    )
    .await;
    let mut capabilities = edgeless_api::node_registration::NodeCapabilities::default();
    capabilities.labels.push(format!("portal-domain={}", settings.general.domain));
    let (_portal_subscriber, portal_subscriber_task, portal_refresh_task) = edgeless_node::node_subscriber::NodeSubscriber::new(
        settings.portal,
        portal_resource_provider_specifications.clone(),
        capabilities,
        None,
        telemetry_performance_target,
    )
    .await;

    // Wait for all the tasks to complete.
    let _ = futures::join!(
        // tasks for the local node
        local_agent_task,
        local_agent_api_server,
        local_subscriber_task,
        local_refresh_task,
        // tasks for the portal node
        portal_agent_task,
        portal_agent_api_server,
        portal_subscriber_task,
        portal_refresh_task,
    );
}

pub fn edgeless_bal_default_conf() -> String {
    let bal_conf = EdgelessBalSettings {
        general: EdgelessBalGeneralSettings::default(),
        local: edgeless_node::EdgelessNodeGeneralSettings::default(),
        portal: edgeless_node::EdgelessNodeGeneralSettings {
            node_id: uuid::Uuid::new_v4(),
            agent_url: String::from("http://127.0.0.1:7105"),
            agent_url_announced: String::from("http://127.0.0.1:7105"),
            invocation_url: String::from("http://127.0.0.1:7106"),
            invocation_url_announced: String::from("http://127.0.0.1:7106"),
            invocation_url_coap: None,
            invocation_url_announced_coap: None,
            node_register_url: String::from("http://127.0.0.1:7104"),
            subscription_refresh_interval_sec: 2,
        },
        telemetry: edgeless_node::EdgelessNodeTelemetrySettings::default(),
    };

    toml::to_string(&bal_conf).expect("Wrong")
}
