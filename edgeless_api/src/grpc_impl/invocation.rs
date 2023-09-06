struct InvocationConverters {}

const TYPE_CALL: i32 = crate::grpc_impl::api::EventType::Call as i32;
const TYPE_CAST: i32 = crate::grpc_impl::api::EventType::Cast as i32;
const TYPE_CALL_RET: i32 = crate::grpc_impl::api::EventType::CallRet as i32;
const TYPE_CALL_NO_RET: i32 = crate::grpc_impl::api::EventType::CallNoRet as i32;

impl InvocationConverters {
    fn parse_api_event(api_event: &crate::grpc_impl::api::Event) -> anyhow::Result<crate::invocation::Event> {
        Ok(crate::invocation::Event {
            target: crate::grpc_impl::function_instance::FunctonInstanceConverters::parse_function_id(&api_event.target.as_ref().unwrap())?,
            source: crate::grpc_impl::function_instance::FunctonInstanceConverters::parse_function_id(&api_event.source.as_ref().unwrap())?,
            stream_id: api_event.stream_id,
            data: Self::parse_api_event_data(&api_event.msg.as_ref().unwrap())?,
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
            target: Some(crate::grpc_impl::function_instance::FunctonInstanceConverters::serialize_function_id(
                &crate_event.target,
            )),
            source: Some(crate::grpc_impl::function_instance::FunctonInstanceConverters::serialize_function_id(
                &crate_event.source,
            )),
            stream_id: crate_event.stream_id,
            msg: Some(Self::encode_crate_event_data(&crate_event.data)),
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
}

impl InvocationAPIClient {
    pub async fn new(server_addr: &str) -> Self {
        loop {
            match crate::grpc_impl::api::function_invocation_client::FunctionInvocationClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Self { client };
                }
                Err(_) => {
                    log::debug!("Waiting for InvocationAPI");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::invocation::InvocationAPI for InvocationAPIClient {
    async fn handle(&mut self, event: crate::invocation::Event) -> anyhow::Result<crate::invocation::LinkProcessingResult> {
        let serialized_event = InvocationConverters::encode_crate_event(&event);
        let res = self.client.handle(tonic::Request::new(serialized_event)).await;
        match res {
            Ok(_) => Ok(crate::invocation::LinkProcessingResult::PROCESSED),
            Err(_) => Err(anyhow::anyhow!("Remote Event Request Failed")),
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
            Ok(_) => Ok(tonic::Response::new(())),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }
}

pub struct InvocationAPIServer {}

impl InvocationAPIServer {
    pub fn run(data_plane: Box<dyn crate::invocation::InvocationAPI>, listen_addr: String) -> futures::future::BoxFuture<'static, ()> {
        let data_plane = data_plane;
        let function_api = crate::grpc_impl::invocation::InvocationAPIServerHandler {
            root_api: tokio::sync::Mutex::new(data_plane),
        };
        Box::pin(async move {
            let function_api = function_api;
            if let Ok((_proto, host, port)) = crate::util::parse_http_host(&listen_addr) {
                if let Ok(host) = format!("{}:{}", host, port).parse() {
                    log::info!("Start InvocationAPI GRPC Server");
                    match tonic::transport::Server::builder()
                        .add_service(
                            crate::grpc_impl::api::function_invocation_server::FunctionInvocationServer::new(function_api)
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
            }

            log::info!("Stop Invocation GRPC Server");
        })
    }
}
