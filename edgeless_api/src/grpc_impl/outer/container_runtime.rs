// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct ContainerRuntimeAPIClient {
    guest_api_host: Box<dyn crate::guest_api_host::GuestAPIHost>,
}

impl ContainerRuntimeAPIClient {
    pub async fn new(api_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        Ok(Self {
            guest_api_host: match crate::grpc_impl::inner::guest_api_host::GuestAPIHostClient::new(api_addr, retry_interval).await {
                Ok(val) => Box::new(val),
                Err(err) => return Err(err),
            },
        })
    }
}

impl crate::outer::container_runtime::ContainerRuntimeAPI for ContainerRuntimeAPIClient {
    fn guest_api_host(&mut self) -> Box<dyn crate::guest_api_host::GuestAPIHost> {
        self.guest_api_host.clone()
    }
}

pub struct GuestAPIHostServer {}

impl GuestAPIHostServer {
    pub fn run(
        container_runtime_api: Box<dyn crate::outer::container_runtime::ContainerRuntimeAPI + Send>,
        container_runtime_url: String,
        tls_config: Option<crate::grpc_impl::tls_config::TlsConfig>,
    ) -> futures::future::BoxFuture<'static, ()> {
        let mut container_runtime_api = container_runtime_api;
        let workflow_api = crate::grpc_impl::inner::guest_api_host::GuestAPIHostService {
            guest_api_host: tokio::sync::Mutex::new(container_runtime_api.guest_api_host()),
        };
        Box::pin(async move {
            let workflow_api = workflow_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&container_runtime_url)
                && let Ok(addr) = format!("{}:{}", host, port).parse() {
                    log::info!("Start ContainerRuntimeAPI GRPC Server at {}", container_runtime_url);

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
                            crate::grpc_impl::api::guest_api_host_server::GuestApiHostServer::new(workflow_api).max_decoding_message_size(usize::MAX),
                        )
                        .serve(addr)
                        .await
                    {
                        Ok(_) => {
                            log::debug!("Clean Exit");
                        }
                        Err(e) => {
                            log::error!("GRPC Server Failure: {}", e);
                        }
                    }
                }

            log::info!("Stop ContainerRuntimeAPI GRPC Server");
        })
    }
}
