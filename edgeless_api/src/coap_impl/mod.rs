// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub mod agent;
pub mod function_instance;
pub mod invocation;
pub mod node_management;
pub mod node_registration;
pub mod orchestration;
pub mod resource_configuration;

#[derive(Clone)]
pub struct CoapClient {
    inner: std::sync::Arc<tokio::sync::Mutex<CoapClientInner>>,
}

struct CoapClientInner {
    sock: tokio::net::UdpSocket,
    endpoint: std::net::SocketAddrV4,
    next_token: u8,
}

impl CoapClient {
    pub async fn new(peer: std::net::SocketAddrV4) -> Self {
        let sock = tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();

        CoapClient {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(CoapClientInner {
                sock,
                endpoint: peer,
                next_token: 0,
            })),
        }
    }

    async fn call_with_reply<'a>(
        &mut self,
        encode_request: impl Fn(u8, std::net::SocketAddrV4, &mut [u8]) -> ((&mut [u8], std::net::SocketAddrV4), &mut [u8]),
    ) -> Result<Vec<u8>, Vec<u8>> {
        let mut lck = self.inner.lock().await;

        let mut buffer = vec![0 as u8; 5000];

        let token = lck.next_token;
        (lck.next_token, _) = lck.next_token.overflowing_add(1);

        for _ in 0..3 {
            let ((packet, addr), _tail) = encode_request(token, lck.endpoint, &mut buffer);
            lck.sock.send_to(&packet, addr).await.unwrap();

            let res = tokio::time::timeout(core::time::Duration::from_millis(500), lck.sock.recv_from(&mut buffer)).await;

            if res.is_err() {
                log::debug!("reached receive timeout");
                continue;
            }

            let (size, sender) = res.unwrap().unwrap();

            if sender != std::net::SocketAddr::V4(lck.endpoint) {
                continue;
            }
            let (res, response_token) = edgeless_api_core::coap_mapping::CoapDecoder::decode(&buffer[..size]).unwrap();
            match res {
                edgeless_api_core::coap_mapping::CoapMessage::Response(response_data, ok) => {
                    if response_token == token {
                        match ok {
                            true => return Ok(Vec::from(response_data)),
                            false => return Err(Vec::from(response_data)),
                        }
                    } else {
                        log::info!("received wrong token: wanted {}, got: {}", token, response_token);
                    }
                }
                _ => {
                    log::info!("received wrong message");
                }
            }
        }
        Err(Vec::from("client cimeout".as_bytes()))
    }
}
