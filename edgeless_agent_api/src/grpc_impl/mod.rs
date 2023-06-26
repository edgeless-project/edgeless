use crate::AgentAPI;
use crate::FunctionId;

pub mod api {
    tonic::include_proto!("agent_api");
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
    async fn spawn(&mut self, request: crate::SpawnFunctionRequest) -> anyhow::Result<FunctionId> {
        if request.function_id.is_none() {
            return Err(anyhow::anyhow!("FunctionId not set"));
        }
        let res = self
            .client
            .start_function_instance(tonic::Request::new(api::SpawnRequest {
                function_id: Some(api::FunctionId {
                    node_id: request.function_id.clone().unwrap().node_id.to_string(),
                    function_id: request.function_id.clone().unwrap().function_id.to_string(),
                }),
                code: request.code,
            }))
            .await;
        let inner_req = res.unwrap().into_inner();
        Ok(crate::FunctionId {
            node_id: uuid::Uuid::parse_str(&inner_req.node_id).unwrap(),
            function_id: uuid::Uuid::parse_str(&inner_req.function_id).unwrap(),
        })
    }

    async fn stop(&mut self, _id: FunctionId) -> anyhow::Result<()> {
        Ok(())
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
    async fn start_function_instance(&self, request: tonic::Request<api::SpawnRequest>) -> Result<tonic::Response<api::FunctionId>, tonic::Status> {
        let inner_request = request.into_inner();
        let function_id = FunctionId {
            node_id: uuid::Uuid::parse_str(&inner_request.function_id.as_ref().unwrap().node_id).unwrap(),
            function_id: uuid::Uuid::parse_str(&inner_request.function_id.as_ref().unwrap().function_id).unwrap(),
        };
        let fid_2 = function_id.clone();
        let res = self
            .root_api
            .lock()
            .await
            .spawn(crate::SpawnFunctionRequest {
                function_id: Some(function_id),
                code: inner_request.code,
            })
            .await;
        match res {
            Ok(_) => Ok(tonic::Response::new(api::FunctionId {
                node_id: fid_2.node_id.to_string(),
                function_id: fid_2.function_id.to_string(),
            })),
            Err(_) => Err(tonic::Status::internal("Server Error")),
        }
    }
}
