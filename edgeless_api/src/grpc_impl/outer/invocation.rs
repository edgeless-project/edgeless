// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use super::super::common::CommonConverters;
use crate::invocation::LinkProcessingResult;
use std::time::Duration;

struct InvocationConverters {}

const TYPE_CALL: i32 = crate::grpc_impl::api::EventType::Call as i32;
const TYPE_CAST: i32 = crate::grpc_impl::api::EventType::Cast as i32;
const TYPE_CALL_RET: i32 = crate::grpc_impl::api::EventType::CallRet as i32;
const TYPE_CALL_NO_RET: i32 = crate::grpc_impl::api::EventType::CallNoRet as i32;

const RECONNECT_TRIES: i32 = 5;
const RECONNECT_TIMEOUT: u64 = 500;
// time after which we consider the connection to invocation grpc server broken
// and start the reconnection attempts
const INVOCATION_TIMEOUT: u64 = 500;
const INVOCATION_TCP_KEEPALIVE: u64 = 2000;

impl InvocationConverters {
    fn parse_api_event(api_event: &crate::grpc_impl::api::Event) -> anyhow::Result<crate::invocation::Event> {
        Ok(crate::invocation::Event {
            target: CommonConverters::parse_instance_id(api_event.target.as_ref().unwrap())?,
            source: CommonConverters::parse_instance_id(api_event.source.as_ref().unwrap())?,
            stream_id: api_event.stream_id,
            data: Self::parse_api_event_data(api_event.msg.as_ref().unwrap())?,
            created: CommonConverters::parse_event_timestamp(api_event.created.as_ref().unwrap())?,
            metadata: api_event
                .metadata
                .as_ref()
                .ok_or(anyhow::anyhow!("the serialized metadata field is missing"))
                .and_then(|x| x.try_into())?,
        })
    }

    fn parse_api_event_data(api_event_data: &crate::grpc_impl::api::EventData) -> anyhow::Result<crate::invocation::EventData> {
        match api_event_data.event_type {
            TYPE_CALL => Ok(crate::invocation::EventData::Call(api_event_data.payload.to_string())),
            TYPE_CAST => Ok(crate::invocation::EventData::Cast(api_event_data.payload.to_string())),
            TYPE_CALL_RET => Ok(crate::invocation::EventData::CallRet(api_event_data.payload.to_string())),
            TYPE_CALL_NO_RET => Ok(crate::invocation::EventData::CallNoRet),
            _ => Ok(crate::invocation::EventData::Err),
        }
    }

    fn encode_crate_event(crate_event: &crate::invocation::Event) -> crate::grpc_impl::api::Event {
        crate::grpc_impl::api::Event {
            target: Some(CommonConverters::serialize_instance_id(&crate_event.target)),
            source: Some(CommonConverters::serialize_instance_id(&crate_event.source)),
            stream_id: crate_event.stream_id,
            msg: Some(Self::encode_crate_event_data(&crate_event.data)),
            created: Some(CommonConverters::serialize_event_timestamp(&crate_event.created)),
            metadata: Some(crate::grpc_impl::api::EventSerializedMetadata::from(&crate_event.metadata)),
        }
    }

    fn encode_crate_event_data(crate_event: &crate::invocation::EventData) -> crate::grpc_impl::api::EventData {
        let mut payload_buffer = "".to_string();
        let event = match crate_event {
            crate::invocation::EventData::Call(payload) => {
                payload_buffer = payload.to_string();
                crate::grpc_impl::api::EventType::Call
            }
            crate::invocation::EventData::Cast(payload) => {
                payload_buffer = payload.to_string();
                crate::grpc_impl::api::EventType::Cast
            }
            crate::invocation::EventData::CallRet(payload) => {
                payload_buffer = payload.to_string();
                crate::grpc_impl::api::EventType::CallRet
            }
            crate::invocation::EventData::CallNoRet => crate::grpc_impl::api::EventType::CallNoRet,
            crate::invocation::EventData::Err => crate::grpc_impl::api::EventType::Err,
        };
        crate::grpc_impl::api::EventData {
            payload: payload_buffer,
            event_type: event as i32,
        }
    }
}

pub struct InvocationAPIClient {
    client: crate::grpc_impl::api::function_invocation_client::FunctionInvocationClient<tonic::transport::Channel>,
    server_addr: String,
}

impl InvocationAPIClient {
    pub async fn new(server_addr: &str) -> Self {
        loop {
            match crate::grpc_impl::api::function_invocation_client::FunctionInvocationClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Self {
                        client,
                        server_addr: server_addr.to_string(),
                    };
                    // TODO: is the server side retry policy really needed?
                    // let retry_policy = super::common::Attempts(super::common::GRPC_RETRIES);
                    // let retrying_client = tower::retry::Retry::new(retry_policy, client);
                    // return Self {
                    //     client: retrying_client.get_ref().clone(),
                    // };
                }
                Err(e) => {
                    log::warn!("Waiting for InvocationAPI to connect: {e}");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn reconnect(&mut self) -> Result<(), anyhow::Error> {
        let mut retries = RECONNECT_TRIES;
        loop {
            if retries == 0 {
                log::error!("could not reconnect in reasonable time");
                anyhow::bail!("could not reconnect in reasonable time");
            }
            match crate::grpc_impl::api::function_invocation_client::FunctionInvocationClient::connect(self.server_addr.clone()).await {
                Ok(client) => {
                    self.client = client.max_decoding_message_size(usize::MAX);
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("Waiting for InvocationAPI to reconnect: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(RECONNECT_TIMEOUT)).await;
                }
            }
            retries -= 1;
        }
    }
}

#[async_trait::async_trait]
impl crate::invocation::InvocationAPI for InvocationAPIClient {
    // NOTE: LinkProcessingResult is decided by the client, based on the fact if
    // the request was successfull (a successfull request returns an empty
    // response). In the current implementation, we assume that a request has
    // been final if it has been sent to a remote client and a response was
    // received (not like the original idea was).
    // NOTE: this needs to be reevaluated for any real world deployments
    async fn handle(&mut self, event: crate::invocation::Event) -> LinkProcessingResult {
        // // Option 1. true non-blocking, as it returns immediately - in spirit
        // of edgeless fire and forget
        // let serialized_event = InvocationConverters::encode_crate_event(&event);
        // // add a timeout, as this could hang the dataplane indefinitely
        // let mut client = self.client.clone();
        // // best effort, try not to block
        // tokio::spawn(async move {
        //     let _ = client.handle(tonic::Request::new(serialized_event)).await;
        // });
        // // return immediately
        // return LinkProcessingResult::FINAL;

        // Option 2. try until successfull
        loop {
            let serialized_event = InvocationConverters::encode_crate_event(&event);
            let res = tokio::time::timeout(
                Duration::from_millis(INVOCATION_TIMEOUT),
                self.client.handle(tonic::Request::new(serialized_event)),
            )
            .await;
            if let Ok(_) = res {
                return LinkProcessingResult::FINAL;
            } else if let Err(elapsed) = res {
                let res = self.reconnect().await;
                if let Ok(_) = res {
                    log::info!("reconnected successfully, retrying the request");
                    continue;
                } else {
                    log::warn!("reconnect did not work");
                    return LinkProcessingResult::ERROR(elapsed.to_string());
                }
            }
        }
    }
}

pub struct InvocationAPIServerHandler {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::invocation::InvocationAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::function_invocation_server::FunctionInvocation for InvocationAPIServerHandler {
    async fn handle(&self, request: tonic::Request<crate::grpc_impl::api::Event>) -> Result<tonic::Response<()>, tonic::Status> {
        let inner_request = request.into_inner();
        let parsed_request = match InvocationConverters::parse_api_event(&inner_request) {
            Ok(val) => val,
            Err(err) => {
                log::error!("Parse Request Failed: {}", err);
                return Err(tonic::Status::invalid_argument("Bad Request"));
            }
        };

        let res = self.root_api.lock().await.handle(parsed_request).await;
        match res {
            LinkProcessingResult::ERROR(e) => Err(tonic::Status::internal("Server error")),
            _ => Ok(tonic::Response::new(())),
        }
    }
}

pub struct InvocationAPIServer {}

impl InvocationAPIServer {
    pub fn run(data_plane: Box<dyn crate::invocation::InvocationAPI>, invocation_url: String) -> futures::future::BoxFuture<'static, ()> {
        let data_plane = data_plane;
        let function_api = super::invocation::InvocationAPIServerHandler {
            root_api: tokio::sync::Mutex::new(data_plane),
        };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&invocation_url) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start InvocationAPI GRPC Server at {}", invocation_url);
                    match tonic::transport::Server::builder()
                        // TODO: left for future reference
                        // for reusing the same tcp connection for many http requests
                        // .http2_keepalive_interval(Some(std::time::Duration::from_millis(500)))
                        // .http2_keepalive_timeout(Some(std::time::Duration::from_millis(200)))
                        // can help in identifying and closing half-open
                        // connections faster
                        .tcp_keepalive(Some(Duration::from_millis(INVOCATION_TCP_KEEPALIVE)))
                        // if the handling of request takes too long on the
                        // server side, drop this request
                        .layer(tower::timeout::TimeoutLayer::new(std::time::Duration::from_millis(
                            crate::grpc_impl::common::GRPC_SERVICE_TIMEOUT,
                        )))
                        .add_service(
                            crate::grpc_impl::api::function_invocation_server::FunctionInvocationServer::new(function_api)
                                .max_decoding_message_size(usize::MAX),
                        )
                        .serve(host)
                        .await
                    {
                        Ok(_) => {
                            log::debug!("Clean Exit");
                            panic!("clean exit");
                        }
                        Err(e) => {
                            log::error!("GRPC Server Failure {:?}", e);
                            panic!("grpc server failure");
                        }
                    }
                }
            }

            log::info!("Stop Invocation GRPC Server");
        })
    }
}
