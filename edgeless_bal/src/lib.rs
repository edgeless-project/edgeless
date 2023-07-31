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

    let multi_resouce_api = Box::new(edgeless_api::resource_configuration::MultiResouceConfigurationAPI::new(
        std::collections::HashMap::<String, Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI>>::from([
            ("http-ingress-1".to_string(), ingress)
        ]),
    ));

    let api_server =
        edgeless_api::grpc_impl::resource_configuration::ResourceConfigurationServer::run(multi_resouce_api, settings.resource_configuration_url);
    api_server.await;
}

pub fn edgeless_bal_default_conf() -> String {
    String::from(
        r##"balancer_id = "2bb0867f-e9ee-4a3a-8872-dbaa5228ee23"
invocation_url = "http://127.0.0.1:7032"
resource_configuration_url = "http://127.0.0.1:7033"
http_ingress_url = "http://127.0.0.1:7035"
nodes = [
        {id = "fda6ce79-46df-4f96-a0d2-456f720f606c", invocation_url="http://127.0.0.1:7002" }
]
"##,
    )
}
