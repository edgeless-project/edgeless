// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct DomainRegisterAPIClient {
    domain_registration_client: Box<dyn crate::domain_registration::DomainRegistrationAPI>,
}

impl DomainRegisterAPIClient {
    pub async fn new(api_addr: String, tls_config: Option<crate::grpc_impl::tls_config::TlsConfig>) -> Self {
        Self {
            domain_registration_client: Box::new(crate::grpc_impl::inner::domain_registration::DomainRegistrationAPIClient::new(
                api_addr,
                tls_config.clone(),
            )),
        }
    }
}

impl crate::outer::domain_register::DomainRegisterAPI for DomainRegisterAPIClient {
    fn domain_registration_api(&mut self) -> Box<dyn crate::domain_registration::DomainRegistrationAPI> {
        self.domain_registration_client.clone()
    }
}

pub struct DomainRegistrationAPIServer {}

impl DomainRegistrationAPIServer {
    pub fn run(
        domain_register_api: Box<dyn crate::outer::domain_register::DomainRegisterAPI + Send>,
        domain_registration_url: String,
        tls_config: Option<crate::grpc_impl::tls_config::TlsConfig>,
    ) -> futures::future::BoxFuture<'static, ()> {
        let mut domain_register_api = domain_register_api;
        let domain_registration_api = crate::grpc_impl::inner::domain_registration::DomainRegistrationAPIServer {
            domain_registration_api: tokio::sync::Mutex::new(domain_register_api.domain_registration_api()),
        };
        Box::pin(async move {
            let domain_registration_api = domain_registration_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&domain_registration_url)
                && let Ok(host) = format!("{}:{}", host, port).parse()
            {
                log::info!("Start DomainRegisterAPI GRPC Server at {}", domain_registration_url);

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
                        crate::grpc_impl::api::domain_registration_server::DomainRegistrationServer::new(domain_registration_api)
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

            log::info!("Stop DomainRegisterAPI GRPC Server");
        })
    }
}
