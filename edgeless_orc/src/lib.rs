// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

mod orchestration_logic;
pub mod orchestrator;
pub mod proxy;
pub mod proxy_none;
pub mod proxy_redis;

use futures::join;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcSettings {
    pub general: EdgelessOrcGeneralSettings,
    pub baseline: EdgelessOrcBaselineSettings,
    pub proxy: EdgelessOrcProxySettings,
    pub collector: EdgelessOrcCollectorSettings,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcGeneralSettings {
    /// The identifier of the orchestration domain managed by this orchestrator.
    pub domain_id: String,
    /// The URL to which the orchestrator is bound.
    pub orchestrator_url: String,
    /// The URL to which the orchestrator can be reached, which may be
    /// different from `orchestrator_url`, e.g., for NAT traversal.
    pub orchestrator_url_announced: String,
    /// The COAP URL to which the orchestrator is bound.
    pub orchestrator_coap_url: Option<String>,
    /// The COAP URL to which the orchestrator can be reached, which may be
    /// different from `orchestrator_url`, e.g., for NAT traversal.
    pub orchestrator_coap_url_announced: Option<String>,
    /// The URL of the agent of the node embedded in the orchestrator.
    pub agent_url: String,
    /// The agent URL announced by the node.
    /// It is the end-point used by the orchestrator to manage the node.
    /// It can be different from `agent_url`, e.g., for NAT traversal.
    pub agent_url_announced: String,
    /// The URL of the data plane of the node embedded in the orchestrator.
    pub invocation_url: String,
    /// The invocation URL announced by the node.
    /// It can be different from `invocation_url`, e.g., for NAT traversal.
    pub invocation_url_announced: String,
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
    /// Settings on whether/how to save events to output files.
    pub dataset_settings: Option<EdgelessOrcProxyDatasetSettings>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcProxyDatasetSettings {
    /// Path where to save the output CSV datasets. If empty, do not save them.
    dataset_path: String,
    /// Append to the output dataset files.
    append: bool,
    /// Additional fields recorded in the CSV output file.
    additional_fields: String,
    /// Header of additional fields recorded in the CSV output file.
    additional_header: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcCollectorSettings {
    /// Type of the metrics collector that is used to store run-time
    /// measurements from function instances.
    pub collector_type: String,
    /// If collector_type is "Redis" then this is the URL of the Redis server.
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
        "redis" => match proxy_redis::ProxyRedis::new(&settings.redis_url.unwrap_or_default(), true, settings.dataset_settings) {
            Ok(proxy_redis) => return Box::new(proxy_redis),
            Err(err) => log::error!("error when connecting to Redis: {}", err),
        },
        _ => log::error!("unknown proxy type: {}", settings.proxy_type),
    }
    Box::new(proxy_none::ProxyNone {})
}

pub async fn edgeless_orc_main(settings: EdgelessOrcSettings) {
    log::info!("Starting Edgeless Orchestrator at {}", settings.general.orchestrator_url,);
    log::debug!("Settings: {:?}", settings);

    // Create a node embedded in the orchestrator for metrics collection.
    let (node_id, agent_task, agent_api_server, resource_provider_specifications) = edgeless_node::create_metrics_collector_node(
        settings.general.invocation_url.clone(),
        settings.general.agent_url.clone(),
        settings.collector.collector_type,
        settings.collector.redis_url,
    )
    .await;

    // Create the orchestrator.
    let (mut orchestrator, orchestrator_task) = orchestrator::Orchestrator::new(settings.baseline.clone(), make_proxy(settings.proxy)).await;

    let orchestrator_server =
        edgeless_api::grpc_impl::outer::orc::OrchestratorAPIServer::run(orchestrator.get_api_client(), settings.general.orchestrator_url.clone());

    let orchestrator_coap_server = if let Some(_url) = settings.general.orchestrator_coap_url {
        edgeless_api::coap_impl::orchestration::CoapOrchestrationServer::run(
            orchestrator.get_api_client().node_registration_api(),
            std::net::SocketAddrV4::new("0.0.0.0".parse().unwrap(), 7050),
        )
    } else {
        Box::pin(async {})
    };

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
        orchestrator_coap_server,
        edgeless_node::register_node(
            edgeless_node::EdgelessNodeGeneralSettings {
                node_id,
                agent_url: settings.general.agent_url,
                agent_url_announced: settings.general.agent_url_announced,
                invocation_url: settings.general.invocation_url,
                invocation_url_announced: settings.general.invocation_url_announced,
                invocation_url_coap: None,
                invocation_url_announced_coap: None,
                orchestrator_url: match settings.general.orchestrator_url_announced.is_empty() {
                    true => settings.general.orchestrator_url,
                    false => settings.general.orchestrator_url_announced.clone(),
                },
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
orchestrator_url_announced = ""
orchestrator_coap_url = "coap://127.0.0.1:7050"
orchestrator_coap_url_announced = ""
agent_url = "http://127.0.0.1:7121"
agent_url_announced = ""
invocation_url = "http://127.0.0.1:7102"
invocation_url_announced = ""

[baseline]
orchestration_strategy = "Random"
keep_alive_interval_secs = 2

[proxy]
proxy_type = "None"
redis_url = ""

[proxy.dataset_settings]
dataset_path = ""
append = true
additional_fields = ""
additional_header = ""

[collector]
collector_type = "None"
redis_url = ""
"##,
    )
}
