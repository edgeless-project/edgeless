// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_api::outer::node_register::NodeRegisterAPI;
use futures::{Future, SinkExt, StreamExt};
use rand::distributions::Distribution;

#[derive(Clone)]
pub struct NodeSubscriber {
    sender: futures::channel::mpsc::UnboundedSender<NodeSubscriberRequest>,
}

#[derive(Clone)]
pub enum NodeSubscriberRequest {
    Refresh(),
}

impl NodeSubscriber {
    pub async fn new(
        node_register_url: String,
        node_id: uuid::Uuid,
        agent_url: String,
        invocation_url: String,
        resource_providers: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
        capabilities: edgeless_api::node_registration::NodeCapabilities,
        subscription_refresh_interval_sec: u64,
        telemetry_performance_target: edgeless_telemetry::performance_target::PerformanceTargetInner,
    ) -> (
        Self,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
    ) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let sender_cloned = sender.clone();
        let mut rng = rand::thread_rng();
        let mut counter = rand::distributions::Uniform::from(0..u64::MAX).sample(&mut rng);

        let main_task = Box::pin(async move {
            Self::main_task(
                node_register_url,
                node_id,
                agent_url,
                invocation_url,
                resource_providers,
                capabilities,
                subscription_refresh_interval_sec,
                counter,
                receiver,
                telemetry_performance_target,
            )
            .await;
        });

        let refresh_task = Box::pin(async move {
            Self::refresh_task(sender_cloned, subscription_refresh_interval_sec).await;
        });

        (Self { sender }, main_task, refresh_task)
    }

    async fn refresh_task(sender: futures::channel::mpsc::UnboundedSender<NodeSubscriberRequest>, subscription_refresh_interval_sec: u64) {
        let mut sender = sender;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(subscription_refresh_interval_sec));
        loop {
            interval.tick().await;
            let _ = sender.send(NodeSubscriberRequest::Refresh()).await;
        }
    }

    async fn main_task(
        node_register_url: String,
        node_id: uuid::Uuid,
        agent_url: String,
        invocation_url: String,
        resource_providers: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
        capabilities: edgeless_api::node_registration::NodeCapabilities,
        subscription_refresh_interval_sec: u64,
        counter: u64,
        receiver: futures::channel::mpsc::UnboundedReceiver<NodeSubscriberRequest>,
        telemetry_performance_target: edgeless_telemetry::performance_target::PerformanceTargetInner,
    ) {
        let mut receiver = receiver;
        let mut client = edgeless_api::grpc_impl::outer::node_register::NodeRegisterAPIClient::new(node_register_url).await;

        while let Some(req) = receiver.next().await {
            match req {
                NodeSubscriberRequest::Refresh() => {
                    log::debug!("Node Subscriber Refresh");
                    // The refresh deadline is set to twice the refresh period
                    // to reduce the likelihood of a race condition on the
                    // register side.
                    let update_node_request = edgeless_api::node_registration::UpdateNodeRequest {
                        node_id: node_id.clone(),
                        invocation_url: invocation_url.clone(),
                        agent_url: agent_url.clone(),
                        resource_providers: resource_providers.clone(),
                        capabilities: capabilities.clone(),
                        refresh_deadline: std::time::SystemTime::now() + std::time::Duration::from_secs(subscription_refresh_interval_sec * 2),
                        counter,
                        health_status: edgeless_api::node_registration::NodeHealthStatus::default(), // XXX
                        performance_samples: edgeless_api::node_registration::NodePerformanceSamples::default(), // XXX
                    };
                    match client.node_registration_api().update_node(update_node_request).await {
                        Ok(response) => {
                            if let edgeless_api::node_registration::UpdateNodeResponse::ResponseError(err) = response {
                                log::error!("Update of node '{}' rejected by node register: {}", node_id, err);
                            }
                        }
                        Err(err) => log::error!("Update of node '{}' failed: {}", node_id, err),
                    };
                }
            }
        }
    }

    pub fn get_subscriber_sender(&mut self) -> futures::channel::mpsc::UnboundedSender<NodeSubscriberRequest> {
        self.sender.clone()
    }
}
