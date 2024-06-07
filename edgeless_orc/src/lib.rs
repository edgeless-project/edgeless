// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

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
    pub domain_id: String,
    pub orchestrator_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcBaselineSettings {
    pub orchestration_strategy: OrchestrationStrategy,
    pub keep_alive_interval_secs: u64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcProxySettings {
    pub proxy_type: String,
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

    let (mut orchestrator, orchestrator_task) = orchestrator::Orchestrator::new(settings.baseline.clone(), make_proxy(settings.proxy)).await;

    let orchestrator_server =
        edgeless_api::grpc_impl::orc::OrchestratorAPIServer::run(orchestrator.get_api_client(), settings.general.orchestrator_url);

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

    join!(orchestrator_task, orchestrator_server);
}

pub fn edgeless_orc_default_conf() -> String {
    String::from(
        r##"[general]
domain_id = "domain-1"
orchestrator_url = "http://127.0.0.1:7011"

[baseline]
orchestration_strategy = "Random"
keep_alive_interval_secs = 2

[proxy]
proxy_type = "None"
redis_url = ""
"##,
    )
}
