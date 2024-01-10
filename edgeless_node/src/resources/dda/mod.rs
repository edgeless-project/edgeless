use base64::Engine;
use edgeless_api::resource_configuration::ResourceConfigurationAPI;

pub mod dda_com {
    tonic::include_proto!("dda.com.v1");
}

/// Structs
pub struct DDAResource {
    join_handle: tokio::task::JoinHandle<()>,
}

pub struct DDAResourceProvider {
    /// resources are also identified by an InstanceId, like functions
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    /// all resources on a node share the same dataplane_provider which can be
    /// used to get a handle to the dataplane
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    /// there is always only one DDA resource instance per node
    dda_instance: DDAResource,
}

/// Implementations
impl DDAResource {
    async fn new(dataplane_handle: edgeless_dataplane::handle::DataplaneHandle, dda_url: String) -> anyhow::Result<Self> {
        let mut dataplane_handle = dataplane_handle;

        log::info!("DDA resource created, connecting to sidecar at url={}", dda_url);
        let mut dda_client = match dda_com::com_service_client::ComServiceClient::connect(dda_url).await {
            Ok(client) => client,
            Err(err) => {
                log::error!("Failed to connect to the DDA sidecar: {}", err);
                return Err(anyhow::anyhow!("Failed to connect to the DDA sidecar: {}", err));
            }
        };

        // handle dataplane events for dda resource
        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                } = dataplane_handle.receive_next().await;

                let mut need_reply = false;
                let message_data = match message {
                    edgeless_dataplane::core::Message::Call(data) => {
                        need_reply = true;
                        data
                    }
                    edgeless_dataplane::core::Message::Cast(data) => data,
                    _ => {
                        continue;
                    }
                };

                // forward to the dda sidecar
                match message_data.as_str() {
                    "test" => {
                        let mut request = dda_com::Action::default();
                        request.r#type = "dda_test".to_string();
                        request.id = "0".to_string();
                        request.source = "r2d2".to_string();
                        request.params = base64::encode("ping").to_string().into_bytes();
                        let stream = dda_client.publish_action(request).await;
                        match stream {
                            Ok(responses) => {
                                println!("hello");
                                let response = responses.into_inner().message().await;
                                match response {
                                    Ok(response) => {
                                        println!("test action worked!");

                                        // we need a reply in case of a call from the dataplane
                                        if need_reply {
                                            // dataplane currently only supports
                                            // returning strings - this is not
                                            // suitable in cases where an
                                            // edgeless function would like to
                                            // wait for many responses from the
                                            // dda sidecar - first we would need
                                            // to check if it's even possible to
                                            // stream to wasm; see
                                            // edgless_function/wit/edgefunction.wit
                                            // for the API
                                            dataplane_handle
                                                .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                                .await;
                                        }
                                    }
                                    Err(e) => println!("gRPC error {}", e),
                                }
                            }
                            Err(status) => {
                                println!("gRPC error {}", status);
                            }
                        }
                    }
                    _ => {
                        log::info!("dda resource only supports call / cast to test for now");
                        continue;
                    }
                }
            }
        });

        // returns the DDAResource with a handle to the async tokio task
        Ok(Self { join_handle: handle })
    }
}

impl Drop for DDAResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl DDAResourceProvider {}

#[async_trait::async_trait]
impl ResourceConfigurationAPI for DDAResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse> {
        // dda only needs to have the url of the dda sidecar configured to work
        if let Some(url) = instance_specification.configuration.get("sidecar_url") {
            let new_id = edgeless_api::function_instance::InstanceId::new(self.resource_provider_id.node_id);
            let dataplane_handle = self.dataplane_provider.get_handle_for(new_id.clone()).await;

            // try to create a DDAResource and connect to the sidecar
            match DDAResource::new(dataplane_handle, url.to_string()).await {
                Ok(dda_resource) => {
                    self.dda_instance = dda_resource;
                    return Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id));
                }
                Err(err) => {
                    return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: "Invalid resource configuration".to_string(),
                            detail: Some(format!("Error when creating a DDA resource: {}", err)),
                        },
                    ));
                }
            }
        }
        // TODO: else start a managed sidecar inside of the resource

        Ok(edgeless_api::common::StartComponentResponse::ResponseError(
            edgeless_api::common::ResponseError {
                summary: "Invalid resource configuration".to_string(),
                detail: Some("url is missing from the resource configuration".to_string()),
            },
        ))
    }

    /// On stop the binary is stopped
    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        Ok(())
    }

    /// TODO: what do we need to patch here?
    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // TODO: patch the channels here?
        Ok(())
    }
}
