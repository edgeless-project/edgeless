use crate::AgentAPI;
use crate::FunctionId;

pub mod api {
    tonic::include_proto!("agent_api");
}

pub struct AgentAPIConverters {}

impl AgentAPIConverters {
    pub fn parse_function_id(api_id: &api::FunctionId) -> FunctionId {
        FunctionId {
            node_id: uuid::Uuid::parse_str(&api_id.node_id).unwrap(),
            function_id: uuid::Uuid::parse_str(&api_id.function_id).unwrap(),
        }
    }

    pub fn parse_function_class_specification(api_spec: &api::FunctionClassSpecification) -> crate::FunctionClassSpecification {
        crate::FunctionClassSpecification {
            function_class_id: api_spec.function_class_id.clone(),
            function_class_type: api_spec.function_class_type.clone(),
            function_class_version: api_spec.function_class_version.clone(),
            function_class_inlude_code: api_spec.function_class_inline_code().to_vec(),
            output_callback_declarations: api_spec.output_callback_declarations.clone(),
        }
    }

    pub fn parse_api_request(api_request: &api::SpawnFunctionRequest) -> Option<crate::SpawnFunctionRequest> {
        if api_request.code.is_none() {
            return None;
        }
        Some(crate::SpawnFunctionRequest {
            function_id: api_request.function_id.as_ref().and_then(|f| Some(Self::parse_function_id(f))),
            code: Self::parse_function_class_specification(api_request.code.as_ref().unwrap()),
            output_callback_definitions: api_request
                .output_callback_definitions
                .iter()
                .map(|(key, value)| return (key.clone(), Self::parse_function_id(&value)))
                .collect(),
            return_continuation: Self::parse_function_id(&api_request.return_continuation.as_ref().unwrap()),
            annotations: api_request.annotations.clone(),
        })
    }

    pub fn serialize_function_id(function_id: &crate::FunctionId) -> api::FunctionId {
        api::FunctionId {
            node_id: function_id.node_id.to_string(),
            function_id: function_id.function_id.to_string(),
        }
    }

    pub fn serialize_function_class_specification(spec: &crate::FunctionClassSpecification) -> api::FunctionClassSpecification {
        api::FunctionClassSpecification {
            function_class_id: spec.function_class_id.clone(),
            function_class_type: spec.function_class_type.clone(),
            function_class_version: spec.function_class_version.clone(),
            function_class_inline_code: Some(spec.function_class_inlude_code.clone()),
            output_callback_declarations: spec.output_callback_declarations.clone(),
        }
    }

    pub fn serialize_spawn_function_request(req: &crate::SpawnFunctionRequest) -> api::SpawnFunctionRequest {
        api::SpawnFunctionRequest {
            function_id: req.function_id.as_ref().and_then(|fid| Some(Self::serialize_function_id(fid))),
            code: Some(Self::serialize_function_class_specification(&req.code)),
            output_callback_definitions: req
                .output_callback_definitions
                .iter()
                .map(|(key, value)| (key.clone(), Self::serialize_function_id(&value)))
                .collect(),
            return_continuation: Some(Self::serialize_function_id(&req.return_continuation)),
            annotations: req.annotations.clone(),
        }
    }
}

pub struct AgentAPIClient {
    client: api::agent_client::AgentClient<tonic::transport::Channel>,
}

impl AgentAPIClient {
    pub async fn new(server_addr: &str) -> AgentAPIClient {
        let client = api::agent_client::AgentClient::connect(server_addr.to_string()).await.unwrap();
        AgentAPIClient { client }
    }
}

#[async_trait::async_trait]
impl AgentAPI for AgentAPIClient {
    async fn start_function_instance(&mut self, request: crate::SpawnFunctionRequest) -> anyhow::Result<FunctionId> {
        if request.function_id.is_none() {
            return Err(anyhow::anyhow!("FunctionId not set"));
        }
        let serialized_request = AgentAPIConverters::serialize_spawn_function_request(&request);

        let res = self.client.start_function_instance(tonic::Request::new(serialized_request)).await;
        match(res) {
            Ok(function_id) => {
                Ok(AgentAPIConverters::parse_function_id(&function_id.into_inner()))
            },
            Err(_) => {
                Err(anyhow::anyhow!("Start Request Failed"))
            }
        }

    }

    async fn stop_function_instance(&mut self, id: FunctionId) -> anyhow::Result<()> {
        let serialized_id = AgentAPIConverters::serialize_function_id(&id);
        let res = self.client.stop_function_instance(tonic::Request::new(serialized_id)).await;
        match(res) {
            Ok(_) => {
                Ok(())
            },
            Err(_) => {
                Err(anyhow::anyhow!("Stop Request Failed"))
            }
        }
    }
}

pub struct AgentAPIServer {
    root_api: tokio::sync::Mutex<Box<dyn AgentAPI + Send>>,
}

impl AgentAPIServer {
    pub fn run(root_api: Box<dyn AgentAPI + Send>, listen_addr: String) -> futures::future::BoxFuture<'static, ()> {
        let slf = Self {
            root_api: tokio::sync::Mutex::new(root_api),
        };
        Box::pin(async move {
            let slf = slf;
            let addr = listen_addr[7..].parse().unwrap();

            log::info!("Start AgentAPI GRPC Server");

            tonic::transport::Server::builder()
                .add_service(api::agent_server::AgentServer::new(slf))
                .serve(addr)
                .await
                .unwrap();

            log::info!("Stop AgentAPI GRPC Server");
        })
    }
}

#[async_trait::async_trait]
impl api::agent_server::Agent for AgentAPIServer {
    async fn start_function_instance(&self, request: tonic::Request<api::SpawnFunctionRequest>) -> Result<tonic::Response<api::FunctionId>, tonic::Status> {
        let inner_request = request.into_inner();
        let parsed_request = AgentAPIConverters::parse_api_request(&inner_request).unwrap();
        let res = self.root_api.lock().await.start_function_instance(parsed_request).await;
        match res {
            Ok(fid) => Ok(tonic::Response::new(AgentAPIConverters::serialize_function_id(&fid))),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }

    async fn stop_function_instance(&self, request: tonic::Request<api::FunctionId>) -> Result<tonic::Response<()>, tonic::Status> {
        let stop_function_id = AgentAPIConverters::parse_function_id(&request.into_inner());
        let res = self.root_api.lock().await.stop_function_instance(stop_function_id).await;
        match res {
            Ok(_fid) => Ok(tonic::Response::new(())),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }
}
