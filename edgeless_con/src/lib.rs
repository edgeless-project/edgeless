// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
mod controller;
mod orchestration_logic;
mod orchestration_utils;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessConOrcConfig {
    pub domain_id: String,
    pub orchestrator_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessConSettings {
    pub controller_url: String,
    pub orchestrators: Vec<EdgelessConOrcConfig>,
}

pub async fn edgeless_con_main(settings: EdgelessConSettings) {
    log::info!("Starting Edgeless Controller at {}", settings.controller_url);
    log::debug!("Settings: {:?}", settings);

    let (mut controller, controller_task) = controller::Controller::new_from_config(settings.clone()).await;

    let server_task =
        edgeless_api::grpc_impl::controller::WorkflowInstanceAPIServer::run(controller.get_api_client(), settings.controller_url.clone());

    futures::join!(controller_task, server_task);
}

pub fn edgeless_con_default_conf() -> String {
    String::from(
        r##"controller_url = "http://127.0.0.1:7001"
orchestrators = [
    { domain_id = "domain-1", orchestrator_url="http://127.0.0.1:7011" }
]
"##,
    )
}
