// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

pub struct CoapInvocationServer {
    sock: tokio::net::UdpSocket,
    root_api: Box<dyn crate::invocation::InvocationAPI>,
}

impl CoapInvocationServer {
    pub fn run(
        data_plane: Box<dyn crate::invocation::InvocationAPI>,
        listen_addr: std::net::SocketAddrV4,
    ) -> futures::future::BoxFuture<'static, ()> {
        Box::pin(async move {
            let sck = tokio::net::UdpSocket::bind(listen_addr).await.unwrap();

            let mut slf = CoapInvocationServer {
                sock: sck,
                root_api: data_plane,
            };

            let mut buffer = vec![0 as u8; 5000];

            let mut received_tokens: std::collections::HashMap<std::net::IpAddr, u8> = std::collections::HashMap::new();

            loop {
                let (size, sender) = slf.sock.recv_from(&mut buffer[..]).await.unwrap();

                let (pack, token) = edgeless_api_core::coap_mapping::CoapDecoder::decode(&buffer[..size]).unwrap();
                match pack {
                    edgeless_api_core::coap_mapping::CoapMessage::Invocation(invocation_event) => {
                        let event = crate::invocation::Event {
                            target: invocation_event.target,
                            source: invocation_event.source,
                            stream_id: invocation_event.stream_id,
                            data: match invocation_event.data {
                                edgeless_api_core::invocation::EventData::Cast(val) => {
                                    crate::invocation::EventData::Cast(String::from_utf8(val.to_vec()).unwrap())
                                }
                                edgeless_api_core::invocation::EventData::Call(val) => {
                                    crate::invocation::EventData::Call(String::from_utf8(val.to_vec()).unwrap())
                                }
                                edgeless_api_core::invocation::EventData::CallRet(val) => {
                                    crate::invocation::EventData::CallRet(String::from_utf8(val.to_vec()).unwrap())
                                }
                                edgeless_api_core::invocation::EventData::CallNoRet => crate::invocation::EventData::CallNoRet,
                                edgeless_api_core::invocation::EventData::Err => crate::invocation::EventData::Err,
                            },
                        };

                        let key_entry = received_tokens.entry(sender.ip());

                        match key_entry {
                            std::collections::hash_map::Entry::Vacant(entry) => {
                                slf.root_api.handle(event).await.unwrap();
                                entry.insert(token);
                            }
                            std::collections::hash_map::Entry::Occupied(mut entry) => {
                                if entry.get() < &token || token == 0 {
                                    slf.root_api.handle(event).await.unwrap();
                                    entry.insert(token);
                                } else {
                                    log::info!("Message Duplicate: {} !< {}", entry.get(), token);
                                }
                            }
                        }
                    }
                    _ => {
                        log::info!("Unhandled COAP Message");
                    }
                }
            }
        })
    }
}

#[async_trait::async_trait]
impl crate::invocation::InvocationAPI for super::CoapClient {
    async fn handle(&mut self, event: crate::invocation::Event) -> anyhow::Result<crate::invocation::LinkProcessingResult> {
        let encoded_event = edgeless_api_core::invocation::Event::<&[u8]> {
            target: event.target,
            source: event.source,
            stream_id: event.stream_id,
            data: match &event.data {
                crate::invocation::EventData::Cast(val) => edgeless_api_core::invocation::EventData::Cast(val.as_bytes().into()),
                crate::invocation::EventData::Call(val) => edgeless_api_core::invocation::EventData::Call(val.as_bytes().into()),
                crate::invocation::EventData::CallRet(val) => edgeless_api_core::invocation::EventData::CallRet(val.as_bytes().into()),
                crate::invocation::EventData::CallNoRet => edgeless_api_core::invocation::EventData::CallNoRet,
                crate::invocation::EventData::Err => edgeless_api_core::invocation::EventData::Err,
            },
        };

        let mut lck = self.inner.lock().await;

        let mut buffer = vec![0 as u8; 2000];

        let token = lck.next_token;
        lck.next_token = if lck.next_token == u8::MAX { 0 } else { lck.next_token + 1 };

        let ((packet, addr), _tail) =
            edgeless_api_core::coap_mapping::COAPEncoder::encode_invocation_event(lck.endpoint, encoded_event, token, &mut buffer[..]);
        self.outgoing_sender.send(Vec::from(packet)).unwrap();
        Ok(crate::invocation::LinkProcessingResult::FINAL)
    }
}
