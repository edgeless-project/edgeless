// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_api::{function_instance::InstanceId, resource_configuration::ResourceConfigurationAPI};
use edgeless_dataplane::handle::DataplaneProvider;
use serde::Deserialize;
use serde_json::Error;
use std::{collections::HashMap, process, str::from_utf8, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

// imports the generated proto file for dda
// TODO: maybe generate the rust bindings for all the proto files and put them
// in here instead of dynamic generation each time? Check if it makes sense to use protos for functions also.
pub mod dda_com {
    // TODO: Check if this can be done in the edgeless function itself..
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
                //target_id: [].to_vec(),
                output_mapping: HashMap::new(),
            })),
        }
    }
}

struct DDAResourceProviderInner {
    resource_provider_id: InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    dda_resource: DDAResource,
    //target_id: Vec<edgeless_api::function_instance::InstanceId>,
    output_mapping: std::collections::HashMap<String, InstanceId>,
}

pub struct DDAResource {
    // dda_client: dda_com::com_service_client::ComServiceClient<Channel>,
}

#[derive(Debug, Deserialize)]
struct DDAComSubscription {
    ddatopic: String,
    ddapattern: String,
    cast_mapping: String,
}

#[derive(Debug, Deserialize)]
struct DDAComPublication {
    pubid: String,
    ddatopic: String,
}
#[derive(Debug, Deserialize)]
struct DataplanePubMessage {
    pubid: String,
    pattern: String,
    params: String,
}

impl DDAResource {
    async fn new(
        dda_resource_provider: DDAResourceProvider,
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        dda_url: String,
        dda_com_subscription_mapping: String,
        dda_com_publication_mapping: String,
    ) -> Self {
        let dcs: Result<Vec<DDAComSubscription>, Error> = serde_json::from_str(&dda_com_subscription_mapping);

        let dda_sub_array = match dcs {
            Ok(dda_array) => dda_array,
            Err(err) => {
                log::error!("Error parsing input dda_com_subscription_mapping JSON: {}", err);
                panic!("Error parsing input dda_com_subscription_mapping JSON: {}", err);
                //eprintln!("Error parsing input dda_com_subscription_mapping JSON: {}", err);
                //ToDo: After discussion clarify if process exit is the right way to react..
                //process::exit(1);
            }
        };

        let dcp: Result<Vec<DDAComPublication>, Error> = serde_json::from_str(&dda_com_publication_mapping);

        let dda_pub_array = match dcp {
            Ok(dda_array) => dda_array,
            Err(err) => {
                log::error!("Error parsing input dda_com_publication_mapping JSON: {}", err);
                panic!("Error parsing input dda_com_publication_mapping JSON: {}", err);
                //eprintln!("Error parsing input dda_com_publication_mapping JSON: {}", err);
                //process::exit(1);
            }
        };

        // connect the DDAResource gRPC client to the DDA sidecar server
        let mut dda_client = match dda_com::com_service_client::ComServiceClient::connect(dda_url.clone()).await {
            Ok(client) => client,
            Err(err) => {
                log::error!("Failed to connect to the DDA sidecar: {}", err);
                panic!("Failed to connect to the DDA sidecar: {}", err);
            }
        };

        //TODO: check if we act really as singleton
        log::info!("DDA singleton resource created, connected to the DDA sidecar at url={}", dda_url);

        // subscribe to configured dda topics
        for dda_sub in dda_sub_array {
            let mut dda_subscription_filter = dda_com::SubscriptionFilter::default();
            dda_subscription_filter.r#type = dda_sub.ddatopic.clone();
            if dda_sub.ddapattern == "event" {
                let mut dda_temp_subcription_stream = match dda_client.subscribe_event(dda_subscription_filter).await {
                    Ok(dda_resp) => {
                        log::info!("configured dda subscription successful {:?}", dda_sub.ddatopic);
                        dda_resp.into_inner()
                    }
                    Err(err) => {
                        log::error!("configured dda subscription failed {:?} - {}", dda_sub.ddatopic, err);
                        panic!("configureddda subscription failed {:?} - {}", dda_sub.ddatopic, err);
                    }
                };
                let mut dataplane_handle = dataplane_handle.clone();
                let dda_resource_provider = dda_resource_provider.clone();
                let _sub_task = tokio::spawn(async move {
                    loop {
                        match dda_temp_subcription_stream.message().await {
                            Ok(evt) => {
                                //TODO: In future add full event data e.g. also id
                                let evt_d = evt.unwrap().data;
                                match from_utf8(&evt_d) {
                                    Ok(str) => {
                                        // log::info!("Data from subscription {}", str);
                                        let inner = dda_resource_provider.inner.lock().await;

                                        //TODO: In future, this should be iterated upon since multiple outputs might be mapped
                                        if let Some(target_id) = inner.output_mapping.get(&dda_sub.cast_mapping.to_string()) {
                                            log::info!("target id for data {} from subscription is {}", str, target_id);
                                            dataplane_handle.send(target_id.clone(), str.to_string()).await;
                                        } else {
                                            log::info!("target id unknwon for data {} from subscription", str);
                                        }
                                    }
                                    Err(_) => {
                                        log::error!("subscription event parser error");
                                    }
                                };
                            }
                            Err(_) => {
                                log::error!("subscription event parser error");
                            }
                        };
                    }
                });
            } else {
                log::info!(
                    "configured dda subscription {:?} failed as pattern {:?} not yet implemented!",
                    dda_sub.ddatopic,
                    dda_sub.ddapattern
                );
            }
        }

        // Spawn asynchrounous task to dispatch edgeless dataplane events
        let _dda_task = tokio::spawn(async move {
            loop {
                let mut dataplane_handle = dataplane_handle.clone();
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

                let dpmd: Result<DataplanePubMessage, Error> = serde_json::from_str(&message_data);

                let msg_obj = match dpmd {
                    Ok(msg) => msg,
                    Err(err) => {
                        log::error!("Error parsing input dataplane json message: {}", err);
                        return; // Add return statement here
                    }
                };

                if let Some(dda_pub_task) = dda_pub_array.iter().find(|&p| p.pubid == msg_obj.pubid) {
                    log::info!(
                        "Dataplane message is {} and corresponding DDA pubid is {}",
                        message_data,
                        dda_pub_task.pubid
                    );

                    if msg_obj.pattern == "action" {
                        let mut request = dda_com::Action::default();
                        log::info!("DDA action topic name {} is being called", dda_pub_task.ddatopic);
                        request.r#type = dda_pub_task.ddatopic.to_string();
                        request.id = Uuid::new_v4().to_string();
                        request.source = "edgeless_dda_resource".to_string();
                        request.params = msg_obj.params.into_bytes();
                        let stream = dda_client.publish_action(request).await;
                        match stream {
                            Ok(responses) => {
                                log::info!("DDA action on topic {} successfully executed", dda_pub_task.ddatopic);
                                let response = responses.into_inner().message().await;
                                match response {
                                    Ok(_response) => {
                                        // we need a reply in case of a call from the dataplane
                                        if need_reply {
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
                    } else {
                        log::info!(
                            "failed to publish {} pattern on DDA - not yet implemented! only ACTION implemented",
                            msg_obj.pattern
                        );
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
            if let Some(dda_com_publication_mapping) = instance_specification.configuration.get("dda_com_publication_mapping") {
                if let Some(dda_com_subscription_mapping) = instance_specification.configuration.get("dda_com_subscription_mapping") {
                    // log::info!("dda_com_subscription_mapping is provided {}", dda_com_subscription_mapping);
                    let mut lck = self.inner.lock().await;
                    let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
                    let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id.clone()).await;

                    // wrap the grpc client into a nice DDAResource object
                    let dda_res = DDAResource::new(
                        self.clone(),
                        dataplane_handle,
                        dda_url.clone(),
                        dda_com_subscription_mapping.clone(),
                        dda_com_publication_mapping.clone(),
                    )
                    .await;
                    lck.dda_resource = dda_res;

                    // we always return the fixed singleton id
                    Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
                } else {
                    Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: "Invalid resource configuration".to_string(),
                            detail: Some("dda_topic_name not found in configuration".to_string()),
                        },
                    ))
                }
            } else {
                Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                    edgeless_api::common::ResponseError {
                        summary: "Invalid resource configuration".to_string(),
                        detail: Some("dda_task_action not found in configuration".to_string()),
                    },
                ))
            }
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
        // TODO: check if task clean up is needed
        // let mut lck = self.inner.lock().await;
        // lck.dda_resource.dda_client
        Ok(())
    }

    //always gets called after instantiation
    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        let mut lck = self.inner.lock().await;
        lck.output_mapping = update.output_mapping.clone();
        Ok(())
    }
}
