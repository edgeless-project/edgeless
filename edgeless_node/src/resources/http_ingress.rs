// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use edgeless_api::function_instance::InstanceId;
use http_body_util::BodyExt;
use std::str::FromStr;

struct ResourceDesc {
    host: String,
    allow: std::collections::HashSet<edgeless_http::EdgelessHTTPMethod>,
}

struct IngressState {
    interests: Vec<HTTPIngressInterest>,
    active_resources: std::collections::HashMap<InstanceId, ResourceDesc>,
    dataplane: edgeless_dataplane::handle::DataplaneHandle,
}

#[derive(Clone)]
struct IngressService {
    listen_addr: String,
    interests: std::sync::Arc<tokio::sync::Mutex<IngressState>>,
}

impl hyper::service::Service<hyper::Request<hyper::body::Incoming>> for IngressService {
    type Response = hyper::Response<http_body_util::Full<hyper::body::Bytes>>;

    type Error = anyhow::Error;

    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: hyper::Request<hyper::body::Incoming>) -> Self::Future {
        let cloned = self.interests.clone();
        let cloned_addr = self.listen_addr.clone();
        Box::pin(async move {
            let mut lck = cloned.lock().await;

            let (parts, body) = req.into_parts();

            let host = match parts.headers.get(hyper::header::HOST) {
                Some(val) => val.to_str()?,
                None => &cloned_addr,
            };
            let method = edgeless_http::hyper_method_to_edgeless(&parts.method)?;
            let data = body.collect().await?.to_bytes();

            if let Some((host, target, target_port)) = lck.interests.iter().find_map(|intr| {
                if host == intr.host && intr.allow.contains(&method) {
                    Some((intr.host.clone(), intr.target.clone(), intr.target_port.clone()))
                } else {
                    None
                }
            }) {
                let msg = edgeless_http::EdgelessHTTPRequest {
                    host: host.to_string(),
                    protocol: edgeless_http::EdgelessHTTPProtocol::Unknown,
                    method: method.clone(),
                    path: parts.uri.to_string(),
                    body: Some(Vec::from(data)),
                    headers: parts
                        .headers
                        .iter()
                        .filter_map(|(k, v)| match v.to_str() {
                            Ok(header_value) => Some((k.to_string(), header_value.to_string())),
                            Err(_) => {
                                log::warn!("Bad Header Value.");
                                None
                            }
                        })
                        .collect(),
                };
                let serialized_msg = serde_json::to_string(&msg)?;
                let res = lck.dataplane.call(target.clone(), target_port, serialized_msg).await;
                match res {
                    edgeless_dataplane::core::CallRet::Reply(data) => {
                        let processor_response: edgeless_http::EdgelessHTTPResponse = serde_json::from_str(&data)?;
                        let mut response_builder = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
                            processor_response.body.unwrap_or(vec![]),
                        )));
                        *response_builder.status_mut() = hyper::StatusCode::from_u16(processor_response.status)?;
                        {
                            let headers = response_builder.headers_mut();
                            for (header_key, header_val) in processor_response.headers {
                                if let (Ok(key), Ok(value)) = (
                                    hyper::header::HeaderName::from_bytes(header_key.as_bytes()),
                                    hyper::header::HeaderValue::from_str(&header_val),
                                ) {
                                    headers.append(key, value);
                                }
                            }
                        }
                        return Ok(response_builder);
                    }
                    _ => {}
                }
            }

            let mut not_found = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from("Not Found")));
            *not_found.status_mut() = hyper::StatusCode::NOT_FOUND;
            Ok(not_found)
        })
    }
}

pub async fn ingress_task(
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    ingress_id: edgeless_api::function_instance::InstanceId,
    ingress_url: String,
) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>> {
    let mut provider = dataplane_provider;
    let (_, host, port) = edgeless_api::util::parse_http_host(&ingress_url).unwrap();
    let addr = std::net::SocketAddr::from((std::net::IpAddr::from_str(&host).unwrap(), port));

    let dataplane = provider.get_handle_for(ingress_id.clone()).await;

    let ingress_state = std::sync::Arc::new(tokio::sync::Mutex::new(IngressState {
        interests: Vec::<HTTPIngressInterest>::new(),
        active_resources: std::collections::HashMap::new(),
        dataplane,
    }));

    let cloned_interests = ingress_state.clone();

    let _web_task: tokio::task::JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await?;
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(val) => val,
                Err(_) => {
                    log::error!("Accept Error");
                    continue;
                }
            };
            let io = hyper_util::rt::TokioIo::new(stream);
            let cloned_interests = cloned_interests.clone();
            let cloned_host = host.clone();
            let cloned_port = port.clone();
            tokio::task::spawn(async move {
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .serve_connection(
                        io,
                        IngressService {
                            interests: cloned_interests,
                            listen_addr: format!("{}:{}", cloned_host, cloned_port).to_string(),
                        },
                    )
                    .await
                {
                    println!("Error serving connection: {:?}", err);
                }
            });
        }
    });

    Box::new(IngressResource {
        own_node_id: ingress_id.node_id.clone(),
        configuration_state: ingress_state,
    })
}

#[derive(Clone)]
struct IngressResource {
    own_node_id: uuid::Uuid,
    configuration_state: std::sync::Arc<tokio::sync::Mutex<IngressState>>,
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for IngressResource {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        let mut lck = self.configuration_state.lock().await;
        if let (Some(host), Some(methods)) = (
            instance_specification.configuration.get("host"),
            instance_specification.configuration.get("methods"),
        ) {
            // Assign a new component identifier to the newly-created  resource.
            let resource_id = edgeless_api::function_instance::InstanceId::new(self.own_node_id.clone());
            lck.active_resources.insert(
                resource_id.clone(),
                ResourceDesc {
                    host: host.clone(),
                    allow: methods
                        .split(",")
                        .filter_map(|str_method| match edgeless_http::string_method_to_edgeless(str_method) {
                            Ok(val) => Some(val),
                            Err(_) => {
                                log::warn!("Bad HTTP Method");
                                None
                            }
                        })
                        .collect(),
                },
            );
            Ok(edgeless_api::common::StartComponentResponse::InstanceId(resource_id))
        } else {
            Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Error when creating a resource".to_string(),
                    detail: Some("Missing Resource Configuration".to_string()),
                },
            ))
        }
    }
    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        let mut lck = self.configuration_state.lock().await;
        lck.interests.retain(|item| item.resource_id != resource_id);
        Ok(())
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        log::info!("{:?}", update.output_mapping);
        let target = match update
            .output_mapping
            .get(&edgeless_api::function_instance::PortId("new_request".to_string()))
        {
            Some(val) => val.clone(),
            None => {
                return Err(anyhow::anyhow!("Missing mapping of channel: new_request"));
            }
        };
        let mut lck = self.configuration_state.lock().await;
        let (host, allow) = match lck.active_resources.get(&update.function_id) {
            Some(val) => (val.host.clone(), val.allow.clone()),
            None => {
                return Err(anyhow::anyhow!("Patching a non-existing resource: {}", update.function_id));
            }
        };

        if let edgeless_api::common::Output::Single(target, port_id) = target {
            lck.interests.push(HTTPIngressInterest {
                resource_id: update.function_id.clone(),
                host,
                allow,
                target,
                target_port: port_id,
            });

            Ok(())
        } else {
            return Err(anyhow::anyhow!("Unsupported Output Type"));
        }
    }
}

struct HTTPIngressInterest {
    resource_id: edgeless_api::function_instance::InstanceId,
    host: String,
    allow: std::collections::HashSet<edgeless_http::EdgelessHTTPMethod>,
    target: edgeless_api::function_instance::InstanceId,
    target_port: edgeless_api::function_instance::PortId,
}
