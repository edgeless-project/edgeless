// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use futures::FutureExt;

pub mod agent;
pub mod function_instance;
pub mod invocation;
pub mod node_management;
pub mod node_register;
pub mod node_registration;
pub mod resource_configuration;

#[derive(Clone)]
pub struct CoapClient {
    inner: std::sync::Arc<tokio::sync::Mutex<CoapClientInner>>,
    outgoing_sender: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    network_task_abort_handle: Option<std::sync::Arc<tokio::task::AbortHandle>>,
}

struct CoapClientInner {
    endpoint: std::net::SocketAddrV4,
    next_token: u8,
    #[allow(clippy::type_complexity)]
    active_requests: std::collections::HashMap<u8, tokio::sync::oneshot::Sender<Result<Vec<u8>, Vec<u8>>>>,
}

impl CoapClient {
    pub async fn new(peer: std::net::SocketAddrV4) -> Self {
        let sock = tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();

        let inner = std::sync::Arc::new(tokio::sync::Mutex::new(CoapClientInner {
            endpoint: peer,
            next_token: 0,
            active_requests: std::collections::HashMap::new(),
        }));

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        let inner_clone = inner.clone();
        let task_handle = tokio::task::spawn(Self::network_task(sock, inner_clone, peer, receiver));

        CoapClient {
            inner: inner.clone(),
            outgoing_sender: sender,
            network_task_abort_handle: Some(std::sync::Arc::new(task_handle.abort_handle())),
        }
    }

    async fn network_task(
        sock: tokio::net::UdpSocket,
        inner: std::sync::Arc<tokio::sync::Mutex<CoapClientInner>>,
        endpoint: std::net::SocketAddrV4,
        mut outgoing_receiver: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    ) {
        loop {
            let mut buffer = vec![0_u8; 5000];

            futures::select! {
                msg = sock.recv_from(&mut buffer).fuse() => {
                    log::debug!("Receive");
                    if let Ok((len, sender)) = msg {
                        if sender != std::net::SocketAddr::V4(endpoint) {
                            log::debug!("Wrong Sender");
                            continue;
                        }

                        let (res, response_token) = edgeless_api_core::coap_mapping::CoapDecoder::decode(&buffer[..len]).unwrap();
                        match res {
                            edgeless_api_core::coap_mapping::CoapMessage::Response(response_data, ok) => {
                                if let Some(responder) = inner.lock().await.active_requests.remove(&response_token) {
                                    if responder.send(
                                        match ok {
                                            true => Ok(Vec::from(response_data)),
                                            false => Err(Vec::from(response_data)),
                                        }
                                    ).is_err() {
                                        log::warn!("Could not send response");
                                    }
                                } else {
                                    log::debug!("No interested receiver for message");
                                }
                            }
                            _ => {
                                log::info!("Received wrong message");
                            }
                        }
                    } else {
                        log::warn!("Socket Receive Error");
                    }

                },
                outgoing = outgoing_receiver.recv().fuse() => {
                    if let Some(outgoing) = outgoing {
                        log::debug!("COAP: {:?} {:?}", endpoint, outgoing);
                        sock.send_to(&outgoing, endpoint).await.unwrap();
                    }
                }
            }
        }
    }

    async fn call_with_reply(
        &mut self,
        encode_request: impl Fn(u8, std::net::SocketAddrV4, &mut [u8]) -> ((&mut [u8], std::net::SocketAddrV4), &mut [u8]),
    ) -> Result<Vec<u8>, Vec<u8>> {
        let (token, endpoint, mut receiver) = {
            let mut lck = self.inner.lock().await;
            let token = lck.next_token;
            (lck.next_token, _) = lck.next_token.overflowing_add(1);

            let (sender, receiver) = tokio::sync::oneshot::channel::<Result<Vec<u8>, Vec<u8>>>();

            lck.active_requests.insert(token, sender);
            (token, lck.endpoint, receiver)
        };

        for i in 0..3 {
            let mut buffer = vec![0_u8; 5000];
            let ((packet, _addr), _tail) = encode_request(token, endpoint, &mut buffer);
            if self.outgoing_sender.send(Vec::from(packet)).is_err() {
                log::warn!("Sender could not send on iteration {}", i);
            }

            let res = tokio::time::timeout(core::time::Duration::from_millis(500), &mut receiver).await;
            match res {
                Ok(reply) => {
                    if let Ok(val) = reply {
                        return val;
                    }
                }
                Err(_timeout) => {
                    log::debug!("reached receive timeout");
                    continue;
                }
            }
        }
        Err(Vec::from("client timeout".as_bytes()))
    }
}

impl Drop for CoapClient {
    fn drop(&mut self) {
        if let Some(abort_handle) = self.network_task_abort_handle.take() {
            if let Some(abort_handle) = std::sync::Arc::into_inner(abort_handle) {
                log::info!("Network task stopped.");
                abort_handle.abort();
            }
        } else {
            log::warn!("Network task handle removed before drop.");
        }
    }
}
