pub mod http_ingress;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct EdgelessBalSettings {
    pub balancer_id: uuid::Uuid,
    pub invocation_url: String,
    pub resource_configuration_url: String,
    pub http_ingress_url: String,
    pub nodes: Vec<edgeless_dataplane::EdgelessDataplaneSettingsPeer>,
}

pub async fn edgeless_bal_main(settings: EdgelessBalSettings) {
    log::info!("Starting Edgeless Balancer");
    log::debug!("Settings: {:?}", settings);
    let data_plane =
        edgeless_dataplane::DataPlaneChainProvider::new(settings.balancer_id.clone(), settings.invocation_url.clone(), settings.nodes.clone()).await;
    let ingress = http_ingress::ingress_task(
        data_plane.clone(),
        edgeless_api::function_instance::FunctionId::new(settings.balancer_id.clone()),
        settings.http_ingress_url.clone(),
    )
    .await;
    let api_server = edgeless_api::grpc_impl::resource_configuration::ResourceConfigurationServer::run(ingress, settings.resource_configuration_url);
    api_server.await;
}
