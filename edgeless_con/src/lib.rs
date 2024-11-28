// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
mod controller;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessConOrcConfig {
    pub domain_id: String,
    pub orchestrator_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessConSettings {
    pub controller_url: String,
    pub domain_register_url: String,
    pub orchestrators: Vec<EdgelessConOrcConfig>,
}

pub async fn edgeless_con_main(settings: EdgelessConSettings) {
    log::info!("Starting Edgeless Controller at {}", settings.controller_url);
    log::debug!("Settings: {:?}", settings);

    let (mut controller, controller_task) = controller::Controller::new_from_config(settings.clone()).await;

    let workflow_instance_server_task = edgeless_api::grpc_impl::outer::controller::WorkflowInstanceAPIServer::run(
        controller.get_workflow_instance_client(),
        settings.controller_url,
    );

    let domain_register_server_task = edgeless_api::grpc_impl::outer::domain_register::DomainRegistrationAPIServer::run(
        controller.get_domain_register_client(),
        settings.domain_register_url,
    );

    futures::join!(controller_task, workflow_instance_server_task, domain_register_server_task);
}

pub fn edgeless_con_default_conf() -> String {
    String::from(
        r##"controller_url = "http://127.0.0.1:7001"
domain_register_url = "http://127.0.0.1:7004"
orchestrators = [
    { domain_id = "domain-1", orchestrator_url="http://127.0.0.1:7011" }
]
"##,
    )
}
