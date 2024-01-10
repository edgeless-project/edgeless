// SPDX-FileCopyrightText: © 2023 TUM
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
mod orchestration_logic;
mod orchestrator;

use futures::join;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessOrcSettings {
    pub domain_id: String,
    pub orchestrator_url: String,
    pub orchestration_strategy: OrchestrationStrategy,
    pub keep_alive_interval_secs: u64,
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

pub async fn edgeless_orc_main(settings: EdgelessOrcSettings) {
    log::info!("Starting Edgeless Orchestrator at {}", settings.orchestrator_url);
    log::debug!("Settings: {:?}", settings);

    let (mut orchestrator, orchestrator_task) = orchestrator::Orchestrator::new(settings.clone()).await;

    let orchestrator_server = edgeless_api::grpc_impl::orc::OrchestratorAPIServer::run(orchestrator.get_api_client(), settings.orchestrator_url);

    if settings.keep_alive_interval_secs == 0 {
        log::info!("node keep-alive disabled");
    } else {
        log::info!("node keep-alive enabled every {} seconds", settings.keep_alive_interval_secs);
        let _keep_alive_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(settings.keep_alive_interval_secs));
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
        r##"domain_id = "domain-1"
orchestrator_url = "http://127.0.0.1:7011"
orchestration_strategy = "Random"
keep_alive_interval_secs = 2
"##,
    )
}
