// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use crate::invocation::InvocationAPI;
use crate::resource_configuration::ResourceConfigurationAPI;

struct CoapMultiplexer {
    sock: embassy_net::udp::UdpSocket<'static>,
    out_reader: embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, crate::agent::AgentEvent, 2>,
    agent: crate::agent::EmbeddedAgent,
    app_buf_tx: [u8; 5000],
    last_tokens: heapless::LinearMap<
        smoltcp::wire::IpEndpoint,
        (
            u8,
            Option<Result<edgeless_api_core::instance_id::InstanceId, edgeless_api_core::common::ErrorResponse>>,
        ),
        4,
    >,
    peers: heapless::LinearMap<edgeless_api_core::node_registration::NodeId, smoltcp::wire::IpEndpoint, 8>,
    token: u8,
    waiting_for_reply: Option<(
        u8,
        &'static embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::NoopRawMutex, crate::agent::RegistrationReply>,
    )>,
}

#[embassy_executor::task]
pub async fn coap_task(
    mut sock: embassy_net::udp::UdpSocket<'static>,
    out_reader: embassy_sync::channel::Receiver<'static, embassy_sync::blocking_mutex::raw::NoopRawMutex, crate::agent::AgentEvent, 2>,
    agent: crate::agent::EmbeddedAgent,
) {
    sock.bind(7050).unwrap();

    let mut slf = CoapMultiplexer {
        sock,
        out_reader,
        agent,
        app_buf_tx: [0_u8; 5000],
        last_tokens: heapless::LinearMap::new(),
        peers: heapless::LinearMap::new(),
        token: 0,
        waiting_for_reply: None,
    };

    slf.task().await;
}

impl CoapMultiplexer {
    async fn task(&mut self) {
        let mut app_buf = [0_u8; 5000];
        loop {
            let res = embassy_futures::select::select(self.sock.recv_from(&mut app_buf), self.out_reader.receive()).await;

            match res {
                // External Message Received
                embassy_futures::select::Either::First(res) => {
                    let (data_len, sender) = match res {
                        Ok(ret) => ret,
                        Err(err) => {
                            log::error!("UDP/COAP Receive Error: {:?}", err);
                            continue;
                        }
                    };
                    let (message, token) = match edgeless_api_core::coap_mapping::CoapDecoder::decode(&app_buf[..data_len]) {
                        Ok(ret) => ret,
                        Err(err) => {
                            log::error!("UDP/COAP Decode Error: {:?}", err);
                            continue;
                        }
                    };
                    match message {
                        edgeless_api_core::coap_mapping::CoapMessage::Invocation(invocation) => {
                            self.incoming_invocation(sender, token, invocation).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::ResourceStart(start_spec) => {
                            self.incoming_resource_start(sender, token, start_spec).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::ResourceStop(stop_instance_id) => {
                            self.incoming_resource_stop(sender, token, stop_instance_id).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::ResourcePatch(patch_req) => {
                            self.incoming_resource_patch(sender, token, patch_req).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::PeerAdd((node_id, addr, port)) => {
                            self.incoming_peer_add(sender, token, node_id, &addr, port).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::PeerRemove(node_id) => {
                            self.incoming_peer_remove(sender, token, node_id).await;
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::Response(data, success) => {
                            log::info!("Got Response: {}, {}", data.len(), success);
                            if let Some((t, channel)) = self.waiting_for_reply.take()
                                && t == token {
                                    channel.signal(crate::agent::RegistrationReply::Sucess)
                                }
                        }
                        edgeless_api_core::coap_mapping::CoapMessage::KeepAlive => {
                            self.incoming_keepalive(sender, token).await;
                        }
                        _ => {
                            log::info!("Unhandled Message");
                        }
                    }
                }
                // Internal Message that needs to be sent out.
                embassy_futures::select::Either::Second(event) => match &event {
                    crate::agent::AgentEvent::Invocation(event) => {
                        self.outgoing_invocation(event).await;
                    }
                    crate::agent::AgentEvent::Registration((registration, reply_signal)) => {
                        self.outgoing_registration(registration, reply_signal).await;
                    }
                },
            }
        }
    }

    async fn incoming_invocation(&mut self, sender: smoltcp::wire::IpEndpoint, token: u8, invocation: edgeless_api_core::invocation::Event<&[u8]>) {
        let key_entry = self.last_tokens.get_mut(&sender);
        match key_entry {
            None => {
                self.agent.handle(invocation).await.unwrap();
                if self.last_tokens.insert(sender, (token, None)).is_err() {
                    log::info!("Could not store token, duplicate delivery is possible!");
                }
            }
            // While we don't send back a response, we still need to block duplicate delivery.
            Some((entry, _message)) => {
                if *entry < token || token == 0 {
                    self.agent.handle(invocation).await.unwrap();
                    *entry = token;
                }
            }
        }
    }

    #[allow(clippy::needless_lifetimes)]
    async fn incoming_resource_start<'a>(
        &mut self,
        sender: smoltcp::wire::IpEndpoint,
        token: u8,
        start_spec: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification<'a>,
    ) {
        let key_entry = self.last_tokens.get_mut(&sender);

        let ret = match key_entry {
            None => {
                let response = self.agent.start(start_spec.clone()).await;
                if self.last_tokens.insert(sender, (token, Some(response.clone()))).is_err() {
                    log::info!("Could not store token, duplicate delivery is possible!");
                }
                Some(response)
            }
            Some((stored_token, stored_response)) => {
                if *stored_token < token || token == 0 {
                    let id = self.agent.start(start_spec).await;
                    *stored_token = token;
                    *stored_response = Some(id.clone());

                    Some(id)
                } else if *stored_token == token {
                    stored_response.clone()
                } else {
                    None
                }
            }
        };

        if let Some(ret) = ret {
            let is_ok = ret.is_ok();

            let (encoded, tail) = match ret {
                Ok(id) => edgeless_api_core::coap_mapping::COAPEncoder::encode_instance_id(id, &mut self.app_buf_tx[..]),
                Err(err) => edgeless_api_core::coap_mapping::COAPEncoder::encode_error_response(err, &mut self.app_buf_tx[..]),
            };

            let ((data, sender), _tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, encoded, token, &mut tail[..], is_ok);
            if let Err(err) = self.sock.send_to(data, sender).await {
                log::error!("UDP/COAP Send Error: {:?}", err);
            }
        }
    }

    async fn incoming_resource_stop(
        &mut self,
        sender: smoltcp::wire::IpEndpoint,
        token: u8,
        stop_instance_id: edgeless_api_core::instance_id::InstanceId,
    ) {
        let key_entry = self.last_tokens.get_mut(&sender);

        let ret = match key_entry {
            None => {
                let res = self.agent.stop(stop_instance_id).await;
                if self.last_tokens.insert(sender, (token, None)).is_err() {
                    log::info!("Could not store token, duplicate delivery is possible!");
                }
                Some(res)
            }
            Some((stored_token, _stored_response)) => {
                if *stored_token < token || token == 0 {
                    *stored_token = token;
                    Some(self.agent.stop(stop_instance_id).await)
                } else {
                    None
                }
            }
        };

        if let Some(ret) = ret {
            let ((data, sender), _tail) = match ret {
                Ok(_) => edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, &mut self.app_buf_tx[..], true),
                Err(err) => {
                    let (data, tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_error_response(err, &mut self.app_buf_tx[..]);
                    edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, data, token, &mut tail[..], false)
                }
            };
            if let Err(err) = self.sock.send_to(data, sender).await {
                log::error!("UDP/COAP Send Error: {:?}", err);
            }
        }
    }

    #[allow(clippy::needless_lifetimes)]
    async fn incoming_resource_patch<'a>(
        &mut self,
        sender: smoltcp::wire::IpEndpoint,
        token: u8,
        patch_req: edgeless_api_core::resource_configuration::EncodedPatchRequest<'a>,
    ) {
        let key_entry = self.last_tokens.get_mut(&sender);

        let ret = match key_entry {
            None => {
                let response = self.agent.patch(patch_req.clone()).await;
                if self.last_tokens.insert(sender, (token, None)).is_err() {
                    log::info!("Could not store token, duplicate delivery is possible!");
                }
                Some(response)
            }
            Some((stored_token, _stored_response)) => {
                if *stored_token < token || token == 0 {
                    *stored_token = token;
                    Some(self.agent.patch(patch_req).await)
                } else {
                    None
                }
            }
        };

        if let Some(ret) = ret {
            let ((data, sender), _tail) = match ret {
                Ok(_) => edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, &mut self.app_buf_tx[..], true),
                Err(err) => {
                    let (data, tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_error_response(err, &mut self.app_buf_tx[..]);
                    edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, data, token, &mut tail[..], false)
                }
            };
            if let Err(err) = self.sock.send_to(data, sender).await {
                log::error!("UDP/COAP Send Error: {:?}", err);
            }
        }
    }

    async fn incoming_peer_add(&mut self, sender: smoltcp::wire::IpEndpoint, token: u8, node_id: uuid::Uuid, addr: &[u8], port: u16) {
        log::info!("Got Peer Add {:?}, {}", addr, port);
        if self
            .peers
            .insert(
                edgeless_api_core::node_registration::NodeId(node_id),
                smoltcp::wire::IpEndpoint {
                    addr: smoltcp::wire::IpAddress::from(smoltcp::wire::Ipv4Address::from_bytes(addr)),
                    port,
                },
            )
            .is_err()
        {
            log::error!("Too many peers!");
        }
        let ((data, sender), _tail) =
            edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, &mut self.app_buf_tx[..], true);
        if let Err(err) = self.sock.send_to(data, sender).await {
            log::error!("UDP/COAP Send Error: {:?}", err);
        }
    }

    async fn incoming_peer_remove(&mut self, sender: smoltcp::wire::IpEndpoint, token: u8, node_id: uuid::Uuid) {
        log::info!("Got Peer Remove");
        self.peers.remove(&edgeless_api_core::node_registration::NodeId(node_id));
        let ((data, sender), _tail) =
            edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, &mut self.app_buf_tx[..], true);
        if let Err(err) = self.sock.send_to(data, sender).await {
            log::error!("UDP/COAP Send Error: {:?}", err);
        }
    }

    async fn incoming_keepalive(&mut self, sender: smoltcp::wire::IpEndpoint, token: u8) {
        let ((data, sender), _tail) =
            edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, &mut self.app_buf_tx[..], true);
        if let Err(err) = self.sock.send_to(data, sender).await {
            log::error!("keepalive UDP/COAP send error: {:?}", err);
        } else {
            log::debug!("Sent Keepalive response");
        }
    }

    async fn outgoing_invocation(&mut self, event: &edgeless_api_core::invocation::Event<heapless::Vec<u8, 1500>>) {
        if let Some(peer) = self.peers.get(&edgeless_api_core::node_registration::NodeId(event.target.node_id)) {
            let new_event: edgeless_api_core::invocation::Event<&[u8]> = edgeless_api_core::invocation::Event::<&[u8]> {
                target: event.target,
                source: event.source,
                stream_id: event.stream_id,
                data: match &event.data {
                    edgeless_api_core::invocation::EventData::Cast(val) => edgeless_api_core::invocation::EventData::Cast(val),
                    edgeless_api_core::invocation::EventData::Call(val) => edgeless_api_core::invocation::EventData::Call(val),
                    edgeless_api_core::invocation::EventData::CallRet(val) => edgeless_api_core::invocation::EventData::CallRet(val),
                    edgeless_api_core::invocation::EventData::CallNoRet => edgeless_api_core::invocation::EventData::CallNoRet,
                    edgeless_api_core::invocation::EventData::Err => edgeless_api_core::invocation::EventData::Err,
                },
                created: event.created,
                metadata: event.metadata.clone(),
            };

            let ((data, endpoint), _tail) =
                edgeless_api_core::coap_mapping::COAPEncoder::encode_invocation_event(peer, new_event, self.token, &mut self.app_buf_tx[..]);
            self.token = match self.token {
                u8::MAX => 0,
                _ => self.token + 1,
            };
            if let Err(err) = self.sock.send_to(data, *endpoint).await {
                log::error!("UDP/COAP Send Error: {:?}", err);
            }
            // we don't wait for a reply here.
        }
    }

    async fn outgoing_registration(
        &mut self,
        registration: &edgeless_api_core::node_registration::EncodedNodeRegistration<'static>,
        reply_channel: &'static embassy_sync::signal::Signal<embassy_sync::blocking_mutex::raw::NoopRawMutex, crate::agent::RegistrationReply>,
    ) {
        let endpoint = crate::REGISTRATION_PEER;
        let ((data, endpoint), _tail) =
            edgeless_api_core::coap_mapping::COAPEncoder::encode_node_registration(endpoint, registration, self.token, &mut self.app_buf_tx[..]);
        let used_token = self.token;
        self.token = match self.token {
            u8::MAX => 0,
            _ => self.token + 1,
        };
        if let Err(err) = self.sock.send_to(data, endpoint).await {
            log::error!("UDP/COAP Send Error: {:?}", err);
        } else {
            self.waiting_for_reply = Some((used_token, reply_channel))
        }
    }
}
