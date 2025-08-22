// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use crate::core::*;
use futures::SinkExt;

pub struct NodeLocalRouter {
    pub receivers: std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedSender<DataplaneEvent>>,
}

// This is used by the remote node that is currently borrowing the `NodeLocalRouter`
#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for NodeLocalRouter {
    async fn handle(&mut self, event: edgeless_api::invocation::Event) -> edgeless_api::invocation::LinkProcessingResult {
        if let Some(sender) = self.receivers.get_mut(&event.target.function_id) {
            let msg = match event.data {
                edgeless_api::invocation::EventData::Call(data) => Message::Call(data),
                edgeless_api::invocation::EventData::Cast(data) => Message::Cast(data),
                edgeless_api::invocation::EventData::CallRet(data) => Message::CallRet(data),
                edgeless_api::invocation::EventData::CallNoRet => Message::CallNoRet,
                edgeless_api::invocation::EventData::Err => Message::Err,
            };
            match sender
                .send(DataplaneEvent {
                    source_id: event.source,
                    channel_id: event.stream_id,
                    message: msg,
                    created: event.created,
                    metadata: event.metadata,
                })
                .await
            {
                Ok(_) => {}
                Err(_) => {
                    log::debug!("Remove old receiver.");
                    self.receivers.remove(&event.target.function_id);
                }
            }
            return LinkProcessingResult::FINAL;
        }
        LinkProcessingResult::IGNORED
    }
}
