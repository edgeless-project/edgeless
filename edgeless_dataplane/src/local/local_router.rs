use crate::core::{DataplaneEvent, Message};
use edgeless_api::invocation::{EventData, LinkProcessingResult};
use futures::{channel::mpsc::UnboundedSender, SinkExt};
use std::collections::HashMap;
use uuid::Uuid;

pub struct NodeLocalRouter {
    pub receivers: HashMap<Uuid, UnboundedSender<DataplaneEvent>>,
}

// This is used by the remote node that is currently borrowing the `NodeLocalRouter`
#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for NodeLocalRouter {
    async fn handle(&mut self, event: edgeless_api::invocation::Event) -> edgeless_api::invocation::LinkProcessingResult {
        // if we have a function to which this event is targetted, we can send
        // it to it
        if let Some(sender) = self.receivers.get_mut(&event.target.function_id) {
            let msg = match event.data {
                EventData::Call(data) => Message::Call(data),
                EventData::Cast(data) => Message::Cast(data),
                EventData::CallRet(data) => Message::CallRet(data),
                EventData::CallNoRet => Message::CallNoRet,
                EventData::Err => Message::Err("edgeless_api invocation error".to_owned()),
            };
            // this sender is connected to the DataplaneHandle for this
            // function_id and the events sent here will be read through the
            // receive_next method
            match sender
                .send(DataplaneEvent {
                    source_id: event.source,
                    channel_id: event.stream_id,
                    message: msg.clone(),
                    created: event.created,
                })
                .await
            {
                Ok(_) => {
                    return LinkProcessingResult::FINAL;
                }
                Err(e) => {
                    log::warn!(
                        "NodeLocalRouter: Could not route to a valid DataplaneHandle (it was probably dropped). Removing it from the router. {:?}",
                        e
                    );
                    self.receivers.remove(&event.target.function_id);
                    return LinkProcessingResult::ERROR("NodeLocalRouter could not find the target DataplaneHandle".to_string());
                }
            }
        } else {
            LinkProcessingResult::IGNORED
        }
    }
}
