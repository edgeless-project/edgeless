// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use serde::ser::{Serialize, SerializeTupleVariant, Serializer};

use futures::{Future, SinkExt, StreamExt};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::str::FromStr;

use crate::node_register_client;

pub struct NodeRegister {
    sender: futures::channel::mpsc::UnboundedSender<NodeRegisterRequest>,
}

pub enum NodeRegisterRequest {
    UpdateNode(
        edgeless_api::node_registration::UpdateNodeRequest,
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse>>,
    ),
}

#[derive(Clone)]
pub struct NodeRegistrationClient {
    sender: futures::channel::mpsc::UnboundedSender<NodeRegisterRequest>,
}

impl NodeRegister {
    pub async fn new() -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            Self::main_task(receiver).await;
        });

        (NodeRegister { sender }, main_task)
    }

    async fn main_task(receiver: futures::channel::mpsc::UnboundedReceiver<NodeRegisterRequest>) {
        let mut receiver = receiver;

        // main orchestration loop that reacts to events on the receiver channel
        while let Some(req) = receiver.next().await {
            match req {
                // XXX
                NodeRegisterRequest::UpdateNode(request, reply_channel) => {
                    //
                } // Update the map of clients and, at the same time, prepare
                  //     // the edgeless_api::node_management::UpdatePeersRequest message to be sent to all the
                  //     // clients to notify that a new node exists (Register) or
                  //     // that an existing node left the system (Deregister).
                  //     let mut this_node_id = None;
                  //     let msg = match request {
                  //         edgeless_api::node_registration::UpdateNodeRequest::Registration(
                  //             node_id,
                  //             agent_url,
                  //             invocation_url,
                  //             resources,
                  //             capabilities,
                  //         ) => {
                  //             let mut dup_entry = false;
                  //             if let Some(client_desc) = nodes.get(&node_id) {
                  //                 if client_desc.agent_url == agent_url && client_desc.invocation_url == invocation_url {
                  //                     dup_entry = true;
                  //                 }
                  //             }
                  //             if dup_entry {
                  //                 // A client with same node_id, agent_url, and
                  //                 // invocation_url already exists.
                  //                 None
                  //             } else {
                  //                 this_node_id = Some(node_id);

                  //                 // Create the resource configuration APIs.
                  //                 for resource in &resources {
                  //                     log::info!("new resource advertised by node {}: {}", this_node_id.unwrap(), resource);

                  //                     if resource_providers.contains_key(&resource.provider_id) {
                  //                         log::warn!(
                  //                             "cannot add resource because another one exists with the same provider_id: {}",
                  //                             resource.provider_id
                  //                         )
                  //                     } else {
                  //                         assert!(this_node_id.is_some());

                  //                         resource_providers.insert(
                  //                             resource.provider_id.clone(),
                  //                             ResourceProvider {
                  //                                 class_type: resource.class_type.clone(),
                  //                                 node_id: this_node_id.unwrap(),
                  //                                 outputs: resource.outputs.clone(),
                  //                             },
                  //                         );
                  //                         resource_providers_changed = true;
                  //                     }
                  //                 }

                  //                 // Create the agent API.
                  //                 log::info!(
                  //                     "added function instance client: node_id {}, agent URL {}, invocation URL {}, capabilities {}",
                  //                     node_id,
                  //                     agent_url,
                  //                     invocation_url,
                  //                     capabilities
                  //                 );

                  //                 let (proto, host, port) = edgeless_api::util::parse_http_host(&agent_url).unwrap();
                  //                 let api: Box<dyn edgeless_api::outer::agent::AgentAPI + Send> = match proto {
                  //                     edgeless_api::util::Proto::COAP => {
                  //                         let addr = std::net::SocketAddrV4::new(host.parse().unwrap(), port);
                  //                         Box::new(edgeless_api::coap_impl::CoapClient::new(addr).await)
                  //                     }
                  //                     _ => Box::new(edgeless_api::grpc_impl::outer::agent::AgentAPIClient::new(&agent_url).await),
                  //                 };
                  //                 log::info!("got api");

                  //                 nodes.insert(
                  //                     node_id,
                  //                     ClientDesc {
                  //                         agent_url: agent_url.clone(),
                  //                         invocation_url: invocation_url.clone(),
                  //                         api,
                  //                         capabilities,
                  //                     },
                  //                 );

                  //                 Some(edgeless_api::node_management::UpdatePeersRequest::Add(node_id, invocation_url))
                  //             }
                  //         }
                  //         edgeless_api::node_registration::UpdateNodeRequest::Deregistration(node_id) => {
                  //             if !nodes.contains_key(&node_id) {
                  //                 // There is no client with that node_id
                  //                 None
                  //             } else {
                  //                 nodes.remove(&node_id);
                  //                 Some(edgeless_api::node_management::UpdatePeersRequest::Del(node_id))
                  //             }
                  //         }
                  //     };

                  //     // If no operation was done (either a new node was already
                  //     // present with same agent/invocation URLs or a deregistering
                  //     // node did not exist) we accept the command.
                  //     let mut response = edgeless_api::node_registration::UpdateNodeResponse::Accepted;

                  //     if let Some(msg) = msg {
                  //         // Update the orchestration logic & proxy with the new set of nodes.
                  //         orchestration_logic.update_nodes(&nodes, &resource_providers);
                  //         proxy.update_nodes(&nodes);

                  //         // Update all the peers (including the node, unless it
                  //         // was a deregister operation).
                  //         let mut num_failures: u32 = 0;
                  //         for (_node_id, client) in nodes.iter_mut() {
                  //             if client.api.node_management_api().update_peers(msg.clone()).await.is_err() {
                  //                 num_failures += 1;
                  //             }
                  //         }

                  //         log::info!("updated peers");

                  //         // Only with registration, we also update the new node
                  //         // by adding as peers all the existing nodes.
                  //         if let Some(this_node_id) = this_node_id {
                  //             let mut new_node_client = nodes.get_mut(&this_node_id).unwrap().api.node_management_api();
                  //             for (other_node_id, client_desc) in nodes.iter_mut() {
                  //                 if other_node_id.eq(&this_node_id) {
                  //                     continue;
                  //                 }
                  //                 if new_node_client
                  //                     .update_peers(edgeless_api::node_management::UpdatePeersRequest::Add(
                  //                         *other_node_id,
                  //                         client_desc.invocation_url.clone(),
                  //                     ))
                  //                     .await
                  //                     .is_err()
                  //                 {
                  //                     num_failures += 1;
                  //                 }
                  //             }
                  //         }

                  //         response = match num_failures {
                  //             0 => edgeless_api::node_registration::UpdateNodeResponse::Accepted,
                  //             _ => edgeless_api::node_registration::UpdateNodeResponse::ResponseError(edgeless_api::common::ResponseError {
                  //                 summary: "UpdatePeers() failed on some node when updating a node".to_string(),
                  //                 detail: None,
                  //             }),
                  //         };
                  //     }

                  //     if let Err(err) = reply_channel.send(Ok(response)) {
                  //         log::error!("Orchestrator channel error in UPDATENODE: {:?}", err);
                  //     }
                  // }
            }
        }
    }

    pub fn get_node_registration_client(&mut self) -> Box<dyn edgeless_api::outer::node_register::NodeRegisterAPI + Send> {
        node_register_client::NodeRegisterClient::new(self.sender.clone())
    }
}

#[async_trait::async_trait]
impl edgeless_api::node_registration::NodeRegistrationAPI for NodeRegistrationClient {
    async fn update_node(
        &mut self,
        request: edgeless_api::node_registration::UpdateNodeRequest,
    ) -> anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse> {
        log::debug!("NodeRegistrationAPI::update_node() {:?}", request);
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse>>();
        if let Err(err) = self.sender.send(NodeRegisterRequest::UpdateNode(request, reply_sender)).await {
            return Err(anyhow::anyhow!("Orchestrator channel error when updating a node: {}", err.to_string()));
        }
        match reply_receiver.await {
            Ok(res) => res,
            Err(err) => Err(anyhow::anyhow!("Orchestrator channel error  when updating a node: {}", err.to_string())),
        }
    }
}
