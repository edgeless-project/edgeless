// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use dda_state::{ObserveMembershipChangeParams, ObserveStateChangeParams};
use edgeless_api::function_instance::InstanceId;
use edgeless_api::resource_configuration::ResourceConfigurationAPI;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Error;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use uuid::Uuid;

// imports the generated proto file for dda
pub mod dda_com {
    tonic::include_proto!("dda.com.v1");
}

pub mod dda_state {
    tonic::include_proto!("dda.state.v1");
}

pub mod dda_store {
    tonic::include_proto!("dda.store.v1");
}

// through this trait, when the provider is cloned, we still have only a singleton
#[derive(Clone)]
pub struct DDAResourceProvider {
    // inner is a singleton behind arc + mutex
    inner: Arc<Mutex<DDAResourceProviderInner>>,
}

impl DDAResourceProvider {
    pub async fn new(dataplane_provider: edgeless_dataplane::handle::DataplaneProvider, resource_provider_id: InstanceId) -> Self {
        Self {
            inner: Arc::new(Mutex::new(DDAResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                instances: HashMap::<Uuid, DDAResource>::new(),
                // TODO: inner hashmap should be mapped to a vector of
                // InstanceIDs -> in case we decide that multiple functions can
                // listen to the same dda event
                mappings: HashMap::<Uuid, HashMap<String, InstanceId>>::new(),
            })),
        }
    }
}

struct DDAResourceProviderInner {
    resource_provider_id: InstanceId, // resource provider is the node
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    instances: HashMap<Uuid, DDAResource>, // there is a single sidecar per node, but dda can be used for multiple workflows
    // NOTE: limitation: since output mapping is set through the patching
    // request, there is a small window of time during which incoming DDA
    // events would not get correctly sent to functions that might have
    // already been started (race condition). For now we ignore this edge
    // case
    mappings: HashMap<Uuid, HashMap<String, InstanceId>>,
}

pub struct DDAResource {
    // gRPC clients get dropped automatically when the tokio tasks are aborted
    sub_tasks: Vec<tokio::task::JoinHandle<()>>,
}

// used to map incoming dda events to dataplane subscriptions
#[derive(Debug, Deserialize)]
struct DDAComSubscription {
    topic: String,   // also known as type in DDA
    pattern: String, // action / event / query / state / membership
    method: String,  // cast or call as means of passing the dda event
    target: String,  // which function should be invoked (or alias of another resource)
}

// used to map events from functions to dda publications
#[derive(Debug, Deserialize)]
struct DDAComPublication {
    topic: String,   // also known as type in DDA
    pattern: String, // action / event/ query / input
    id: String,      // used to identify a publication mapping
}

impl Drop for DDAResource {
    fn drop(&mut self) {
        // clean up the connections to the sidecar and drop the tokio handles
        for handle in &self.sub_tasks {
            // aborting them also cleans up the grpc clients!
            handle.abort();
        }
    }
}

impl DDAResource {
    async fn new(
        dda_resource_provider: DDAResourceProvider,
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        dda_url: String,
        dda_com_subscription_mapping: String,
        dda_com_publication_mapping: String,
        self_id: Uuid,
    ) -> Self {
        // Parse the configuration of action / event / query bindings to functions
        let dcs: Result<Vec<DDAComSubscription>, Error> = serde_json::from_str(&dda_com_subscription_mapping);
        let dps: Result<Vec<DDAComPublication>, Error> = serde_json::from_str(&dda_com_publication_mapping);

        // they are like dda subscription filters + method (subscribe_x)
        // TODO: (topic, pattern) -> (target, method) max 1 (injective) ->
        // otherwise dda would need to send the same event multiple times? this
        // could also be the semantics
        let dda_sub_array = match dcs {
            Ok(dda_array) => dda_array,
            Err(err) => {
                log::error!("Error parsing input dda_com_subscription_mapping JSON: {}", err);
                panic!("Error parsing input dda_com_subscription_mapping JSON: {}", err);
            }
        };

        // id -> (topic, pattern) (subjektiv) (ids must be unique, otherwise no
        // way of finding the correct)
        // TODO: same discussion as previously
        let dda_pub_array = match dps {
            Ok(dda_array) => dda_array,
            Err(err) => {
                log::error!("Error parsing input dda_com_publication_mapping JSON: {}", err);
                panic!("Error parsing input dda_com_publication_mapping JSON: {}", err);
            }
        };
        let dda_pub_map: HashMap<String, DDAComPublication> = dda_pub_array.into_iter().map(|p| (p.id.clone(), p)).collect();

        // always connect all clients to the sidecar, because the function can
        // use any of the subsystems
        let mut dda_com_client = dda_com::com_service_client::ComServiceClient::connect(dda_url.clone())
            .await
            .expect("dda sidecar: com connection failed");

        let mut dda_state_client = dda_state::state_service_client::StateServiceClient::connect(dda_url.clone())
            .await
            .expect("dda sidecar: state connection failed");

        let mut dda_store_client = dda_store::store_service_client::StoreServiceClient::connect(dda_url.clone())
            .await
            .expect("dda sidecar: store connection failed");

        // subscribe to configured dda topics
        let mut sub_tasks: Vec<tokio::task::JoinHandle<()>> = vec![];
        // constructed here to get access to Self
        let mut dda_resource = Self { sub_tasks: vec![] };

        for dda_sub in dda_sub_array {
            let mut dda_subscription_filter = dda_com::SubscriptionFilter::default();
            // topic is used as the type for filtering
            dda_subscription_filter.r#type = dda_sub.topic.clone();
            let mut dataplane_handle = dataplane_handle.clone();
            // clone the ref to the provider (behind arc+mutex)
            let provider = dda_resource_provider.clone();

            let (sender, receiver) = futures::channel::mpsc::unbounded();
            let mut sender = sender;
            let mut receiver: futures::channel::mpsc::UnboundedReceiver<dda::DDA> = receiver; // cast

            // this task is responsible for passing the received task to
            // functions; split from subscription task for performance and clarity reasons
            let passer_task = tokio::spawn(async move {
                while let Some(event) = receiver.next().await {
                    let encoded = serde_json::to_string(&event).expect("error parsing DataplaneDDA");
                    let mapping = provider.inner.lock().await.mappings.get(&self_id).cloned();
                    let target_id = match mapping {
                        Some(m) => m.get(&dda_sub.target.to_string()).cloned(),
                        _ => None,
                    };
                    match target_id {
                        Some(target_id) => match dda_sub.method.as_str() {
                            "cast" => {
                                dataplane_handle.send(target_id, encoded).await;
                            }
                            "call" => {
                                // NOTE: we don't care about the dataplane
                                // response to this call - the application
                                // developer must ensure that he responds to
                                // actions that he has executed - this
                                // simplifies the logic
                                let _ = dataplane_handle.call(target_id, encoded).await;
                            }
                            _ => {
                                panic!("Unexpected method");
                            }
                        },
                        None => {
                            log::warn!("Unknown target for incoming event that was subscribed {:?}", event);
                            continue;
                        }
                    }
                }
            });

            // start a subscription based on the pattern
            let sub_task = match dda_sub.pattern.as_str() {
                "event" => {
                    let mut dda_com_client = dda_com_client.clone();
                    tokio::spawn(async move {
                        let mut event_stream = match dda_com_client.subscribe_event(dda_subscription_filter).await {
                            Ok(dda_resp) => dda_resp.into_inner(),
                            Err(err) => {
                                panic!("dda subscription failed {:?} - {}", dda_sub.topic, err);
                            }
                        };
                        loop {
                            match event_stream.message().await {
                                Ok(e) => {
                                    let _ = sender.send(dda::DDA::ComSubscribeEvent(e.unwrap().data)).await;
                                }
                                Err(_) => {
                                    log::error!("subscription event parser error");
                                }
                            };
                        }
                    })
                }
                "action" => {
                    let mut dda_com_client = dda_com_client.clone();
                    tokio::spawn(async move {
                        let mut action_stream = match dda_com_client.subscribe_action(dda_subscription_filter).await {
                            Ok(dda_resp) => dda_resp.into_inner(),
                            Err(err) => {
                                panic!("failed");
                            }
                        };
                        loop {
                            match action_stream.message().await {
                                Ok(e) => {
                                    let action_correlated = e.unwrap();
                                    let _ = sender
                                        .send(dda::DDA::ComSubscribeAction(
                                            action_correlated.correlation_id,
                                            action_correlated.action.unwrap().params,
                                        ))
                                        .await;
                                }
                                Err(_) => {
                                    log::error!("subscription action error");
                                }
                            }
                        }
                    })
                }
                "query" => {
                    let mut dda_com_client = dda_com_client.clone();
                    tokio::spawn(async move {
                        let mut query_stream = match dda_com_client.subscribe_query(dda_subscription_filter).await {
                            Ok(dda_resp) => dda_resp.into_inner(),
                            Err(err) => {
                                panic!("failed");
                            }
                        };
                        loop {
                            match query_stream.message().await {
                                Ok(e) => {
                                    let query_correlated = e.unwrap();
                                    let _ = sender
                                        .send(dda::DDA::ComSubscribeQuery(
                                            query_correlated.correlation_id,
                                            query_correlated.query.unwrap().data,
                                        ))
                                        .await;
                                }
                                Err(_) => {
                                    log::error!("subscription action error");
                                }
                            }
                        }
                    })
                }
                "input" => {
                    let mut dda_state_client = dda_state_client.clone();
                    tokio::spawn(async move {
                        let params = ObserveStateChangeParams::default();
                        let mut state_input_stream = match dda_state_client.observe_state_change(params).await {
                            Ok(state_input) => state_input.into_inner(),
                            Err(err) => {
                                panic!("failed");
                            }
                        };
                        loop {
                            match state_input_stream.message().await {
                                Ok(e) => {
                                    let e = e.unwrap();
                                    let op = e.op;
                                    let key = e.key;
                                    let value = e.value;
                                    let event = match op {
                                        1 => dda::DDA::StateSubscribeSet(key, value),
                                        2 => dda::DDA::StateSubscribeDelete(key),
                                        _ => {
                                            panic!("wrong")
                                        }
                                    };
                                    let _ = sender.send(event).await;
                                }
                                Err(_) => {
                                    log::error!("subscription input error");
                                }
                            }
                        }
                    })
                }
                "membership" => {
                    let mut dda_state_client = dda_state_client.clone();
                    tokio::spawn(async move {
                        let params = ObserveMembershipChangeParams::default();
                        let mut membership_change_stream = match dda_state_client.observe_membership_change(params).await {
                            Ok(memb_change) => memb_change.into_inner(),
                            Err(err) => {
                                panic!("failed");
                            }
                        };
                        loop {
                            match membership_change_stream.message().await {
                                Ok(m) => {
                                    let m = m.unwrap();
                                    let _ = sender.send(dda::DDA::StateSubscribeMembershipChange(m.id, m.joined)).await;
                                }
                                Err(_) => {
                                    log::error!("subscription membership error");
                                }
                            }
                        }
                    })
                }
                _ => {
                    log::info!(
                        "configured dda subscription {:?} failed as pattern {:?} not yet implemented!",
                        dda_sub.topic,
                        dda_sub.pattern
                    );
                    continue;
                }
            };
            // persist the task for future cancellation on resource stop
            sub_tasks.push(sub_task);
            sub_tasks.push(passer_task);
        }

        // Spawn asynchrounous task to handle edgeless dataplane events -
        // these are events that are received by the dda resource from other
        // components
        let mut dataplane_handle = dataplane_handle.clone();
        let mut id: u128 = 0;
        let _dda_task = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                } = dataplane_handle.receive_next().await;

                let message: dda::DDA = match message {
                    edgeless_dataplane::core::Message::Call(data) => {
                        // all calls to DDA resource must be Calls with
                        // DataplaneDDA as serialized data
                        serde_json::from_str::<dda::DDA>(&data).expect("wrong incoming dataplane event from a function")
                    }
                    _ => {
                        // disregard anything but Calls
                        continue;
                    }
                };

                match message {
                    dda::DDA::ComPublishEvent(alias, data) => {
                        let p = dda_pub_map.get(&alias).expect("publication not specified");
                        if p.pattern != "event" {
                            log::warn!("wrong publication type");
                            dataplane_handle
                                .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Err)
                                .await;
                            continue;
                        }
                        let topic = p.topic.to_string();
                        let msg_id = id.to_string();
                        id += 1;
                        let source = self_id.clone().to_string();
                        let mut event = dda_com::Event::default();
                        event.source = source;
                        event.id = msg_id;
                        event.r#type = topic;
                        event.data = data;
                        let _ = dda_com_client.publish_event(event).await;
                        // NoReply = success for event
                        let _ = dataplane_handle
                            .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::NoReply)
                            .await;
                    }
                    dda::DDA::ComPublishAction(alias, data) => {
                        let p = dda_pub_map.get(&alias).expect("publication not specified");
                        if p.pattern != "action" {
                            log::warn!("wrong publication type");
                            dataplane_handle
                                .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Err)
                                .await;
                            continue;
                        }
                        let topic = p.topic.to_string();
                        let msg_id = id.to_string();
                        id += 1;
                        let source = self_id.clone().to_string();
                        // construct the Action
                        let mut action = dda_com::Action::default();
                        action.source = source;
                        action.id = msg_id;
                        action.r#type = topic;
                        action.params = data;

                        // wait for an action response as specified in the
                        // parameters
                        // match
                        // tokio::time::timeout(Duration::from_secs(1),
                        // dda_com_client.publish_action(action)).await {
                        match dda_com_client.publish_action(action).await {
                            Ok(res) => {
                                let mut stream = res.into_inner();
                                // TODO: add polling using tokio::select
                                match stream.message().await {
                                    Ok(response) => {
                                        let action_result = response.expect("expected an action result!").data;
                                        let res = dda::DDA::ComSubscribeActionResult(action_result);
                                        let r = serde_json::to_string(&res).expect("wrong");
                                        log::info!("returning to the dda library {:?}", r);
                                        dataplane_handle
                                            .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply(r))
                                            .await;
                                    }
                                    Err(status) => {
                                        log::error!("could not retrieve an action within the timeout {:?}", status);
                                        dataplane_handle
                                            .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::NoReply)
                                            .await;
                                    }
                                }
                            }
                            Err(status) => {
                                log::error!("gRPC call to sidecar failed {:?}", status);
                                dataplane_handle
                                    .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::NoReply)
                                    .await;
                                continue;
                            }
                        };
                    }
                    dda::DDA::ComPublishQuery(alias, data) => {
                        dataplane_handle
                            .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Err)
                            .await;
                        todo!("finish")
                    }
                    dda::DDA::ComPublishActionResult(correlation_id, data) => {
                        let mut action_result = dda_com::ActionResult::default();
                        action_result.data = data;
                        action_result.sequence_number = 0; // one action result will be published
                        let action_result_correlated = dda_com::ActionResultCorrelated {
                            result: Some(action_result),
                            correlation_id,
                        };
                        let _ = dda_com_client.publish_action_result(action_result_correlated).await;
                        dataplane_handle
                            .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::NoReply)
                            .await;
                    }
                    dda::DDA::ComPublishQueryResult(correlation_id, data) => {
                        dataplane_handle
                            .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Err)
                            .await;
                        todo!("finish")
                    }

                    _ => {
                        dataplane_handle
                            .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Err)
                            .await;
                        panic!("orphan publication");
                    }
                }
            }
        });
        sub_tasks.push(_dda_task);
        log::info!("DDA resource created, connected to the DDA sidecar at url={}", dda_url);
        dda_resource.sub_tasks = sub_tasks;
        return dda_resource;
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
        if let (Some(dda_url), Some(dda_com_subscription_mapping), Some(dda_com_publication_mapping)) = (
            instance_specification.configuration.get("dda_url"),
            instance_specification.configuration.get("dda_com_subscription_mapping"),
            instance_specification.configuration.get("dda_com_publication_mapping"),
        ) {
            let mut lck = self.inner.lock().await;
            // creates a new id for the new DDA Instance with the node_id of the
            // resource provider as its component
            let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id.clone()).await;

            // insert the initial output_mapping before the Instance is created
            // to avoid data race
            lck.mappings.insert(new_id.function_id, instance_specification.output_mapping);

            // create the resource
            let dda_res = DDAResource::new(
                self.clone(),
                dataplane_handle,
                dda_url.clone(),
                dda_com_subscription_mapping.clone(),
                dda_com_publication_mapping.clone(),
                new_id.function_id,
            )
            .await;
            // save a reference to the dda instance for future reference
            lck.instances.insert(new_id.function_id, dda_res);
            Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id))
        } else {
            Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Invalid resource configuration".to_string(),
                    detail: Some("dda configuration incomplete: consult the docs".to_string()),
                },
            ))
        }
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> anyhow::Result<()> {
        let mut lck = self.inner.lock().await;
        match lck.instances.get_mut(&resource_id.function_id) {
            Some(instance) => {
                instance.sub_tasks.iter().for_each(|i| i.abort());
            }
            None => {
                return Err(anyhow::anyhow!("Stopping a non-existing resource instance: {}", resource_id.function_id));
            }
        };
        // don't forget the mappings
        lck.mappings.remove(&resource_id.function_id);
        Ok(())
    }

    // always gets called after instantiation
    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        let mut lck = self.inner.lock().await;
        // update the mappings
        log::info!("Patching request to dda provider {:?}", update);
        lck.mappings.insert(update.function_id, update.output_mapping);
        Ok(())
    }
}
