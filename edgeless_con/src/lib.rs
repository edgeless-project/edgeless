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
    log::info!("Starting Edgeless Controller");
    log::debug!("Settings: {:?}", settings);

    let (mut controller, controller_task) = controller::Controller::new(settings.clone());

    let server_task = edgeless_api::grpc_impl::con::WorkflowInstanceAPIServer::run(controller.get_api_client(), settings.controller_url.clone());

    futures::join!(controller_task, server_task);
}
