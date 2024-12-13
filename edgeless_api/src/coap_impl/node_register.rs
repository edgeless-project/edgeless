// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct CoapNodeRegisterServer {
    sock: tokio::net::UdpSocket,
    registration_api: Box<dyn crate::node_registration::NodeRegistrationAPI>,
    received_tokens: std::collections::HashMap<std::net::IpAddr, u8>,
    tx_buffer: Vec<u8>,
}

impl CoapNodeRegisterServer {
    pub fn run(
        registration_api: Box<dyn crate::node_registration::NodeRegistrationAPI>,
        listen_addr: std::net::SocketAddrV4,
    ) -> futures::future::BoxFuture<'static, ()> {
        Box::pin(async move {
            log::info!("Start CoapNodeRegister server at {}", listen_addr);
            let sck = tokio::net::UdpSocket::bind(listen_addr).await.unwrap();

            let mut slf = CoapNodeRegisterServer {
                sock: sck,
                registration_api,
                tx_buffer: vec![0_u8; 5000],
                received_tokens: std::collections::HashMap::new(),
            };

            let mut buffer = vec![0_u8; 5000];

            loop {
                let (size, sender) = slf.sock.recv_from(&mut buffer[..]).await.unwrap();

                let (pack, token) = edgeless_api_core::coap_mapping::CoapDecoder::decode(&buffer[..size]).unwrap();
                match pack {
                    edgeless_api_core::coap_mapping::CoapMessage::NodeRegistration(registration) => {
                        slf.process_node_registration(&registration, token, sender).await;
                    }
                    _ => {
                        log::info!("Unhandled Message");
                    }
                }
            }
        })
    }

    async fn process_node_registration(
        &mut self,
        registration: &edgeless_api_core::node_registration::EncodedNodeRegistration<'_>,
        token: u8,
        sender: core::net::SocketAddr,
    ) {
        let key_entry = self.received_tokens.entry(sender.ip());

        let registration = crate::node_registration::UpdateNodeRequest {
            node_id: registration.node_id.0,
            agent_url: String::from(registration.agent_url.as_str()),
            invocation_url: String::from(registration.invocation_url.as_str()),
            resource_providers: registration
                .resources
                .iter()
                .map(|core_spec| crate::node_registration::ResourceProviderSpecification {
                    provider_id: String::from(core_spec.provider_id),
                    class_type: String::from(core_spec.class_type),
                    outputs: core_spec.outputs.iter().map(|core_output| String::from(*core_output)).collect(),
                })
                .collect(),
            capabilities: crate::node_registration::NodeCapabilities::default(),
            refresh_deadline: std::time::SystemTime::now() + std::time::Duration::from_secs(86400),
            nonce: 0,
            health_status: crate::node_registration::NodeHealthStatus::default(),
            performance_samples: crate::node_registration::NodePerformanceSamples::default(),
        };

        let ret = match key_entry {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(token);
                match self.registration_api.update_node(registration).await.unwrap() {
                    crate::node_registration::UpdateNodeResponse::Accepted => Some(Ok(())),
                    crate::node_registration::UpdateNodeResponse::ResponseError(err) => Some(Err(err)),
                }
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if entry.get() < &token || token == 0 {
                    entry.insert(token);
                    match self.registration_api.update_node(registration).await.unwrap() {
                        crate::node_registration::UpdateNodeResponse::Accepted => Some(Ok(())),
                        crate::node_registration::UpdateNodeResponse::ResponseError(err) => Some(Err(err)),
                    }
                } else {
                    log::info!("Message Duplicate: {} !< {}", entry.get(), token);
                    Some(Ok(()))
                }
            }
        };

        if let Some(ret) = ret {
            let ((data, sender), _tail) = match ret {
                Ok(_) => edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, &mut self.tx_buffer[..], true),
                Err(_) => {
                    let (data, tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_error_response(
                        edgeless_api_core::common::ErrorResponse {
                            // Passing the error message would be desired. This requires ErrorResponse to be generic over str and strings (or lifetime annotations).
                            summary: "Server Error",
                            detail: None,
                        },
                        &mut self.tx_buffer[..],
                    );
                    edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, data, token, &mut tail[..], false)
                }
            };
            if let Err(err) = self.sock.send_to(data, sender).await {
                log::error!("UDP/COAP Send Error: {:?}", err);
            }
        }
    }
}
