// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

use dda_state::{ObserveMembershipChangeParams, ObserveStateChangeParams};
use dda_store::KeyValue;
use edgeless_api::function_instance::InstanceId;
use edgeless_api::resource_configuration::ResourceConfigurationAPI;
use serde::Deserialize;
use serde_json::Error;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct DdaResourceSpec {}

impl super::resource_provider_specs::ResourceProviderSpecs for DdaResourceSpec {
    fn class_type(&self) -> String {
        String::from("dda")
    }

    fn description(&self) -> String {
        r"Interact with a Data Distribution Agent -- see https://github.com/coatyio/dda".to_string()
    }

    fn outputs(&self) -> Vec<String> {
        vec![String::from("out")]
    }

    fn configurations(&self) -> std::collections::HashMap<String, String> {
        std::collections::HashMap::from([
            (String::from("dda_url"), String::from("URL of the DDA")),
            (
                String::from("dda_com_subscription_mapping"),
                String::from("JSON encoding the DDA subscription mapping"),
            ),
            (
                String::from("dda_com_publication_mapping"),
                String::from("JSON encoding the DDA publication mapping"),
            ),
        ])
    }

    fn version(&self) -> String {
        String::from("1.1")
    }
}

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

// There is only one provider per node and it is responsible for providing
// access to any number of logical DDA resources.
// Through this trait, when the provider is cloned, we still have only a singleton
#[derive(Clone)]
pub struct DDAResourceProvider {
    // inner is a singleton behind arc + mutex
    inner: Arc<Mutex<DDAResourceProviderInner>>,
}

impl DDAResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        resource_provider_id: InstanceId,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(DDAResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                telemetry_handle,
                instances: HashMap::<Uuid, DDAResource>::new(),
                // TODO: inner hashmap should be mapped to a vector of
                // InstanceIDs -> in case we decide that multiple functions can
                // listen to the same dda event
                // NOTE: For now, we assume that each subscription has only one
                // target per workflow to avoid duplicating incoming events -
                // this is up to discussion later on.
                mappings: HashMap::<Uuid, HashMap<String, InstanceId>>::new(),
            })),
        }
    }
}

/// Implements the ResourceConfigurationAPI for the DDAResource
#[async_trait::async_trait]
impl ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for DDAResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> anyhow::Result<
        edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>,
    > {
        // read the sidecar url from the instance specification configuration
        if let (
            Some(dda_url),
            Some(dda_com_subscription_mapping),
            Some(dda_com_publication_mapping),
        ) = (
            instance_specification.configuration.get("dda_url"),
            instance_specification
                .configuration
                .get("dda_com_subscription_mapping"),
            instance_specification
                .configuration
                .get("dda_com_publication_mapping"),
        ) {
            let mut lck = self.inner.lock().await;
            // creates a new id for the new DDA Instance with the node_id of the
            // resource provider as its component
            let new_id =
                edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id).await;

            // create the resource
            let dda_res = match DDAResource::new(
                self.clone(),
                dataplane_handle,
                lck.telemetry_handle
                    .fork(std::collections::BTreeMap::from([(
                        "FUNCTION_ID".to_string(),
                        new_id.function_id.to_string(),
                    )])),
                dda_url.clone(),
                dda_com_subscription_mapping.clone(),
                dda_com_publication_mapping.clone(),
                new_id.function_id,
            )
            .await
            {
                Ok(res) => res,
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Wrong configuration of DDA resource in workflow.json: {}",
                        e
                    ))
                }
            };
            // save a reference to the dda instance for future reference
            lck.instances.insert(new_id.function_id, dda_res);
            Ok(edgeless_api::common::StartComponentResponse::InstanceId(
                new_id,
            ))
        } else {
            Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                edgeless_api::common::ResponseError {
                    summary: "Invalid resource configuration".to_string(),
                    detail: Some("dda configuration incomplete: consult the docs".to_string()),
                },
            ))
        }
    }

    async fn stop(
        &mut self,
        resource_id: edgeless_api::function_instance::InstanceId,
    ) -> anyhow::Result<()> {
        let mut lck = self.inner.lock().await;
        match lck.instances.get_mut(&resource_id.function_id) {
            Some(instance) => {
                instance.sub_tasks.iter().for_each(|i| i.abort());
            }
            None => {
                return Err(anyhow::anyhow!(
                    "Stopping a non-existing resource instance: {}",
                    resource_id.function_id
                ));
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
        lck.mappings
            .insert(update.function_id, update.output_mapping);
        Ok(())
    }
}

struct DDAResourceProviderInner {
    resource_provider_id: InstanceId, // resource provider is the edgeless node
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
    // there is a single DDA Sidecar per edgeless_node, but there can be any
    // number of workflows that use separate DDA resources to work
    instances: HashMap<Uuid, DDAResource>,
    // NOTE: limitation: since output mapping is set through the patching
    // request, there is a small window of time during which incoming DDA
    // events would not get correctly sent to functions that might have
    // already been started (race condition). For now we ignore this edge
    // case.
    mappings: HashMap<Uuid, HashMap<String, InstanceId>>,
}

pub struct DDAResource {
    // gRPC clients get dropped automatically when the tokio tasks are aborted
    sub_tasks: Vec<tokio::task::JoinHandle<()>>,
}

// used to map incoming dda events to dataplane subscriptions
#[derive(Debug, Deserialize, Clone)]
struct DDAComSubscription {
    topic: String,   // also known as type in DDA
    pattern: String, // action / event / query / state / membership
    method: String,  // cast or call as means of passing the dda event
    target: String,  // which function should be invoked (or alias of another resource)
}

// used to map events from functions to dda publications
#[derive(Debug, Deserialize, Clone)]
struct DDAComPublication {
    topic: String,   // also known as type in DDA
    pattern: String, // action / event/ query / input
    alias: String,   // used to identify a publication mapping
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
        provider: DDAResourceProvider,
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        telemetry_handle: Box<dyn edgeless_telemetry::telemetry_events::TelemetryHandleAPI>,
        dda_url: String,
        dda_com_subscription_mapping: String,
        dda_com_publication_mapping: String,
        self_id: Uuid,
    ) -> anyhow::Result<Self> {
        let mut telemetry_handle = telemetry_handle;
        // Parse the configuration of action / event / query bindings to functions
        let dcs: Result<Vec<DDAComSubscription>, Error> =
            serde_json::from_str(&dda_com_subscription_mapping);
        let dps: Result<Vec<DDAComPublication>, Error> =
            serde_json::from_str(&dda_com_publication_mapping);

        let dda_sub_array = match dcs {
            Ok(dda_array) => dda_array,
            Err(err) => {
                log::error!(
                    "Error parsing input dda_com_subscription_mapping JSON: {}",
                    err
                );
                return Err(anyhow::anyhow!(
                    "Error parsing input dda_com_subscription_mapping JSON: {}",
                    err
                ));
            }
        };

        use std::collections::HashSet;
        // max one (target, method) per whole mapping
        let mut seen: HashSet<(String, String, String)> = HashSet::new();
        for item in dda_sub_array.clone() {
            let key = (
                item.topic.clone(),
                item.pattern.clone(),
                item.method.clone(),
            );
            if !seen.insert(key) {
                return Err(anyhow::anyhow!(
                    "Ambiguous mapping of (target, method) in sub: ({}, {}, {}) is mapped at least twice to two different targets",
                    item.topic,
                    item.pattern,
                    item.method
                ));
            }
        }

        let dda_pub_array = match dps {
            Ok(dda_array) => dda_array,
            Err(err) => {
                log::error!(
                    "Error parsing input dda_com_publication_mapping JSON: {}",
                    err
                );
                return Err(anyhow::anyhow!(
                    "Error parsing input dda_com_publication_mapping JSON: {}",
                    err
                ));
            }
        };
        // each alias must be present maximally once
        let mut seen: HashSet<String> = HashSet::new();
        for item in dda_pub_array.clone() {
            let key = item.alias.clone();
            if !seen.insert(key) {
                return Err(anyhow::anyhow!(
                    "Ambiguous mapping of (alias) in sub: {} is mapped at least twice",
                    item.alias
                ));
            }
        }
        let dda_pub_map: HashMap<String, DDAComPublication> = dda_pub_array
            .into_iter()
            .map(|p| (p.alias.clone(), p))
            .collect();

        // always connect all clients to the sidecar, because the function can
        // use any of the subsystems
        let mut dda_com_client =
            dda_com::com_service_client::ComServiceClient::connect(dda_url.clone())
                .await
                .expect("dda sidecar: com connection failed");

        let mut dda_state_client =
            dda_state::state_service_client::StateServiceClient::connect(dda_url.clone())
                .await
                .expect("dda sidecar: state connection failed");

        let mut dda_store_client =
            dda_store::store_service_client::StoreServiceClient::connect(dda_url.clone())
                .await
                .expect("dda sidecar: store connection failed");

        // subscribe to configured dda topics
        let mut sub_tasks: Vec<tokio::task::JoinHandle<()>> = vec![];
        // constructed here to get access to Self
        let mut dda_resource = Self { sub_tasks: vec![] };

        let (sender, receiver) =
            tokio::sync::mpsc::unbounded_channel::<(dda::DDA, DDAComSubscription)>();
        let mut receiver = receiver;
        let mut handle = dataplane_handle.clone();
        let passer_task = tokio::spawn(async move {
            while let Some((event, dda_sub)) = receiver.recv().await {
                // if receiver.len() > 10 {
                //     log::warn!("a lot of messages are waiting to be processed");
                // }
                // mapping has to be looked up each time, since the dataplane
                // can change
                let mapping = provider
                    .inner
                    .lock()
                    .await
                    .mappings
                    .get(&self_id)
                    .cloned()
                    .expect("invalid mapping for DDA");
                let encoded_event = serde_json::to_string(&event).expect("malformed DDA event");
                let target_function_id = mapping.get(&dda_sub.target.to_string()).cloned();
                let dataplane_event_type = dda_sub.method;
                match target_function_id {
                    Some(target_function_id) => match dataplane_event_type.as_str() {
                        "cast" => {
                            let _ = handle
                                .send(
                                    target_function_id,
                                    encoded_event,
                                    &edgeless_api::function_instance::EventMetadata::empty_new_root(
                                    ),
                                )
                                .await;
                        }
                        "call" => {
                            panic!("do not use calls - they will probably be removed later on");
                            // let _ = handle.call(target_function_id, encoded_event).await;
                        }
                        _ => {
                            panic!("Unexpected method used in DDA mapping");
                        }
                    },
                    None => {
                        log::warn!(
                            "Unknown target for incoming event that was subscribed {:?}",
                            event
                        );
                    }
                }
            }
        });
        sub_tasks.push(passer_task);

        for dda_sub in dda_sub_array {
            let sender = sender.clone();
            // topic is used as the type for filtering
            let dda_subscription_filter = dda_com::SubscriptionFilter {
                r#type: dda_sub.topic.clone(),
                ..Default::default()
            };
            // start a subscription based on the pattern
            let sub_task = match dda_sub.pattern.as_str() {
                "event" => {
                    let mut dda_com_client = dda_com_client.clone();
                    tokio::spawn(async move {
                        let mut event_stream = match dda_com_client
                            .subscribe_event(dda_subscription_filter)
                            .await
                        {
                            Ok(dda_resp) => dda_resp.into_inner(),
                            Err(err) => {
                                panic!(
                                    "dda event subscription failed {:?} - {}",
                                    dda_sub.topic, err
                                );
                            }
                        };
                        loop {
                            match event_stream.message().await {
                                Ok(e) => {
                                    let _ = sender.send((
                                        dda::DDA::ComSubscribeEvent(e.unwrap().data),
                                        dda_sub.clone(),
                                    ));
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
                        let mut action_stream = match dda_com_client
                            .subscribe_action(dda_subscription_filter)
                            .await
                        {
                            Ok(dda_resp) => dda_resp.into_inner(),
                            Err(err) => {
                                panic!(
                                    "dda action subscription failed {:?} - {}",
                                    dda_sub.topic, err
                                );
                            }
                        };
                        loop {
                            match action_stream.message().await {
                                Ok(e) => {
                                    let action_correlated = e.unwrap();
                                    let _ = sender.send((
                                        dda::DDA::ComSubscribeAction(
                                            action_correlated.correlation_id,
                                            action_correlated.action.unwrap().params,
                                        ),
                                        dda_sub.clone(),
                                    ));
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
                        let mut query_stream = match dda_com_client
                            .subscribe_query(dda_subscription_filter)
                            .await
                        {
                            Ok(dda_resp) => dda_resp.into_inner(),
                            Err(err) => {
                                panic!(
                                    "dda query subscription failed {:?} - {}",
                                    dda_sub.topic, err
                                );
                            }
                        };
                        loop {
                            match query_stream.message().await {
                                Ok(e) => {
                                    let query_correlated = e.unwrap();
                                    let _ = sender.send((
                                        dda::DDA::ComSubscribeQuery(
                                            query_correlated.correlation_id,
                                            query_correlated.query.unwrap().data,
                                        ),
                                        dda_sub.clone(),
                                    ));
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
                        let mut state_input_stream =
                            match dda_state_client.observe_state_change(params).await {
                                Ok(state_input) => state_input.into_inner(),
                                Err(err) => {
                                    panic!(
                                        "dda input subscription failed {:?} - {}",
                                        dda_sub.topic, err
                                    );
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
                                    let _ = sender.send((event, dda_sub.clone()));
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
                        let mut membership_change_stream =
                            match dda_state_client.observe_membership_change(params).await {
                                Ok(memb_change) => memb_change.into_inner(),
                                Err(err) => {
                                    panic!(
                                        "dda membership subscription failed {:?} - {}",
                                        dda_sub.topic, err
                                    );
                                }
                            };
                        loop {
                            match membership_change_stream.message().await {
                                Ok(m) => {
                                    let m = m.unwrap();
                                    let _ = sender.send((
                                        dda::DDA::StateSubscribeMembershipChange(m.id, m.joined),
                                        dda_sub.clone(),
                                    ));
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
        }

        // Spawn asynchrounous task to handle edgeless dataplane events -
        // these are incoming events from e.g. edgeless functions that need to
        // be sent out etc.
        let id: u128 = 0;
        let mut dataplane_handle = dataplane_handle.clone();
        let _dda_task = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    created,
                    metadata,
                } = dataplane_handle.receive_next().await;
                let started = crate::resources::observe_transfer(created, &mut telemetry_handle);
                let message: dda::DDA = match message {
                    edgeless_dataplane::core::Message::Call(data) => {
                        // all calls to DDA resource must be Calls with
                        // DataplaneDDA as serialized data
                        serde_json::from_str::<dda::DDA>(&data)
                            .expect("wrong incoming dataplane event from a function")
                    }
                    _ => {
                        // disregard anything but Calls
                        continue;
                    }
                };

                let mut handle = dataplane_handle.clone();
                let respond = {
                    move |msg: edgeless_dataplane::core::CallRet| async move {
                        let _ = handle.reply(source_id, channel_id, msg, &metadata).await;
                    }
                };

                match message {
                    dda::DDA::ComPublishEvent(alias, data) => {
                        let p = match dda_pub_map.get(&alias) {
                            Some(al) => al,
                            None => {
                                log::error!("Alias not found! Outgoing message will be ignored!");
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                                continue;
                            }
                        };
                        if p.pattern != "event" {
                            log::warn!("wrong publication type");
                            respond(edgeless_dataplane::core::CallRet::Err).await;
                            continue;
                        }
                        let event = dda_com::Event {
                            source: self_id.clone().to_string(),
                            id: id.to_string(),
                            r#type: p.topic.to_string(),
                            data,
                            ..Default::default()
                        };
                        let _ = dda_com_client.publish_event(event).await;
                        respond(edgeless_dataplane::core::CallRet::Reply("".to_string())).await;
                    }
                    dda::DDA::ComPublishAction(alias, data) => {
                        let p = match dda_pub_map.get(&alias) {
                            Some(p) => p,
                            None => {
                                log::warn!("attempting to publish an action using an alias which is not mapped!");
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                                continue;
                            }
                        };
                        if p.pattern != "action" {
                            log::warn!("wrong publication type");
                            respond(edgeless_dataplane::core::CallRet::Err).await;
                            continue;
                        }
                        // construct the Action
                        let action = dda_com::Action {
                            source: self_id.clone().to_string(),
                            id: id.to_string(),
                            r#type: p.topic.to_string(),
                            params: data,
                            ..Default::default()
                        };

                        // wait for an action response (currently 1)
                        match dda_com_client.publish_action(action).await {
                            Ok(res) => {
                                let mut stream = res.into_inner();
                                match stream.message().await {
                                    Ok(response) => {
                                        let action_result =
                                            response.expect("expected an action result!").data;
                                        let res = dda::DDA::ComSubscribeActionResult(action_result);
                                        let r = serde_json::to_string(&res).expect("wrong");
                                        respond(edgeless_dataplane::core::CallRet::Reply(r)).await;
                                    }
                                    Err(status) => {
                                        log::error!(
                                            "could not retrieve an action result {:?}",
                                            status
                                        );
                                        respond(edgeless_dataplane::core::CallRet::Err).await;
                                    }
                                }
                            }
                            Err(status) => {
                                log::error!("gRPC call to sidecar failed {:?}", status);
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                                continue;
                            }
                        };
                    }
                    dda::DDA::ComPublishQuery(alias, data) => {
                        let p = match dda_pub_map.get(&alias) {
                            Some(p) => p,
                            None => {
                                log::warn!("attempting to publish a query using an alias which is not mapped!");
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                                continue;
                            }
                        };
                        if p.pattern != "query" {
                            log::warn!(
                                "can not publish a query using alias={:?}. Mapping specifies: {:?}",
                                alias,
                                p.pattern
                            );
                            respond(edgeless_dataplane::core::CallRet::Err).await;
                            continue;
                        }
                        // construct the Query
                        let query = dda_com::Query {
                            source: self_id.clone().to_string(),
                            id: id.to_string(),
                            r#type: p.topic.to_string(),
                            data,
                            ..Default::default()
                        };

                        // wait for an action response as specified in the
                        // parameters - currently waiting for one response
                        match dda_com_client.publish_query(query).await {
                            Ok(res) => {
                                let mut stream = res.into_inner();
                                match stream.message().await {
                                    Ok(response) => {
                                        let query_result =
                                            response.expect("expected a query result!").data;
                                        let res = dda::DDA::ComSubscribeQueryResult(query_result);
                                        let r = serde_json::to_string(&res)
                                            .expect("should never happen");
                                        respond(edgeless_dataplane::core::CallRet::Reply(r)).await;
                                    }
                                    Err(status) => {
                                        log::error!(
                                            "could not get any result for a query{:?}",
                                            status
                                        );
                                        respond(edgeless_dataplane::core::CallRet::Err).await;
                                    }
                                }
                            }
                            Err(status) => {
                                log::error!("gRPC call to sidecar failed {:?}", status);
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        };
                    }
                    dda::DDA::ComPublishActionResult(correlation_id, data) => {
                        let action_result = dda_com::ActionResult {
                            data,
                            sequence_number: 0,
                            ..Default::default()
                        };
                        let action_result_correlated = dda_com::ActionResultCorrelated {
                            result: Some(action_result),
                            correlation_id,
                        };
                        match dda_com_client
                            .publish_action_result(action_result_correlated)
                            .await
                        {
                            Ok(_) => {
                                respond(edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                    .await
                            }
                            Err(status) => {
                                log::error!("publishing action result failed: {:?}", status);
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    dda::DDA::ComPublishQueryResult(correlation_id, data) => {
                        let query_result = dda_com::QueryResult {
                            data,
                            sequence_number: 0,
                            ..Default::default()
                        };
                        let query_result_correlated = dda_com::QueryResultCorrelated {
                            result: Some(query_result),
                            correlation_id,
                        };
                        match dda_com_client
                            .publish_query_result(query_result_correlated)
                            .await
                        {
                            Ok(_) => {
                                respond(edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                    .await
                            }
                            Err(status) => {
                                log::error!(
                                    "publishing query result failed: {:?}",
                                    status.message()
                                );
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    dda::DDA::StatePublishSet(key, value) => {
                        let set_input = dda_state::Input {
                            op: dda_state::InputOperation::Set as i32,
                            key,
                            value,
                        };
                        match dda_state_client.propose_input(set_input).await {
                            Ok(_) => {
                                respond(edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                log::error!("DDA: StatePublishSet: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    dda::DDA::StatePublishDelete(key) => {
                        let delete_input = dda_state::Input {
                            op: dda_state::InputOperation::Delete as i32,
                            key,
                            value: vec![], // empty
                        };
                        match dda_state_client.propose_input(delete_input).await {
                            Ok(_) => {
                                respond(edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                log::error!("DDA: StatePublishDelete: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    dda::DDA::StoreGet(key) => {
                        let get = dda_store::Key { key };
                        match dda_store_client.get(get).await {
                            Ok(val) => match val.into_inner().value {
                                Some(v) => {
                                    let v_as_str =
                                        String::from_utf8(v).expect("should never happen");
                                    respond(edgeless_dataplane::core::CallRet::Reply(v_as_str))
                                        .await;
                                }
                                None => {
                                    respond(edgeless_dataplane::core::CallRet::Reply(
                                        "".to_string(),
                                    ))
                                    .await;
                                }
                            },
                            Err(e) => {
                                log::error!("DDA: StoreGet: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        };
                    }
                    dda::DDA::StoreSet(key, value) => {
                        let set = dda_store::KeyValue { key, value };
                        match dda_store_client.set(set).await {
                            Ok(_) => {
                                respond(edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                log::error!("DDA: StoreSet: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        };
                    }
                    dda::DDA::StoreDelete(key) => {
                        let delete = dda_store::Key { key };
                        match dda_store_client.delete(delete).await {
                            Ok(_) => {
                                respond(edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                log::error!("DDA: StoreDelete: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    dda::DDA::StoreDeleteAll() => {
                        let delete_all = dda_store::DeleteAllParams {};
                        match dda_store_client.delete_all(delete_all).await {
                            Ok(_) => {
                                respond(edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                log::error!("DDA: StoreDeleteAll: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    dda::DDA::StoreDeletePrefix(key) => {
                        let delete_prefix = dda_store::Key { key };
                        match dda_store_client.delete_prefix(delete_prefix).await {
                            Ok(_) => {
                                respond(edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                log::error!("DDA: StoreDeletePrefix: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    dda::DDA::StoreDeleteRange(start, end) => {
                        let delete_range = dda_store::Range { start, end };
                        match dda_store_client.delete_range(delete_range).await {
                            Ok(_) => {
                                respond(edgeless_dataplane::core::CallRet::Reply("".to_string()))
                                    .await;
                            }
                            Err(e) => {
                                log::error!("DDA: StoreDeleteRange: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    // NOTE: returns only the first KeyValue that matches the
                    // Prefix - can be adapted arbitrarily when demend arises
                    dda::DDA::StoreScanPrefix(key) => {
                        let scan_prefix = dda_store::Key { key };
                        match dda_store_client.scan_prefix(scan_prefix).await {
                            Ok(key_value_stream) => {
                                let mut stream = key_value_stream.into_inner();
                                match stream.message().await {
                                    Ok(response) => {
                                        // unwrap or provide an empty key-value
                                        let res = response.unwrap_or(KeyValue {
                                            key: "".to_string(),
                                            value: vec![],
                                        });
                                        let key_value =
                                            dda::DDA::StoreScanPrefixResult(res.key, res.value);
                                        let r = serde_json::to_string(&key_value)
                                            .expect("should never happen");
                                        respond(edgeless_dataplane::core::CallRet::Reply(r)).await;
                                    }
                                    Err(status) => {
                                        log::error!(
                                            "could not get any result for a query{:?}",
                                            status
                                        );
                                        respond(edgeless_dataplane::core::CallRet::Err).await;
                                    }
                                };
                            }
                            Err(e) => {
                                log::error!("DDA: StoreScanPrefix: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    dda::DDA::StoreScanRange(start, end) => {
                        let scan_range = dda_store::Range { start, end };
                        match dda_store_client.scan_range(scan_range).await {
                            Ok(key_value_stream) => {
                                let mut stream = key_value_stream.into_inner();
                                match stream.message().await {
                                    Ok(response) => {
                                        // unwrap or provide an empty key-value
                                        let res = response.unwrap_or(KeyValue {
                                            key: "".to_string(),
                                            value: vec![],
                                        });
                                        let key_value =
                                            dda::DDA::StoreScanRangeResult(res.key, res.value);
                                        let r = serde_json::to_string(&key_value)
                                            .expect("should never happen");
                                        respond(edgeless_dataplane::core::CallRet::Reply(r)).await;
                                    }
                                    Err(status) => {
                                        log::error!(
                                            "could not get any result for a query{:?}",
                                            status
                                        );
                                        respond(edgeless_dataplane::core::CallRet::Err).await;
                                    }
                                };
                            }
                            Err(e) => {
                                log::error!("DDA: StoreScanRange: {:?}", e.message());
                                respond(edgeless_dataplane::core::CallRet::Err).await;
                            }
                        }
                    }
                    _ => {
                        log::warn!(
                            "this should never happen - dda received an unexpected message over the dataplane from a function / other component!"
                        );
                        continue;
                    }
                }
                crate::resources::observe_execution(started, &mut telemetry_handle, false);
            }
        });
        sub_tasks.push(_dda_task);
        log::info!(
            "DDA resource created, connected to the DDA sidecar at url={}",
            dda_url
        );
        dda_resource.sub_tasks = sub_tasks;
        Ok(dda_resource)
    }
}
