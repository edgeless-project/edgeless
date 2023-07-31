mod controller;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EdgelessConOrcConfig {
    pub domain_id: String,
    pub orchestrator_url: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EdgelessConResourceConfig {
    pub resource_provider_id: String,
    pub resource_class_type: String,
    pub output_callback_declarations: Vec<String>,
    pub resource_configuration_url: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EdgelessConSettings {
    pub controller_url: String,
    pub orchestrators: Vec<EdgelessConOrcConfig>,
    pub resources: Vec<EdgelessConResourceConfig>,
}

pub async fn edgeless_con_main(settings: EdgelessConSettings) {
    log::info!("Starting Edgeless Controller at {}", settings.controller_url);
    log::debug!("Settings: {:?}", settings);

    let (mut controller, controller_task) = controller::Controller::new(settings.clone());

    let server_task =
        edgeless_api::grpc_impl::controller::WorkflowInstanceAPIServer::run(controller.get_api_client(), settings.controller_url.clone());

    futures::join!(controller_task, server_task);
}

pub fn edgeless_con_default_conf() -> String {
    String::from(
        r##"controller_url = "http://127.0.0.1:7021"
orchestrators = [
    { domain_id = "domain-1", orchestrator_url="http://127.0.0.1:7011" }
]
resources = [
    { resource_provider_id = "http-ingress-1",  resource_class_type = "http-ingress", output_callback_declarations = ["new_request"], resource_configuration_url = "http://127.0.0.1:7033" },
    { resource_provider_id = "http-egress-1",  resource_class_type = "http-egress", output_callback_declarations = [], resource_configuration_url = "http://127.0.0.1:7033" }
]
"##,
    )
}
