// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod active_instance;
pub mod affinity_level;
pub mod client_desc;
pub mod deploy_intent;
pub mod deployment_requirements;
pub mod domain_info;
pub mod domain_subscriber;
pub mod node_register;
pub mod node_register_client;
pub mod orchestration_logic;
pub mod orchestrator;
pub mod orchestrator_task;
pub mod proxy;
pub mod proxy_none;
pub mod proxy_redis;
pub mod resource_provider;

#[cfg(test)]
mod tests;

use futures::join;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcSettings {
    pub general: EdgelessOrcGeneralSettings,
    pub baseline: EdgelessOrcBaselineSettings,
    pub proxy: EdgelessOrcProxySettings,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcGeneralSettings {
    /// The URL of the domain register.
    pub domain_register_url: String,
    /// The interval at which the orchestrator refreshes subscription, s.
    pub subscription_refresh_interval_sec: u64,
    /// The identifier of the orchestration domain managed by this orchestrator.
    pub domain_id: String,
    /// The URL to which the orchestrator is bound.
    pub orchestrator_url: String,
    /// The URL to which the orchestrator can be reached, which may be
    /// different from `orchestrator_url`, e.g., for NAT traversal.
    pub orchestrator_url_announced: String,
    /// The gRPC URL of the node register.
    pub node_register_url: String,
    /// The CoAP URL of the node register.
    pub node_register_coap_url: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcBaselineSettings {
    /// The orchestration strategy.
    pub orchestration_strategy: OrchestrationStrategy,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcProxySettings {
    /// Type of the proxy that is used to mirror the internal data structures
    /// of the orchestrator and to receive orchestration directives.
    pub proxy_type: String,
    /// Gargabe collection interval, in seconds. 0 means disabled.
    pub proxy_gc_period_seconds: u64,
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

impl Default for EdgelessOrcProxyDatasetSettings {
    fn default() -> Self {
        Self {
            dataset_path: String::default(),
            append: true,
            additional_fields: String::default(),
            additional_header: String::default(),
        }
    }
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

pub fn make_proxy(settings: EdgelessOrcProxySettings) -> std::sync::Arc<tokio::sync::Mutex<dyn proxy::Proxy>> {
    match settings.proxy_type.to_lowercase().as_str() {
        "none" => {}
        "redis" => match proxy_redis::ProxyRedis::new(&settings.redis_url.unwrap_or_default(), true, settings.dataset_settings) {
            Ok(proxy_redis) => return std::sync::Arc::new(tokio::sync::Mutex::new(proxy_redis)),
            Err(err) => log::error!("error when connecting to Redis: {}", err),
        },
        _ => log::error!("unknown proxy type: {}", settings.proxy_type),
    }
    std::sync::Arc::new(tokio::sync::Mutex::new(proxy_none::ProxyNone {}))
}

pub async fn edgeless_orc_main(settings: EdgelessOrcSettings) {
    log::info!("Starting Edgeless Orchestrator");
    log::debug!("Settings: {:?}", settings);

    // Create the component that subscribes to the domain register to
    // notify domain updates (periodically refreshed).
    let (mut subscriber, subscriber_task, subscriber_refresh_task) = domain_subscriber::DomainSubscriber::new(
        settings.general.domain_id.clone(),
        match edgeless_api::util::get_announced(&settings.general.orchestrator_url, &settings.general.orchestrator_url_announced) {
            Ok(url) => url,
            Err(err) => {
                log::error!(
                    "invalid URL '{}' (announced: '{}'): {}",
                    settings.general.orchestrator_url,
                    settings.general.orchestrator_url_announced,
                    err
                );
                String::default()
            }
        },
        settings.general.domain_register_url,
        settings.general.subscription_refresh_interval_sec,
    )
    .await;

    // Create the proxy.
    let proxy_gc_period = tokio::time::Duration::from_secs(settings.proxy.proxy_gc_period_seconds);
    let proxy = make_proxy(settings.proxy);
    proxy.lock().await.update_domain_info(&crate::domain_info::DomainInfo {
        domain_id: settings.general.domain_id.to_string(),
    });

    // Create the orchestrator.
    let (mut orchestrator, orchestrator_task, orchestrator_refresh_task) =
        orchestrator::Orchestrator::new(settings.baseline.clone(), proxy.clone(), subscriber.get_subscriber_sender()).await;

    let orchestrator_server = edgeless_api::grpc_impl::outer::orc::OrchestratorAPIServer::run(
        orchestrator.get_api_client(),
        settings.general.orchestrator_url,
        Some(edgeless_api::grpc_impl::tls_config::TlsConfig::global_server().clone()),
    );

    // Create the node register.
    let (mut node_register, node_register_task, node_register_refresh_task) =
        node_register::NodeRegister::new(proxy, proxy_gc_period, orchestrator.get_sender()).await;

    let node_register_server = edgeless_api::grpc_impl::outer::node_register::NodeRegisterAPIServer::run(
        node_register.get_node_registration_client(),
        settings.general.node_register_url,
        Some(edgeless_api::grpc_impl::tls_config::TlsConfig::global_server().clone()),
    );

    let node_register_coap_server = if let Some(url) = settings.general.node_register_coap_url {
        if let Ok((proto, address, port)) = edgeless_api::util::parse_http_host(&url) {
            if proto != edgeless_api::util::Proto::COAP {
                log::warn!("Wrong protocol for the CoAP node register ({}): assuming coap://", url);
            }
            if address != "0.0.0.0" {
                log::warn!("CoAP node register requested to be bound at {}: ignored, using 0.0.0.0 instead", address);
            }
            edgeless_api::coap_impl::node_register::CoapNodeRegisterServer::run(
                node_register.get_node_registration_client().node_registration_api(),
                std::net::SocketAddrV4::new("0.0.0.0".parse().unwrap(), port),
            )
        } else {
            log::error!("Wrong URL for the CoAP node register: {}", url);
            Box::pin(async {})
        }
    } else {
        Box::pin(async {})
    };

    // Wait for all the tasks to come to an end.
    join!(
        orchestrator_task,
        orchestrator_refresh_task,
        orchestrator_server,
        node_register_task,
        node_register_refresh_task,
        node_register_server,
        node_register_coap_server,
        subscriber_task,
        subscriber_refresh_task
    );
}

pub fn edgeless_orc_default_conf() -> String {
    let orc_conf = EdgelessOrcSettings {
        general: EdgelessOrcGeneralSettings {
            domain_register_url: String::from("http://127.0.0.1:7002"),
            subscription_refresh_interval_sec: 2,
            domain_id: String::from("domain-7000"),
            orchestrator_url: String::from("http://127.0.0.1:7003"),
            orchestrator_url_announced: String::from("http://127.0.0.1:7003"),
            node_register_url: String::from("http://127.0.0.1:7004"),
            node_register_coap_url: None,
        },
        baseline: EdgelessOrcBaselineSettings {
            orchestration_strategy: OrchestrationStrategy::Random,
        },
        proxy: EdgelessOrcProxySettings {
            proxy_type: "None".to_string(),
            proxy_gc_period_seconds: 360,
            redis_url: Some(String::from("redis://127.0.0.1:6379")),
            dataset_settings: Some(EdgelessOrcProxyDatasetSettings::default()),
        },
    };

    toml::to_string(&orc_conf).expect("Wrong")
}
