// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use base64::{engine::general_purpose::STANDARD, Engine as _};
use edgeless_api::{function_instance::InstanceId, resource_configuration::ResourceConfigurationAPI};
use edgeless_dataplane::handle::DataplaneProvider;
use std::{str::from_utf8, sync::Arc};
use tokio::sync::Mutex;

// imports the generated proto file for dda
// TODO: maybe generate the rust bindings for all the proto files and put them
// in here instead of dynamic generation each time?
pub mod dda_com {
    tonic::include_proto!("dda.com.v1");
}

#[derive(Clone)]
pub struct DDAResourceProvider {
    inner: Arc<Mutex<DDAResourceProviderInner>>,
}

impl DDAResourceProvider {
    pub async fn new(dataplane_provider: DataplaneProvider, resource_provider_id: InstanceId) -> Self {
        Self {
            inner: Arc::new(Mutex::new(DDAResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                dda_resource: DDAResource {},
            })),
        }
    }
}

struct DDAResourceProviderInner {
    resource_provider_id: InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    dda_resource: DDAResource,
}

pub struct DDAResource {
    // dda_client: dda_com::com_service_client::ComServiceClient<Channel>,
}

impl DDAResource {
    async fn new(dataplane_handle: edgeless_dataplane::handle::DataplaneHandle, dda_url: String) -> Self {
        let mut dataplane_handle = dataplane_handle;

        // connect the DDAResource gRPC client to the DDA sidecar server
        log::info!("Trying to connect to the DDA sidecar at url={}", dda_url.clone());
        let mut dda_client = match dda_com::com_service_client::ComServiceClient::connect(dda_url.clone()).await {
            Ok(client) => client,
            Err(err) => {
                log::error!("Failed to connect to the DDA sidecar: {}", err);
                panic!("Failed to connect to the DDA sidecar: {}", err);
            }
        };
        log::info!("DDA singleton resource created, connected to the DDA sidecar at url={}", dda_url);

        // SETUP exemplary subscription for temperature to DDA
        // TODO: integrate subscription management for functions

        let mut dda_subscription_filter = dda_com::SubscriptionFilter::default();
        dda_subscription_filter.r#type = "com.edgeless.temperature".to_string();

        // let mut current_temperature = "0";

        let mut dda_temp_subcription_stream = match dda_client.subscribe_event(dda_subscription_filter).await {
            Ok(dda_resp) => {
                log::info!("Subscription successfull");
                dda_resp.into_inner()
            }
            Err(err) => {
                log::error!("Subscription failed {}", err);
                panic!("Subscription failed {}", err);
            }
        };

        let _sub_task = tokio::spawn(async move {
            loop {
                match dda_temp_subcription_stream.message().await {
                    Ok(evt) => {
                        let evt_d = evt.unwrap().data;
                        match from_utf8(&evt_d) {
                            Ok(str) => {
                                //current_temperature = str.clone();
                                log::info!("Temperature from subscription {}", str);
                                // TODO set current temperature current_temperature = from_utf8(str).expect("String for temperature");
                            }
                            Err(_) => {
                                log::error!("Subscription event parser error");
                            }
                        };
                    }
                    Err(_) => {
                        log::error!("Subscription event parser error");
                    }
                };
            }
        });
        // END SETUP SOME SUBSCRIPTIONS TO DDA

        // handle dataplane events for dda resource
        // unused, since we never want to stop the dda sidecar (singleton)
        let _dda_task = tokio::spawn(async move {
            loop {
                log::info!("Waiting for dataplane events");
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                } = dataplane_handle.receive_next().await;

                log::info!("Received a dataplane event");

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
                    "dda_read_temperature" => {
                        log::info!("Read_temperature called");
                        // TODO: listen for x temperature readings and return
                        // them as a vector
                        // subscribe DDA event
                        // onEvent we call back to the function callee
                        dataplane_handle
                            .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply("too_hot".to_string()))
                            .await;
                    }
                    // Example for a DDA request/response pattern using Action
                    "dda_move_arm" => {
                        let mut request = dda_com::Action::default();
                        log::info!("Move_arm called");
                        request.r#type = "com.edgeless.moveRobotArm".to_string();
                        request.id = "0".to_string();
                        request.source = "r2d2".to_string();
                        request.params = STANDARD.encode("boop").to_string().into_bytes();
                        let stream = dda_client.publish_action(request).await;
                        match stream {
                            Ok(responses) => {
                                log::info!("DDA com.edgeless.moveRobotArm action successfully executed");
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
                                    Err(e) => log::error!("gRPC error {}", e),
                                }
                            }
                            Err(status) => {
                                log::error!("gRPC error {}", status);
                            }
                        }
                    }
                    // TODO: add an example for a query service
                    _ => {
                        log::info!("DDA resource only supports call / cast to test for now");
                        continue;
                    }
                }
            }
        });

        Self {}
    }
}

/// Implements the ResourceConfigurationAPI for the DDAResource
#[async_trait::async_trait]
impl ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for DDAResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        // read the sidecar url from the instance specification configuration
        if let Some(dda_url) = instance_specification.configuration.get("dda_url") {
            let mut lck = self.inner.lock().await;
            let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id.clone()).await;

            // wrap the grpc client into a nice DDAResource object
            let dda_res = DDAResource::new(dataplane_handle, dda_url.clone()).await;
            lck.dda_resource = dda_res;

            // we always return the fixed singleton id
            Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
        } else {
            Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Invalid resource configuration".to_string(),
                    detail: Some("dda_url not found in configuration".to_string()),
                },
            ))
        }
    }

    /// nothing is stopped, since we keep the singleton alive for the lifetime
    /// of a node
    async fn stop(&mut self, _resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        // TODO: should close the connection to the grpc server
        // let mut lck = self.inner.lock().await;
        // lck.dda_resource.dda_client
        Ok(())
    }

    /// nothing is patched here, since all calls to the dda are explicit
    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        // TODO: this will be needed in case DDA is used as a type of an ingress service!
        Ok(())
    }
}
