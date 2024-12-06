// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::{Future, SinkExt, StreamExt};
use std::collections;

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

pub(crate) enum InternalRequest {
    Refresh(
        // Reply Channel
        tokio::sync::oneshot::Sender<()>,
    ),
}

struct NodeRegisterEntry {
    pub refresh_deadline: std::time::SystemTime,
    pub counter: u64,
}

impl NodeRegister {
    pub async fn new(
        proxy: std::sync::Arc<tokio::sync::Mutex<dyn super::proxy::Proxy>>,
        orchestrator_sender: futures::channel::mpsc::UnboundedSender<super::orchestrator::OrchestratorRequest>,
    ) -> (
        Self,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
    ) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let (internal_sender, internal_receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            Self::main_task(receiver, internal_receiver, proxy, orchestrator_sender).await;
        });

        let refresh_task = Box::pin(async move {
            let mut sender = internal_sender;
            loop {
                let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<()>();
                let _ = sender.send(InternalRequest::Refresh(reply_sender)).await;
                let _ = reply_receiver.await;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        (NodeRegister { sender }, main_task, refresh_task)
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<NodeRegisterRequest>,
        internal_receiver: futures::channel::mpsc::UnboundedReceiver<InternalRequest>,
        proxy: std::sync::Arc<tokio::sync::Mutex<dyn super::proxy::Proxy>>,
        orchestrator_sender: futures::channel::mpsc::UnboundedSender<super::orchestrator::OrchestratorRequest>,
    ) {
        let mut receiver = receiver;
        let mut internal_receiver = internal_receiver;
        let mut orchestrator_sender = orchestrator_sender;

        let mut registered: collections::HashMap<uuid::Uuid, NodeRegisterEntry> = std::collections::HashMap::new();

        // main loop that reacts to events on the receiver channels
        loop {
            tokio::select! {
            Some(req) = internal_receiver.next() => {
                match req {
                    InternalRequest::Refresh(reply_sender) => {
                        // Find all nodes that are stale, i.e., which have not been
                        // refreshed by their own indicated deadline.
                        let mut stale_nodes = vec![];
                        for (uuid, entry) in &registered {
                            if std::time::SystemTime::now() > entry.refresh_deadline {
                                stale_nodes.push(*uuid);
                            }
                        }

                        // Delete all stale nodes.
                        for stale_node in stale_nodes {
                            log::info!("Removing node '{}' because it is stale", stale_node);
                            registered.remove(&stale_node);

                            let _ = orchestrator_sender.send(super::orchestrator::OrchestratorRequest::DelNode(stale_node)).await;
                        }

                        let _ = reply_sender.send(());
                    }
                }
            },
            Some(req) = receiver.next() => match req {
                    NodeRegisterRequest::UpdateNode(request, reply_channel) => {
                        // Update the orchestrator, if needed.
                        let add_node = match registered.get_mut(&request.node_id) {
                            None => {
                                registered.insert(
                                    request.node_id,
                                    NodeRegisterEntry {
                                        refresh_deadline: request.refresh_deadline,
                                        counter: request.counter,
                                    },
                                );
                                true
                        }
                            Some(existing_node) => {
                                if existing_node.counter == request.counter {
                                    existing_node.refresh_deadline = request.refresh_deadline;
                                    false
                                } else {
                                    let _ = orchestrator_sender.send(crate::orchestrator::OrchestratorRequest::DelNode(request.node_id)).await;
                                    true
                                }
                            }
                        };
                        if add_node {
                            if let Ok(client_desc) = crate::client_desc::ClientDesc::from(&request).await {
                                let _ = orchestrator_sender.send(crate::orchestrator::OrchestratorRequest::AddNode(
                                    request.node_id,
                                    client_desc,
                                    request.resource_providers
                                )).await;
                            }
                        }

                        // Push the dynamic data to the proxy.
                        let mut proxy = proxy.lock().await;
                        proxy.push_node_health(&request.node_id, request.health_status);
                        proxy.push_performance_samples(&request.node_id, request.performance_samples);

                        if let Err(err) = reply_channel.send(Ok(edgeless_api::node_registration::UpdateNodeResponse::Accepted)) {
                            log::error!("NodeRegister channel error in UpdateNode: {:?}", err);
                        }
                    }
                }
            }
        }
    }

    pub fn get_node_registration_client(&mut self) -> Box<dyn edgeless_api::outer::node_register::NodeRegisterAPI + Send> {
        node_register_client::NodeRegisterClient::new(self.sender.clone())
    }
}
