pub mod file_log;
pub mod http_egress;
pub mod http_ingress;
pub mod redis;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessBalSettings {
    pub balancer_id: uuid::Uuid,
    pub invocation_url: String,
    pub resource_configuration_url: String,
    pub http_ingress_url: String,
}

pub async fn edgeless_bal_main(settings: EdgelessBalSettings) {
    log::info!("Starting Edgeless Balancer");
    log::debug!("Settings: {:?}", settings);
    let data_plane = edgeless_dataplane::handle::DataplaneProvider::new(settings.balancer_id.clone(), settings.invocation_url.clone()).await;
    // XXX configure e-BAL peers

    let ingress = http_ingress::ingress_task(
        data_plane.clone(),
        edgeless_api::function_instance::InstanceId::new(settings.balancer_id.clone()),
        settings.http_ingress_url.clone(),
    )
    .await;

    let egress = Box::new(
        http_egress::EgressResourceProvider::new(
            data_plane.clone(),
            edgeless_api::function_instance::InstanceId::new(settings.balancer_id.clone()),
        )
        .await,
    );

    let file_log = Box::new(
        file_log::FileLogResourceProvider::new(
            data_plane.clone(),
            edgeless_api::function_instance::InstanceId::new(settings.balancer_id.clone()),
        )
        .await,
    );

    let redis = Box::new(
        redis::RedisResourceProvider::new(
            data_plane.clone(),
            edgeless_api::function_instance::InstanceId::new(settings.balancer_id.clone()),
        )
        .await,
    );

    let multi_resouce_api = Box::new(edgeless_api::resource_configuration::MultiResouceConfigurationAPI::new(
        std::collections::HashMap::<String, Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI>>::from([
            ("http-ingress-1".to_string(), ingress),
            ("http-egress-1".to_string(), egress),
            ("file-log-1".to_string(), file_log),
            ("redis-1".to_string(), redis),
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
"##,
    )
}
