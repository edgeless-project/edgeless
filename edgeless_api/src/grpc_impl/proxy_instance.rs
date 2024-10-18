#[derive(Clone)]
pub struct ProxyInstanceClient {
    client: crate::grpc_impl::api::proxy_instance_client::ProxyInstanceClient<tonic::transport::Channel>,
}

#[async_trait::async_trait]
impl crate::proxy_instance::ProxyInstanceAPI for ProxyInstanceClient {
    async fn start(&mut self, request: crate::proxy_instance::ProxySpec) -> anyhow::Result<()> {
        match self.client.start(tonic::Request::new(request.into())).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Request Failed: {}", err)),
        }
    }
    async fn stop(&mut self, id: crate::function_instance::InstanceId) -> anyhow::Result<()> {
        match self
            .client
            .stop(tonic::Request::new(super::common::CommonConverters::serialize_instance_id(&id)))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Request Failed: {}", err)),
        }
    }
    async fn patch(&mut self, update: crate::proxy_instance::ProxySpec) -> anyhow::Result<()> {
        match self.client.patch(tonic::Request::new(update.into())).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Request Failed: {}", err)),
        }
    }
}

impl ProxyInstanceClient {
    pub async fn new(server_addr: &str, retry_interval: Option<u64>) -> anyhow::Result<Self> {
        loop {
            match crate::grpc_impl::api::proxy_instance_client::ProxyInstanceClient::connect(server_addr.to_string()).await {
                Ok(client) => {
                    let client = client.max_decoding_message_size(usize::MAX);
                    return Ok(Self { client });
                }
                Err(err) => match retry_interval {
                    Some(val) => tokio::time::sleep(tokio::time::Duration::from_secs(val)).await,
                    None => {
                        return Err(anyhow::anyhow!("Error when connecting to {}: {}", server_addr, err));
                    }
                },
            }
        }
    }
}

pub struct ProxyInstanceServerHandler {
    pub root_api: tokio::sync::Mutex<Box<dyn crate::proxy_instance::ProxyInstanceAPI>>,
}

#[async_trait::async_trait]
impl crate::grpc_impl::api::proxy_instance_server::ProxyInstance for ProxyInstanceServerHandler {
    async fn start(&self, req: tonic::Request<crate::grpc_impl::api::ProxyInstanceSpec>) -> tonic::Result<tonic::Response<()>> {
        let inner_req = req.into_inner();

        let parsed = match crate::proxy_instance::ProxySpec::try_from(inner_req) {
            Ok(req) => req,
            Err(_) => {
                return tonic::Result::Err(tonic::Status::invalid_argument("bad proxy spec"));
            }
        };

        match self.root_api.lock().await.start(parsed).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(e) => Err(tonic::Status::internal(format!("could not start proxy {}", e))),
        }
    }

    async fn stop(&self, req: tonic::Request<crate::grpc_impl::api::InstanceId>) -> tonic::Result<tonic::Response<()>> {
        let inner_req = req.into_inner();

        let parsed = match super::common::CommonConverters::parse_instance_id(&inner_req) {
            Ok(id) => id,
            Err(_) => {
                return tonic::Result::Err(tonic::Status::invalid_argument("bad proxy spec"));
            }
        };

        match self.root_api.lock().await.stop(parsed).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(e) => Err(tonic::Status::internal(format!("could not stop proxy {}", e))),
        }
    }

    async fn patch(&self, req: tonic::Request<crate::grpc_impl::api::ProxyInstanceSpec>) -> tonic::Result<tonic::Response<()>> {
        let inner_req = req.into_inner();

        let parsed = match crate::proxy_instance::ProxySpec::try_from(inner_req) {
            Ok(req) => req,
            Err(_) => {
                return tonic::Result::Err(tonic::Status::invalid_argument("bad proxy spec"));
            }
        };

        match self.root_api.lock().await.patch(parsed).await {
            Ok(_) => Ok(tonic::Response::new(())),
            Err(e) => Err(tonic::Status::internal(format!("could not patch proxy: {}", e))),
        }
    }
}

impl Into<crate::grpc_impl::api::ProxyInstanceSpec> for crate::proxy_instance::ProxySpec {
    fn into(self) -> crate::grpc_impl::api::ProxyInstanceSpec {
        crate::grpc_impl::api::ProxyInstanceSpec {
            id: Some(super::common::CommonConverters::serialize_instance_id(&self.instance_id)),
            internal_inputs: self
                .inner_inputs
                .into_iter()
                .map(|(k, v)| (k.0, super::common::CommonConverters::serialize_input(&v)))
                .collect(),
            internal_outputs: self
                .inner_outputs
                .into_iter()
                .map(|(k, v)| (k.0, super::common::CommonConverters::serialize_output(&v)))
                .collect(),
            external_inputs: self
                .external_inputs
                .into_iter()
                .map(|(k, v)| (k.0, super::common::CommonConverters::serialize_input(&v)))
                .collect(),
            external_outputs: self
                .external_outputs
                .into_iter()
                .map(|(k, v)| (k.0, super::common::CommonConverters::serialize_output(&v)))
                .collect(),
        }
    }
}

impl TryFrom<crate::grpc_impl::api::ProxyInstanceSpec> for crate::proxy_instance::ProxySpec {
    type Error = anyhow::Error;

    fn try_from(serialized: crate::grpc_impl::api::ProxyInstanceSpec) -> Result<Self, Self::Error> {
        Ok(Self {
            instance_id: super::common::CommonConverters::parse_instance_id(&(serialized.id.ok_or(anyhow::anyhow!("Missing Field"))?))?,
            inner_outputs: serialized
                .internal_outputs
                .into_iter()
                .map(|(k, v)| {
                    (
                        crate::function_instance::PortId(k),
                        super::common::CommonConverters::parse_output(&v).unwrap(),
                    )
                })
                .collect(),
            inner_inputs: serialized
                .internal_inputs
                .into_iter()
                .map(|(k, v)| {
                    (
                        crate::function_instance::PortId(k),
                        super::common::CommonConverters::parse_input(&v).unwrap(),
                    )
                })
                .collect(),
            external_outputs: serialized
                .external_outputs
                .into_iter()
                .map(|(k, v)| {
                    (
                        crate::function_instance::PortId(k),
                        super::common::CommonConverters::parse_output(&v).unwrap(),
                    )
                })
                .collect(),
            external_inputs: serialized
                .external_inputs
                .into_iter()
                .map(|(k, v)| {
                    (
                        crate::function_instance::PortId(k),
                        super::common::CommonConverters::parse_input(&v).unwrap(),
                    )
                })
                .collect(),
        })
    }
}
