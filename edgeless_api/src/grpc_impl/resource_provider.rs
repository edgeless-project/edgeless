pub struct ResourceProviderAPIClient {
    resource_configuration_client: Box<dyn crate::resource_configuration::ResourceConfigurationAPI>,
}

impl ResourceProviderAPIClient {
    pub async fn new(api_addr: &str) -> Self {
        let (proto, url, port) = crate::util::parse_http_host(&api_addr).unwrap();
        Self {
            resource_configuration_client: match proto {
                crate::util::Proto::COAP => {
                    log::info!("coap called");
                    Box::new(crate::coap_impl::CoapClient::new(std::net::SocketAddrV4::new(url.parse().unwrap(), port)).await)
                }
                _ => Box::new(crate::grpc_impl::resource_configuration::ResourceConfigurationAPIClient::new(api_addr, true).await),
            },
        }
    }
}

impl crate::resource_provider::ResourceProviderAPI for ResourceProviderAPIClient {
    fn resource_configuration_api(&mut self) -> Box<dyn crate::resource_configuration::ResourceConfigurationAPI> {
        self.resource_configuration_client.clone()
    }
}

pub struct ResourceProviderAPIServer {}

impl ResourceProviderAPIServer {
    pub fn run(
        root_api: Box<dyn crate::resource_configuration::ResourceConfigurationAPI>,
        resource_configuration_url: String,
    ) -> futures::future::BoxFuture<'static, ()> {
        let function_api = crate::grpc_impl::resource_configuration::ResourceConfigurationAPIServer {
            root_api: tokio::sync::Mutex::new(root_api),
        };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&resource_configuration_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start ResourceConfiguration GRPC Server at {}", resource_configuration_url);
                    match tonic::transport::Server::builder()
                        .add_service(
                            crate::grpc_impl::api::resource_configuration_server::ResourceConfigurationServer::new(function_api)
                                .max_decoding_message_size(usize::MAX),
                        )
                        .serve(host)
                        .await
                    {
                        Ok(_) => {
                            log::debug!("Clean Exit");
                        }
                        Err(_) => {
                            log::error!("GRPC ResourceConfiguration Failure");
                        }
                    }
                }
            }

            log::info!("Stop ResourceConfiguration GRPC Server");
        })
    }
}
