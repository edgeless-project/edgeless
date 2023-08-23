use crate::core::*;
use edgeless_api::invocation::InvocationAPI;
use futures::SinkExt;

// Link representing a component on the local node.
// Internally uses a table if link instances (NodeLocalRouter) that enqueues events based on the targeted function_id.
struct NodeLocalLink {
    node_id: uuid::Uuid,
    router: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

#[async_trait::async_trait]
impl DataPlaneLink for NodeLocalLink {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::FunctionId,
        msg: Message,
        src: &edgeless_api::function_instance::FunctionId,
        stream_id: u64,
    ) -> LinkProcessingResult {
        if target.node_id == self.node_id {
            return self
                .router
                .lock()
                .await
                .handle_event(edgeless_api::invocation::Event {
                    target: target.clone(),
                    source: src.clone(),
                    stream_id,
                    data: match msg {
                        Message::Call(data) => edgeless_api::invocation::EventData::Call(data),
                        Message::Cast(data) => edgeless_api::invocation::EventData::Cast(data),
                        Message::CallRet(data) => edgeless_api::invocation::EventData::CallRet(data),
                        Message::CallNoRet => edgeless_api::invocation::EventData::CallNoRet,
                        Message::Err => edgeless_api::invocation::EventData::Err,
                    },
                })
                .await
                .unwrap();
        } else {
            return LinkProcessingResult::PASSED;
        }
    }
}

pub struct NodeLocalRouter {
    pub receivers: std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedSender<DataplaneEvent>>,
}

// This is used by the remote node that is currently borrowing the `NodeLocalRouter`
#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for NodeLocalRouter {
    async fn handle_event(&mut self, event: edgeless_api::invocation::Event) -> anyhow::Result<edgeless_api::invocation::LinkProcessingResult> {
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
                    source_id: event.source.clone(),
                    channel_id: event.stream_id.clone(),
                    message: msg,
                })
                .await
            {
                Ok(_) => {}
                Err(_) => {
                    log::debug!("Remove old receiver.");
                    self.receivers.remove(&event.target.function_id);
                }
            }
            return Ok(LinkProcessingResult::FINAL);
        }
        Ok(LinkProcessingResult::PASSED)
    }
}

pub struct NodeLocalLinkProvider {
    router: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

impl NodeLocalLinkProvider {
    pub fn new() -> Self {
        Self {
            router: std::sync::Arc::new(tokio::sync::Mutex::new(NodeLocalRouter {
                receivers: std::collections::HashMap::<uuid::Uuid, futures::channel::mpsc::UnboundedSender<DataplaneEvent>>::new(),
            })),
        }
    }

    pub async fn new_link(
        &self,
        target: edgeless_api::function_instance::FunctionId,
        sender: futures::channel::mpsc::UnboundedSender<DataplaneEvent>,
    ) -> Box<dyn DataPlaneLink> {
        self.router.lock().await.receivers.insert(target.function_id.clone(), sender);
        Box::new(NodeLocalLink {
            node_id: target.node_id.clone(),
            router: self.router.clone(),
        })
    }
}
