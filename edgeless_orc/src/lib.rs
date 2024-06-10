// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

mod metrics_collector;
mod orchestration_logic;
mod orchestrator;
mod proxy;
mod proxy_none;
mod proxy_redis;

use futures::join;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcSettings {
    pub general: EdgelessOrcGeneralSettings,
    pub baseline: EdgelessOrcBaselineSettings,
    pub proxy: EdgelessOrcProxySettings,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcGeneralSettings {
    /// The identifier of the orchestration domain managed by this orchestrator.
    pub domain_id: String,
    // The URL to which the orchestrator can be reached.
    pub orchestrator_url: String,
    /// The URL of the agent of the node embedded in the orchestrator.
    pub agent_url: String,
    /// The URL of the data plane of the node embedded in the orchestrator.
    pub invocation_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcBaselineSettings {
    /// The orchestration strategy.
    pub orchestration_strategy: OrchestrationStrategy,
    /// The periodic interval at which nodes are polled for keep-alive and
    /// data structures are updated on the proxy.
    pub keep_alive_interval_secs: u64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcProxySettings {
    /// Type of the proxy that is used to mirror the internal data structures
    /// of the orchestrator and to receive orchestration directives.
    pub proxy_type: String,
    /// If proxy_type is "Redis" then this is the URL of the Redis server.
    pub redis_url: Option<String>,
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

pub fn make_proxy(settings: EdgelessOrcProxySettings) -> Box<dyn proxy::Proxy> {
    match settings.proxy_type.to_lowercase().as_str() {
        "none" => {}
        "redis" => match proxy_redis::ProxyRedis::new(&settings.redis_url.unwrap_or_default()) {
            Ok(proxy_redis) => return Box::new(proxy_redis),
            Err(err) => log::error!("error when connecting to Redis: {}", err),
        },
        _ => log::error!("unknown proxy type: {}", settings.proxy_type),
    }
    Box::new(proxy_none::ProxyNone {})
}

pub async fn edgeless_orc_main(settings: EdgelessOrcSettings) {
    log::info!("Starting Edgeless Orchestrator at {}", settings.general.orchestrator_url);
    log::debug!("Settings: {:?}", settings);

    // Create the data plane for the node embedded in the orchestrator.
    let node_id = uuid::Uuid::new_v4();
    let data_plane = edgeless_dataplane::handle::DataplaneProvider::new(node_id.clone(), settings.general.invocation_url.clone()).await;

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
            metrics_collector::MetricsCollectorResourceProvider::new(data_plane.clone(), edgeless_api::function_instance::InstanceId::new(node_id))
                .await,
        ),
    );

    // Create the agent of the node embedded in the orchestrator.
    let (mut agent, agent_task) = edgeless_node::agent::Agent::new(std::collections::HashMap::new(), resources, node_id, data_plane.clone());
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.general.agent_url.clone());

    // Create the orchestrator.
    let (mut orchestrator, orchestrator_task) = orchestrator::Orchestrator::new(settings.baseline.clone(), make_proxy(settings.proxy)).await;

    let orchestrator_server =
        edgeless_api::grpc_impl::orc::OrchestratorAPIServer::run(orchestrator.get_api_client(), settings.general.orchestrator_url.clone());

    if settings.baseline.keep_alive_interval_secs == 0 {
        log::info!("node keep-alive disabled");
    } else {
        log::info!("node keep-alive enabled every {} seconds", settings.baseline.keep_alive_interval_secs);
        let _keep_alive_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(settings.baseline.keep_alive_interval_secs));
            loop {
                interval.tick().await;
                orchestrator.keep_alive().await;
            }
        });
    }

    join!(
        agent_task,
        agent_api_server,
        orchestrator_task,
        orchestrator_server,
        edgeless_node::register_node(
            edgeless_node::EdgelessNodeGeneralSettings {
                node_id,
                agent_url: settings.general.agent_url,
                agent_url_announced: "".to_string(),
                invocation_url: settings.general.invocation_url,
                invocation_url_announced: "".to_string(),
                metrics_url: "".to_string(),
                orchestrator_url: settings.general.orchestrator_url
            },
            edgeless_api::node_registration::NodeCapabilities::empty(),
            resource_provider_specifications
        )
    );
}

pub fn edgeless_orc_default_conf() -> String {
    String::from(
        r##"[general]
domain_id = "domain-1"
orchestrator_url = "http://127.0.0.1:7011"
agent_url = "http://127.0.0.1:7121"
invocation_url = "http://127.0.0.1:7102"

[baseline]
orchestration_strategy = "Random"
keep_alive_interval_secs = 2

[proxy]
proxy_type = "None"
redis_url = ""
"##,
    )
}
