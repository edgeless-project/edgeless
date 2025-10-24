// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

mod controller;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessConSettings {
    pub controller_url: String,
    pub domain_register_url: String,
    pub persistence_filename: String,
}

pub async fn edgeless_con_main(settings: EdgelessConSettings) {
    log::info!(
        "Starting Edgeless Controller at {}, persistence at {}",
        settings.controller_url,
        settings.persistence_filename
    );
    log::debug!("Settings: {:?}", settings);

    let (mut controller, controller_task, refresh_task) = controller::Controller::new(settings.persistence_filename);

    let workflow_instance_server_task = edgeless_api::grpc_impl::outer::controller::WorkflowInstanceAPIServer::run(
        controller.get_workflow_instance_client(),
        settings.controller_url,
        Some(edgeless_api::grpc_impl::tls_config::TlsConfig::global_server().clone()),
    );
    let domain_register_server_task = edgeless_api::grpc_impl::outer::domain_register::DomainRegistrationAPIServer::run(
        controller.get_domain_register_client(),
        settings.domain_register_url,
        Some(edgeless_api::grpc_impl::tls_config::TlsConfig::global_server().clone()),
    );

    futures::join!(controller_task, refresh_task, workflow_instance_server_task, domain_register_server_task);
}

pub fn edgeless_con_default_conf() -> String {
    let con_conf = EdgelessConSettings {
        controller_url: String::from("http://127.0.0.1:7001"),
        domain_register_url: String::from("http://127.0.0.1:7002"),
        persistence_filename: String::from("controller.save"),
    };

    toml::to_string(&con_conf).expect("Wrong")
}
