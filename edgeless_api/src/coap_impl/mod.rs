#[async_trait::async_trait]
pub trait InvocationAPI {
    async fn handle(&mut self, event: edgeless_api_core::invocation::Event<&[u8]>)
        -> Result<edgeless_api_core::invocation::LinkProcessingResult, ()>;
}

#[async_trait::async_trait]
pub trait ResourceConfigurationAPI {
    async fn start(
        &mut self,
        instance_specification: edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification,
    ) -> Result<edgeless_api_core::instance_id::InstanceId, ()>;
    async fn stop(&mut self, resource_id: edgeless_api_core::instance_id::InstanceId) -> Result<(), ()>;
}

pub struct CoapInvocationServer {
    sock: tokio::net::UdpSocket,
    root_api: Box<dyn crate::invocation::InvocationAPI>,
}

pub struct CoapClient {
    sock: tokio::net::UdpSocket,
    endpoint: std::net::SocketAddrV4,
    next_token: u8,
}

impl CoapClient {
    pub async fn new(peer: std::net::SocketAddrV4) -> Self {
        let sock = tokio::net::UdpSocket::bind("0.0.0.0:0").await.unwrap();

        CoapClient {
            sock: sock,
            endpoint: peer,
            next_token: 0,
        }
    }
}

#[async_trait::async_trait]
impl crate::invocation::InvocationAPI for CoapClient {
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

        let mut buffer = vec![0 as u8; 2000];

        let token = self.next_token;
        self.next_token = if self.next_token == u8::MAX { 0 } else { self.next_token + 1 };

        let ((packet, addr), _tail) =
            edgeless_api_core::coap_mapping::COAPEncoder::encode_invocation_event(self.endpoint, encoded_event, token, &mut buffer[..]);
        self.sock.send_to(&packet, addr).await.unwrap();
        Ok(crate::invocation::LinkProcessingResult::FINAL)
    }
}

#[async_trait::async_trait]
impl crate::resource_configuration::ResourceConfigurationAPI for CoapClient {
    async fn start(
        &mut self,
        instance_specification: crate::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<crate::common::StartComponentResponse> {
        let mut outputs: [Option<(&str, edgeless_api_core::instance_id::InstanceId)>; 16] = [None; 16];
        let mut outputs_i: usize = 0;
        let mut configuration: [Option<(&str, &str)>; 16] = [None; 16];
        let mut configuration_i: usize = 0;

        for (key, val) in &instance_specification.output_mapping {
            outputs[outputs_i] = Some((key, val.clone()));
            outputs_i = outputs_i + 1;
        }

        for (key, val) in &instance_specification.configuration {
            configuration[configuration_i] = Some((key, val));
            configuration_i = configuration_i + 1;
        }

        let encoded_resource_spec = edgeless_api_core::resource_configuration::EncodedResourceInstanceSpecification {
            provider_id: &instance_specification.provider_id,
            output_mapping: outputs,
            configuration: configuration,
        };

        let mut buffer = vec![0 as u8; 5000];

        let token = self.next_token;
        self.next_token = self.next_token + 1;

        loop {
            let ((packet, addr), _tail) = edgeless_api_core::coap_mapping::COAPEncoder::encode_start_resource(
                self.endpoint,
                encoded_resource_spec.clone(),
                token,
                &mut buffer[..],
            );
            self.sock.send_to(&packet, addr).await.unwrap();

            let (size, sender) = self.sock.recv_from(&mut buffer).await.unwrap();
            if sender != std::net::SocketAddr::V4(self.endpoint) {
                continue;
            }
            let (res, response_token) = edgeless_api_core::coap_mapping::CoapDecoder::decode(&buffer[..size]).unwrap();
            match res {
                edgeless_api_core::coap_mapping::CoapMessage::Response(response_data, ok) => {
                    if response_token == token {
                        match ok {
                            true => {
                                return Ok(crate::common::StartComponentResponse::InstanceId(
                                    edgeless_api_core::coap_mapping::CoapDecoder::decode_instance_id(response_data).unwrap(),
                                ));
                            }
                            false => {
                                return Ok(crate::common::StartComponentResponse::ResponseError(crate::common::ResponseError {
                                    summary: minicbor::decode::<&str>(response_data).unwrap().to_string(),
                                    detail: None,
                                }));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    async fn stop(&mut self, resource_id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        let mut buffer = vec![0 as u8; 5000];

        let token = self.next_token;
        self.next_token = self.next_token + 1;
        loop {
            let ((packet, addr), _tail) =
                edgeless_api_core::coap_mapping::COAPEncoder::encode_stop_resource(self.endpoint, resource_id, token, &mut buffer[..]);
            self.sock.send_to(&packet, addr).await.unwrap();

            let (size, sender) = self.sock.recv_from(&mut buffer).await.unwrap();
            if sender != std::net::SocketAddr::V4(self.endpoint) {
                continue;
            }
            let (res, response_token) = edgeless_api_core::coap_mapping::CoapDecoder::decode(&buffer[..size]).unwrap();
            match res {
                edgeless_api_core::coap_mapping::CoapMessage::Response(response_data, ok) => {
                    if response_token == token {
                        match ok {
                            true => {
                                return Ok(());
                            }
                            false => return Err(anyhow::anyhow!(core::str::from_utf8(response_data).unwrap().to_string())),
                        }
                    }
                }
                _ => {}
            }
        }
    }
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
                        log::info!("Unhandled Message");
                    }
                }
            }
        })
    }
}
