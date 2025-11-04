// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use edgeless_api::function_instance::ComponentId;
use http_body_util::BodyExt;
use rand::{seq::SliceRandom, SeedableRng};
use std::str::FromStr;

pub struct HttpIngressResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for HttpIngressResourceSpec {
    fn class_type(&self) -> String {
        String::from("http-ingress")
    }

    fn description(&self) -> String {
        r"Ingest HTTP commands from external web clients".to_string()
    }

    fn outputs(&self) -> Vec<String> {
        vec!["new_request".to_string()]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            (
                String::from("host"),
                String::from("If not empty, requires the external client to specify the given hostname in the HTTP header. Default: hostname not required"),
            ),
            (
                String::from("method"),
                String::from("Comma-separated list of HTTP methods allowed. Default: accept any method"),
            ),
            (
                String::from("wf_id"),
                String::from("Boolean specifying if the external client is required to specify the workflow ID in the URL query (?wf_id=<ID>). One of: true, false. Default: false."),
            ),
            (
                String::from("async"),
                String::from("Boolean specifying if the target on the output channel should be invoked via an asynchronous cast. One of: true, false. Default: use a synchronous call."),
            ),
        ])
    }

    fn version(&self) -> String {
        String::from("2.0")
    }
}

struct ResourceDesc {
    host: Option<String>,
    allow: std::collections::HashSet<edgeless_http::EdgelessHTTPMethod>,
    wf_id: Option<String>,
    async_out: bool,
    target: Option<edgeless_api::function_instance::InstanceId>,
}

struct IngressState {
    active_resources: std::collections::HashMap<ComponentId, ResourceDesc>,
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
        let mut rng = rand::rngs::StdRng::from_entropy();
        Box::pin(async move {
            let mut lck = cloned.lock().await;

            let query = req.uri().query().unwrap_or_default().to_string();
            let (parts, body) = req.into_parts();

            let host = match parts.headers.get(hyper::header::HOST) {
                Some(val) => val.to_str()?,
                None => &cloned_addr,
            };
            let method = edgeless_http::hyper_method_to_edgeless(&parts.method)?;

            let mut wf_id = None;
            for param in query.split("&") {
                if let Some((k, v)) = param.split_once("=") {
                    if k == "wf_id" {
                        wf_id = Some(v.to_string());
                        break;
                    }
                }
            }

            let data = body.collect().await?.to_bytes();

            // Find the set of matching resources for this HTTP request.
            let mut matching = vec![];
            for (_resource_id, desc) in &lck.active_resources {
                if (desc.host.is_none() || desc.host == Some(host.to_string()))
                    && (desc.allow.is_empty() || desc.allow.contains(&method))
                    && (desc.wf_id.is_none() || desc.wf_id == wf_id)
                    && desc.target.is_some()
                {
                    matching.push(desc);
                }
            }

            // Choone one resource at random, if any.
            if let Some(desc) = matching.choose(&mut rng) {
                let target = desc.target.unwrap();

                if desc.async_out {
                    // Invoke the next component via cast().
                    lck.dataplane
                        .send(
                            target,
                            String::from_utf8(data.to_vec())?,
                            &edgeless_api::function_instance::EventMetadata::empty_new_root(),
                        )
                        .await;
                    let mut ok_res = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from("OK")));
                    *ok_res.status_mut() = hyper::StatusCode::OK;
                    return Ok(ok_res);
                } else {
                    // Invoke the next component via call().
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
                    let res = lck
                        .dataplane
                        .call(target, serialized_msg, &edgeless_api::function_instance::EventMetadata::empty_new_root())
                        .await;
                    if let edgeless_dataplane::core::CallRet::Reply(data) = res {
                        let processor_response: edgeless_http::EdgelessHTTPResponse = serde_json::from_str(&data)?;
                        let mut response_builder = hyper::Response::new(http_body_util::Full::new(hyper::body::Bytes::from(
                            processor_response.body.unwrap_or_default(),
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

    let dataplane = provider.get_handle_for(ingress_id).await;

    let ingress_state = std::sync::Arc::new(tokio::sync::Mutex::new(IngressState {
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
            let cloned_port = port;
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
        own_node_id: ingress_id.node_id,
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

        let host = instance_specification.configuration.get("host").cloned();
        let mut allow = std::collections::HashSet::new();
        for method in instance_specification
            .configuration
            .get("method")
            .unwrap_or(&String::default())
            .split(",")
            .filter(|x| !x.is_empty())
        {
            match edgeless_http::string_method_to_edgeless(method) {
                Ok(method) => allow.insert(method),
                Err(err) => {
                    return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: "Error when creating a resource".to_string(),
                            detail: Some(format!("Invalid method '{method}' specified in http-ingress: {err}")),
                        },
                    ))
                }
            };
        }
        let wf_id = if instance_specification
            .configuration
            .get("wf_id")
            .unwrap_or(&String::from("false"))
            .eq_ignore_ascii_case("true")
        {
            Some(instance_specification.workflow_id)
        } else {
            None
        };
        let async_out = instance_specification
            .configuration
            .get("async")
            .unwrap_or(&String::from("false"))
            .eq_ignore_ascii_case("true");

        // Assign a new component identifier to the newly-created  resource.
        log::info!(
            "created a new http-ingress resource: host {:?}, methods allowed {:?}, wf_id {:?}, {}",
            host,
            allow,
            wf_id,
            if async_out { "cast" } else { "call" }
        );
        let resource_id = edgeless_api::function_instance::InstanceId::new(self.own_node_id);
        lck.active_resources.insert(
            resource_id.function_id,
            ResourceDesc {
                host,
                allow,
                wf_id,
                async_out,
                target: None, // will be set by patch()
            },
        );
        Ok(edgeless_api::common::StartComponentResponse::InstanceId(resource_id))
    }
    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        self.configuration_state.lock().await.active_resources.remove(&resource_id.function_id);
        Ok(())
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        let target = update.output_mapping.get("new_request").ok_or(anyhow::anyhow!(
            "Missing channel new_request from http-ingress patch request with PID '{}'",
            update.function_id
        ))?;
        let mut lck = self.configuration_state.lock().await;
        let desc = lck.active_resources.get_mut(&update.function_id).ok_or(anyhow::anyhow!(
            "Trying to patch a non-existing resource with PID '{}'",
            update.function_id
        ))?;
        desc.target = Some(*target);

        Ok(())
    }
}
