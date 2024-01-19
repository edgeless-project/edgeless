use edgeless_api::resource_configuration::ResourceConfigurationAPI;

// imports the generated proto file for dda
// TODO: maybe generate the rust bindings for all the proto files and put them
// in here instead of dynamic generation each time?
pub mod dda_com {
    tonic::include_proto!("dda.com.v1");
}

/// Represent the connetion to the dda sidecar; no state is kept for now, no
/// join_handle as in other resources because we never disconnect from the sidecar
#[derive(Clone)]
pub struct DDAResource {
    // uuid of the node on which this dda resource is running
    edgeless_node_id: uuid::Uuid,
}

/// Implemented similarly to the http_ingress resource - singleton
pub async fn start_dda_task(
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    dda_id: edgeless_api::function_instance::InstanceId,
    dda_sidecar_url: String,
) -> Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>> {
    // get a new handle for the dda resource
    let mut provider = dataplane_provider;
    let mut dataplane_handle = provider.get_handle_for(dda_id.clone()).await;

    log::info!("Trying to connect to the DDA sidecar at url={}", dda_sidecar_url.clone());
    let mut dda_client = match dda_com::com_service_client::ComServiceClient::connect(dda_sidecar_url.clone()).await {
        Ok(client) => client,
        Err(err) => {
            log::error!("Failed to connect to the DDA sidecar: {}", err);
            panic!("Failed to connect to the DDA sidecar: {}", err);
        }
    };

    log::info!("DDA singleton resource created, connected to the sidecar at url={}", dda_sidecar_url);

    // handle dataplane events for dda resource
    // unused, since we never want to stop the dda sidecar (singleton)
    let _dda_task = tokio::spawn(async move {
        loop {
            let edgeless_dataplane::core::DataplaneEvent {
                source_id,
                channel_id,
                message,
            } = dataplane_handle.receive_next().await;

            log::info!("dda received a dataplane event!");

            let mut need_reply = false;
            let message_data = match message {
                edgeless_dataplane::core::Message::Call(data) => {
                    need_reply = true;
                    // TODO: DDA data is serialized as a string
                    data
                }
                edgeless_dataplane::core::Message::Cast(data) => data,
                _ => {
                    continue;
                }
            };

            // forward to the dda sidecar
            match message_data.as_str() {
                // Example for DDA subscribing to a varying number of events (x)
                "read_temperature" => {
                    // TODO: listen for x temperature readings and return
                    // them as a vector
                    log::info!("read_temperature called");
                }
                // Example for a DDA request/response pattern using Action
                "move_arm" => {
                    let mut request = dda_com::Action::default();
                    log::info!("move_arm called");
                    request.r#type = "com.edgeless.moveRobotArm".to_string();
                    request.id = "0".to_string();
                    request.source = "r2d2".to_string();
                    request.params = base64::encode("boop").to_string().into_bytes();
                    let stream = dda_client.publish_action(request).await;
                    match stream {
                        Ok(responses) => {
                            println!("dda action worked!");
                            let response = responses.into_inner().message().await;
                            match response {
                                Ok(_response) => {
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

                                        // in general: WASM component model
                                        // might not be the right fit at
                                        // all!

                                        // https://jsoverson.medium.com/async-streams-in-webassembly-with-wasmrs-c3604410c999
                                        // - check this out for streaming

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
                // TODO: add an example for a query service
                _ => {
                    log::info!("dda resource only supports call / cast to test for now");
                    continue;
                }
            }
        }
    });

    // returns the DDAResource with a handle to the async tokio task
    Box::new(DDAResource {
        edgeless_node_id: dda_id.node_id.clone(),
    })
}

/// Implements the ResourceConfigurationAPI for the DDAResource
#[async_trait::async_trait]
impl ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for DDAResource {
    async fn start(
        &mut self,
        _instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        // There is a single DDA singleton running that is connected to the
        // sidecar url that was defined in edgeless_node settings, so we don't
        // need to do anything here just yet.
        let resource_id = edgeless_api::function_instance::InstanceId::new(self.edgeless_node_id.clone());

        // we always return the fixed singleton id
        Ok(edgeless_api::common::StartComponentResponse::InstanceId(resource_id))
    }

    /// nothing is stopped, since we keep the singleton alive for the lifetime
    /// of a node
    async fn stop(&mut self, _resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        Ok(())
    }

    /// nothing is patched here, since all calls to the dda are explicit
    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // TODO: this will be needed in case DDA is used as a type of an ingress service!
        Ok(())
    }
}
