// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use edgeless_api::outer::domain_register::DomainRegisterAPI;
use futures::{Future, SinkExt, StreamExt};

#[derive(Clone)]
pub struct DomainSubscriber {
    sender: futures::channel::mpsc::UnboundedSender<DomainSubscriberRequest>,
}

#[derive(Clone)]
pub enum DomainSubscriberRequest {
    Update(edgeless_api::domain_registration::DomainCapabilities),
    Refresh(),
}

impl DomainSubscriber {
    pub async fn new(
        domain_id: String,
        orchestrator_url: String,
        controller_url: String,
        subscription_refresh_interval_sec: u64,
    ) -> (
        Self,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
        std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
    ) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let sender_cloned = sender.clone();
        let main_task = Box::pin(async move {
            Self::main_task(domain_id, orchestrator_url, controller_url, subscription_refresh_interval_sec, receiver).await;
        });

        let refresh_task = Box::pin(async move {
            Self::refresh_task(sender_cloned, subscription_refresh_interval_sec).await;
        });

        (Self { sender }, main_task, refresh_task)
    }

    async fn refresh_task(sender: futures::channel::mpsc::UnboundedSender<DomainSubscriberRequest>, subscription_refresh_interval_sec: u64) {
        let mut sender = sender;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(subscription_refresh_interval_sec));
        loop {
            interval.tick().await;
            let _ = sender.send(DomainSubscriberRequest::Refresh()).await;
        }
    }

    async fn main_task(
        domain_id: String,
        orchestrator_url: String,
        controller_url: String,
        subscription_refresh_interval_sec: u64,
        receiver: futures::channel::mpsc::UnboundedReceiver<DomainSubscriberRequest>,
    ) {
        let mut receiver = receiver;

        let mut client = edgeless_api::grpc_impl::outer::domain_register::DomainRegisterAPIClient::new(controller_url).await;
        let mut last_caps = edgeless_api::domain_registration::DomainCapabilities::default();
        let mut counter = 0;

        while let Some(req) = receiver.next().await {
            match req {
                DomainSubscriberRequest::Update(new_caps) => {
                    log::debug!("Subscriber Update {:?}", new_caps);
                    counter += 1;
                    last_caps = new_caps;
                }
                DomainSubscriberRequest::Refresh() => {
                    log::debug!("Subscriber Refresh");
                    // The refresh deadline is set to twice the refresh period
                    // to reduce the likelihood of a race condition on the domai
                    // register side.
                    let update_domain_request = edgeless_api::domain_registration::UpdateDomainRequest {
                        domain_id: domain_id.clone(),
                        orchestrator_url: orchestrator_url.clone(),
                        capabilities: last_caps.clone(),
                        refresh_deadline: std::time::SystemTime::now() + std::time::Duration::from_secs(subscription_refresh_interval_sec * 2),
                        counter,
                    };
                    match client.domain_registration_api().update_domain(update_domain_request).await {
                        Ok(response) => {
                            if let edgeless_api::domain_registration::UpdateDomainResponse::ResponseError(err) = response {
                                log::error!("Update of domain '{}' rejected by controller: {}", domain_id, err);
                            }
                        }
                        Err(err) => log::error!("Update of domain '{}' failed: {}", domain_id, err),
                    };
                }
            }
        }
    }

    pub fn get_subscriber_sender(&mut self) -> futures::channel::mpsc::UnboundedSender<DomainSubscriberRequest> {
        self.sender.clone()
    }
}
