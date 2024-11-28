// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::{Future, StreamExt};

pub struct Subscriber {
    sender: futures::channel::mpsc::UnboundedSender<SubscriberRequest>,
}

enum SubscriberRequest {
    Update(),
    KeepAlive(),
}

impl Subscriber {
    pub async fn new(controller_url: String, subscription_refresh_interval_sec: u64) -> (Self, std::pin::Pin<Box<dyn Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();
        let main_task = Box::pin(async move {
            Self::main_task(receiver, controller_url, subscription_refresh_interval_sec).await;
        });

        (Self { sender }, main_task)
    }

    async fn main_task(
        receiver: futures::channel::mpsc::UnboundedReceiver<SubscriberRequest>,
        controller_url: String,
        subscription_refresh_interval_sec: u64,
    ) {
        let mut receiver = receiver;

        // main orchestration loop that reacts to events on the receiver channel
        while let Some(req) = receiver.next().await {
            match req {
                SubscriberRequest::Update() => {
                    log::info!("XXX update");
                }
                SubscriberRequest::KeepAlive() => {
                    log::info!("XX keep-alive");
                }
            }
        }
    }
}
