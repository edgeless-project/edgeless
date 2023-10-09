use edgeless_api_core::invocation::InvocationAPI;
use edgeless_api_core::resource_configuration::ResourceConfigurationAPI;

#[embassy_executor::task]
pub async fn coap_task(
    stack: &'static embassy_net::Stack<esp_wifi::wifi::WifiDevice<'static>>,
    out_reader: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        edgeless_api_core::invocation::Event<heapless::String<1500>>,
        2,
    >,
    agent: crate::agent::ResourceRegistry,
) {
    let mut agent = agent;

    let mut rx_buf = [0 as u8; 5000];
    let mut rx_meta = [embassy_net::udp::PacketMetadata::EMPTY; 10];
    let mut tx_buf = [0 as u8; 5000];
    let mut tx_meta = [embassy_net::udp::PacketMetadata::EMPTY; 10];
    let mut app_buf = [0 as u8; 5000];
    let mut app_buf_tx = [0 as u8; 5000];

    let mut sock = embassy_net::udp::UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);

    let mut last_tokens = heapless::LinearMap::<smoltcp::wire::IpEndpoint, (u8, Option<edgeless_api_core::instance_id::InstanceId>), 4>::new();

    let mut token = 0 as u8;

    sock.bind(7050).unwrap();

    loop {
        let res = embassy_futures::select::select(sock.recv_from(&mut app_buf), out_reader.receive()).await;

        match res {
            embassy_futures::select::Either::First(res) => {
                let (data_len, sender) = res.unwrap();
                let (message, token) = edgeless_api_core::coap_mapping::CoapDecoder::decode(&app_buf[..data_len]).unwrap();
                match message {
                    edgeless_api_core::coap_mapping::CoapMessage::Invocation(invocation) => {
                        let key_entry = last_tokens.get_mut(&sender);
                        match key_entry {
                            None => {
                                agent.handle(invocation).await.unwrap();
                                last_tokens.insert(sender.clone(), (token, None));
                            }
                            Some((entry, _message)) => {
                                if &*entry < &token || token == 0 {
                                    agent.handle(invocation).await.unwrap();
                                    *entry = token;
                                }
                            }
                        }
                    }
                    edgeless_api_core::coap_mapping::CoapMessage::ResourceStart(start_spec) => {
                        let key_entry = last_tokens.get_mut(&sender);
                        let id = match key_entry {
                            None => {
                                let id = agent.start(start_spec.clone()).await.unwrap();
                                last_tokens.insert(sender.clone(), (token, Some(id.clone())));
                                Some(id)
                            }
                            Some((entry, message)) => {
                                if &*entry < &token || token == 0 {
                                    let id = agent.start(start_spec).await.unwrap();
                                    *entry = token;
                                    *message = Some(id.clone());

                                    Some(id)
                                } else {
                                    if *entry == token && message.is_some() {
                                        Some(message.unwrap())
                                    } else {
                                        None
                                    }
                                }
                            }
                        };

                        if let Some(id) = id {
                            let mut buffer = [0; 128];
                            let (encoded_id, _tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_instance_id(id, &mut buffer[..]);
                            let ((data, sender), _tail) =
                                edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, encoded_id, token, &mut app_buf_tx[..]);
                            sock.send_to(data, sender).await;
                        }
                    }
                    edgeless_api_core::coap_mapping::CoapMessage::ResourceStop(stop_instance_id) => {
                        let key_entry = last_tokens.get_mut(&sender);
                        match key_entry {
                            None => {
                                agent.stop(stop_instance_id).await.unwrap();
                                last_tokens.insert(sender.clone(), (token, None));
                            }
                            Some((entry, _message)) => {
                                if &*entry < &token || token == 0 {
                                    agent.stop(stop_instance_id).await.unwrap();
                                    *entry = token;
                                }
                            }
                        }

                        let ((data, sender), _tail) =
                            edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, &mut app_buf_tx[..]);
                        sock.send_to(data, sender).await;
                    }
                    _ => {}
                }
            }
            embassy_futures::select::Either::Second(event) => {
                for (peer_id, peer) in &crate::COAP_PEERS {
                    if peer_id == &event.target.node_id {
                        let new_event: edgeless_api_core::invocation::Event<&[u8]> = edgeless_api_core::invocation::Event::<&[u8]> {
                            target: event.target,
                            source: event.source,
                            stream_id: event.stream_id,
                            data: match &event.data {
                                edgeless_api_core::invocation::EventData::Cast(val) => edgeless_api_core::invocation::EventData::Cast(val.as_bytes()),
                                edgeless_api_core::invocation::EventData::Call(val) => edgeless_api_core::invocation::EventData::Call(val.as_bytes()),
                                edgeless_api_core::invocation::EventData::CallRet(val) => {
                                    edgeless_api_core::invocation::EventData::CallRet(val.as_bytes())
                                }
                                edgeless_api_core::invocation::EventData::CallNoRet => edgeless_api_core::invocation::EventData::CallNoRet,
                                edgeless_api_core::invocation::EventData::Err => edgeless_api_core::invocation::EventData::Err,
                            },
                        };

                        let ((data, endpoint), _tail) =
                            edgeless_api_core::coap_mapping::COAPEncoder::encode_invocation_event(peer, new_event, token, &mut app_buf_tx[..]);
                        token = match token {
                            u8::MAX => 0,
                            _ => token + 1,
                        };
                        sock.send_to(&data, *endpoint).await.unwrap();
                        break;
                    }
                }
            }
        }
    }
}
