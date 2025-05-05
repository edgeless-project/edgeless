// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
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
        target: &edgeless_api::function_instance::InstanceId,
        msg: Message,
        src: &edgeless_api::function_instance::InstanceId,
        created: &edgeless_api::function_instance::EventTimestamp,
        stream_id: u64,
        metadata: &edgeless_api::function_instance::EventMetadata,
    ) -> LinkProcessingResult {
        if target.node_id == self.node_id {
            return self
                .router
                .lock()
                .await
                .handle(edgeless_api::invocation::Event {
                    target: *target,
                    source: *src,
                    stream_id,
                    data: match msg {
                        Message::Call(data) => edgeless_api::invocation::EventData::Call(data),
                        Message::Cast(data) => edgeless_api::invocation::EventData::Cast(data),
                        Message::CallRet(data) => edgeless_api::invocation::EventData::CallRet(data),
                        Message::CallNoRet => edgeless_api::invocation::EventData::CallNoRet,
                        Message::Err(_) => edgeless_api::invocation::EventData::Err,
                    },
                    created: *created,
                    metadata: metadata.clone(),
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
    async fn handle(&mut self, event: edgeless_api::invocation::Event) -> anyhow::Result<edgeless_api::invocation::LinkProcessingResult> {
        if let Some(sender) = self.receivers.get_mut(&event.target.function_id) {
            let msg = match event.data {
                edgeless_api::invocation::EventData::Call(data) => Message::Call(data),
                edgeless_api::invocation::EventData::Cast(data) => Message::Cast(data),
                edgeless_api::invocation::EventData::CallRet(data) => Message::CallRet(data),
                edgeless_api::invocation::EventData::CallNoRet => Message::CallNoRet,
                edgeless_api::invocation::EventData::Err => Message::Err("edgeless_api invocation error".to_owned()),
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
            return Ok(LinkProcessingResult::FINAL);
        }
        Ok(LinkProcessingResult::PASSED)
    }
}

pub struct NodeLocalLinkProvider {
    router: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

impl Default for NodeLocalLinkProvider {
    fn default() -> Self {
        Self::new()
    }
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
        target: edgeless_api::function_instance::InstanceId,
        sender: futures::channel::mpsc::UnboundedSender<DataplaneEvent>,
    ) -> Box<dyn DataPlaneLink> {
        self.router.lock().await.receivers.insert(target.function_id, sender);
        Box::new(NodeLocalLink {
            node_id: target.node_id,
            router: self.router.clone(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::NodeLocalLinkProvider;

    #[tokio::test]
    async fn basic_forwarding() {
        let node_id = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_3 = edgeless_api::function_instance::InstanceId::new(node_id);
        let ts = edgeless_api::function_instance::EventTimestamp::default();
        let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00017u128, 0x42a42bdecaf00018u64);

        let provider = NodeLocalLinkProvider::new();

        let (sender_1, mut receiver_1) = futures::channel::mpsc::unbounded::<crate::core::DataplaneEvent>();
        let mut handle_1 = provider.new_link(fid_1, sender_1).await;

        let (sender_2, mut receiver_2) = futures::channel::mpsc::unbounded::<crate::core::DataplaneEvent>();
        let _handle_2 = provider.new_link(fid_2, sender_2).await;

        assert!(receiver_1.try_next().is_err());
        assert!(receiver_2.try_next().is_err());

        let ret_1 = handle_1
            .handle_send(&fid_3, crate::core::Message::Cast("".to_string()), &fid_1, &ts, 0, &metad_1)
            .as_mut()
            .await;

        assert_eq!(ret_1, crate::core::LinkProcessingResult::PASSED);
        assert!(receiver_1.try_next().is_err());
        assert!(receiver_2.try_next().is_err());

        let ret_2 = handle_1
            .handle_send(&fid_2, crate::core::Message::Cast("".to_string()), &fid_1, &ts, 0, &metad_1)
            .as_mut()
            .await;

        assert_eq!(ret_2, crate::core::LinkProcessingResult::FINAL);
        assert!(receiver_1.try_next().is_err());
        let result = {
            let tmp = receiver_2.try_next();
            assert!(tmp.is_ok());
            assert!(tmp.as_ref().unwrap().is_some());
            tmp.unwrap().unwrap()
        };
        assert_eq!(metad_1, result.metadata)
    }
}
