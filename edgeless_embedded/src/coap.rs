// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT
use crate::invocation::InvocationAPI;
use crate::resource_configuration::ResourceConfigurationAPI;

#[embassy_executor::task]
pub async fn coap_task(
    mut sock: embassy_net::udp::UdpSocket<'static>,
    out_reader: embassy_sync::channel::Receiver<
        'static,
        embassy_sync::blocking_mutex::raw::NoopRawMutex,
        edgeless_api_core::invocation::Event<heapless::String<1500>>,
        2,
    >,
    agent: crate::agent::EmbeddedAgent,
) {
    let mut agent = agent;

    let mut app_buf = [0 as u8; 5000];
    let mut app_buf_tx = [0 as u8; 5000];

    let mut last_tokens = heapless::LinearMap::<
        smoltcp::wire::IpEndpoint,
        (
            u8,
            Option<Result<edgeless_api_core::instance_id::InstanceId, edgeless_api_core::common::ErrorResponse>>,
        ),
        4,
    >::new();

    let mut token = 0 as u8;

    sock.bind(7050).unwrap();

    loop {
        let res = embassy_futures::select::select(sock.recv_from(&mut app_buf), out_reader.receive()).await;

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
                        let key_entry = last_tokens.get_mut(&sender);
                        match key_entry {
                            None => {
                                agent.handle(invocation).await.unwrap();
                                if let Err(_) = last_tokens.insert(sender.clone(), (token, None)) {
                                    log::info!("Could not store token, duplicate delivery is possible!");
                                }
                            }
                            // While we don't send back a response, we still need to block duplicate delivery.
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

                        let ret = match key_entry {
                            None => {
                                let response = agent.start(start_spec.clone()).await;
                                if let Err(_) = last_tokens.insert(sender.clone(), (token, Some(response.clone()))) {
                                    log::info!("Could not store token, duplicate delivery is possible!");
                                }
                                Some(response)
                            }
                            Some((stored_token, stored_response)) => {
                                if &*stored_token < &token || token == 0 {
                                    let id = agent.start(start_spec).await;
                                    *stored_token = token;
                                    *stored_response = Some(id.clone());

                                    Some(id)
                                } else {
                                    if *stored_token == token {
                                        stored_response.clone()
                                    } else {
                                        None
                                    }
                                }
                            }
                        };

                        if let Some(ret) = ret {
                            let is_ok = ret.is_ok();

                            let (encoded, tail) = match ret {
                                Ok(id) => edgeless_api_core::coap_mapping::COAPEncoder::encode_instance_id(id, &mut app_buf_tx[..]),
                                Err(err) => edgeless_api_core::coap_mapping::COAPEncoder::encode_error_response(err, &mut app_buf_tx[..]),
                            };

                            let ((data, sender), _tail) =
                                edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, encoded, token, &mut tail[..], is_ok);
                            if let Err(err) = sock.send_to(data, sender).await {
                                log::error!("UDP/COAP Send Error: {:?}", err);
                            }
                        }
                    }
                    edgeless_api_core::coap_mapping::CoapMessage::ResourceStop(stop_instance_id) => {
                        let key_entry = last_tokens.get_mut(&sender);

                        let ret = match key_entry {
                            None => {
                                let res = agent.stop(stop_instance_id).await;
                                if let Err(_) = last_tokens.insert(sender.clone(), (token, None)) {
                                    log::info!("Could not store token, duplicate delivery is possible!");
                                }
                                Some(res)
                            }
                            Some((stored_token, _stored_response)) => {
                                if &*stored_token < &token || token == 0 {
                                    *stored_token = token;
                                    Some(agent.stop(stop_instance_id).await)
                                } else {
                                    None
                                }
                            }
                        };

                        if let Some(ret) = ret {
                            let ((data, sender), _tail) = match ret {
                                Ok(_) => edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &[], token, &mut app_buf_tx[..], true),
                                Err(err) => {
                                    let (data, tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_error_response(err, &mut app_buf_tx[..]);
                                    edgeless_api_core::coap_mapping::COAPEncoder::encode_response(sender, &data, token, &mut tail[..], false)
                                }
                            };
                            if let Err(err) = sock.send_to(data, sender).await {
                                log::error!("UDP/COAP Send Error: {:?}", err);
                            }
                        }
                    }
                    _ => {}
                }
            }
            // Internal Message that needs to be sent out.
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
                        if let Err(err) = sock.send_to(&data, *endpoint).await {
                            log::error!("UDP/COAP Send Error: {:?}", err);
                        }
                        break;
                    }
                }
            }
        }
    }
}
