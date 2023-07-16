mod controller;

#[derive(Clone)]
pub struct EdgelessConOrcConfig {
    pub domain_id: String,
    pub api_addr: String,
}
#[derive(Clone)]
pub struct EdgelessConSettings {
    pub controller_grpc_api_addr: String,
    pub orchestrators: Vec<EdgelessConOrcConfig>,
}

pub async fn edgeless_con_main(settings: EdgelessConSettings) {
    log::info!("Starting Edgeless Controller");

    let (mut controller, controller_task) = controller::Controller::new(settings.clone());

    let server_task =
        edgeless_api::grpc_impl::con::WorkflowInstanceAPIServer::run(controller.get_api_client(), settings.controller_grpc_api_addr.clone());

    futures::join!(controller_task, server_task);
}
