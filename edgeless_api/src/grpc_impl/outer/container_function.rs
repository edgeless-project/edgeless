// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct ContainerFunctionAPIClient {
    guest_api_function: Box<dyn crate::guest_api_function::GuestAPIFunction>,
}

impl ContainerFunctionAPIClient {
    pub async fn new(api_addr: &str, timeout: std::time::Duration) -> anyhow::Result<Self> {
        Ok(Self {
            guest_api_function: match crate::grpc_impl::inner::guest_api_function::GuestAPIFunctionClient::new(api_addr, timeout).await {
                Ok(val) => Box::new(val),
                Err(err) => return Err(err),
            },
        })
    }
}

impl crate::outer::container_function::ContainerFunctionAPI for ContainerFunctionAPIClient {
    fn guest_api_function(&mut self) -> Box<dyn crate::guest_api_function::GuestAPIFunction> {
        self.guest_api_function.clone()
    }
}

pub struct GuestAPIFunctionServer {}

impl GuestAPIFunctionServer {
    pub fn run(
        container_function_api: Box<dyn crate::outer::container_function::ContainerFunctionAPI + Send>,
        container_function_url: String,
        tls_config: Option<crate::grpc_impl::tls_config::TlsConfig>,
    ) -> futures::future::BoxFuture<'static, ()> {
        let mut container_function_api = container_function_api;
        let workflow_api = crate::grpc_impl::inner::guest_api_function::GuestAPIFunctionService {
            guest_api_function: tokio::sync::Mutex::new(container_function_api.guest_api_function()),
        };
        Box::pin(async move {
            let workflow_api = workflow_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&container_function_url)
                && let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start ContainerFunctionAPI GRPC Server at {}", container_function_url);

                    let mut server_builder = tonic::transport::Server::builder();

                    if let Some(tls_config) = tls_config {
                        match tls_config.create_server_tls_config() {
                            Ok(Some(config)) => {
                                log::info!("TLS enabled for GRPC server");
                                match server_builder.tls_config(config) {
                                    Ok(builder) => server_builder = builder,
                                    Err(e) => {
                                        log::error!("Failed to apply TLS config: {}", e);
                                        return;
                                    }
                                }
                            }
                            Ok(None) => {
                                log::info!("TLS disabled for GRPC server");
                            }
                            Err(e) => {
                                log::error!("Failed to create TLS config: {}", e);
                                return;
                            }
                        }
                    }

                    match server_builder
                        .add_service(
                            crate::grpc_impl::api::guest_api_function_server::GuestApiFunctionServer::new(workflow_api)
                                .max_decoding_message_size(usize::MAX),
                        )
                        .serve(host)
                        .await
                    {
                        Ok(_) => {
                            log::debug!("Clean Exit");
                        }
                        Err(_) => {
                            log::error!("GRPC Server Failure");
                        }
                    }
                }

            log::info!("Stop ContainerFunctionAPI GRPC Server");
        })
    }
}
