// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of
// Connected Mobility SPDX-FileCopyrightText: © 2023 Claudio Cicconetti
// <c.cicconetti@iit.cnr.it> SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use super::local_router::NodeLocalRouter;
use crate::core::*;
use edgeless_api::{
    function_instance::{EventTimestamp, InstanceId},
    invocation::{EventData, InvocationAPI, LinkProcessingResult},
};
use futures::channel::mpsc::UnboundedSender;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct NodeLocalLinkProvider {
    // One local_router is shared by all NodeLocalLinks
    local_router: Arc<Mutex<NodeLocalRouter>>,
}

impl Default for NodeLocalLinkProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeLocalLinkProvider {
    pub fn new() -> Self {
        Self {
            local_router: Arc::new(Mutex::new(NodeLocalRouter {
                receivers: HashMap::<Uuid, UnboundedSender<DataplaneEvent>>::new(),
            })),
        }
    }

    pub async fn new_link(&self, target: InstanceId, sender: UnboundedSender<DataplaneEvent>) -> Box<dyn DataPlaneLink> {
        // one local router is shared among all NodeLocalLinks, so we add the
        // link to the list of the receivers of that local_router; events with
        // this function_id as target will be sent on the sender channel end
        self.local_router.lock().await.receivers.insert(target.function_id, sender);
        Box::new(NodeLocalLink {
            own_node_id: target.node_id,
            // note the clone
            local_router: self.local_router.clone(),
        })
    }
}

// Link representing a component on the local node. Internally uses a table of
// link instances (NodeLocalRouter) that enqueues events based on the targeted
// function_id.
struct NodeLocalLink {
    own_node_id: Uuid,
    local_router: Arc<Mutex<NodeLocalRouter>>,
}

#[async_trait::async_trait]
impl DataPlaneLink for NodeLocalLink {
    async fn handle_cast(
        &mut self,
        target: &InstanceId,
        msg: Message,
        src: &InstanceId,
        created: &EventTimestamp,
        stream_id: u64,
    ) -> LinkProcessingResult {
        // if the cast targets the current node, we need to send it to the local
        // router to route to the appropriate function instance
        if target.node_id == self.own_node_id {
            let mut lck = self.local_router.lock().await;
            let res = lck
                .handle(edgeless_api::invocation::Event {
                    target: *target,
                    source: *src,
                    stream_id,
                    data: match msg {
                        Message::Call(data) => EventData::Call(data),
                        Message::Cast(data) => EventData::Cast(data),
                        Message::CallRet(data) => EventData::CallRet(data),
                        Message::CallNoRet => EventData::CallNoRet,
                        Message::Err(_) => EventData::Err,
                    },
                    created: *created,
                })
                .await;
            return res;
        } else {
            return LinkProcessingResult::IGNORED;
        }
    }
}

#[cfg(test)]
mod test {
    use crate::core::{DataplaneEvent, Message};

    use super::NodeLocalLinkProvider;
    use edgeless_api::{
        function_instance::{EventTimestamp, InstanceId},
        invocation::LinkProcessingResult,
    };
    use futures::channel::mpsc::unbounded;

    #[tokio::test]
    async fn basic_forwarding() {
        let node_id = uuid::Uuid::new_v4();
        let fid_1 = InstanceId::new(node_id);
        let fid_2 = InstanceId::new(node_id);
        let fid_3 = InstanceId::new(node_id);
        let ts = EventTimestamp::default();

        let provider = NodeLocalLinkProvider::new();

        let (sender_1, mut receiver_1) = unbounded::<DataplaneEvent>();
        let mut handle_1 = provider.new_link(fid_1, sender_1).await;

        let (sender_2, mut receiver_2) = unbounded::<DataplaneEvent>();
        let _handle_2 = provider.new_link(fid_2, sender_2).await;

        assert!(receiver_1.try_next().is_err());
        assert!(receiver_2.try_next().is_err());

        let ret_1 = handle_1.handle_cast(&fid_3, Message::Cast("".to_string()), &fid_1, &ts, 0).as_mut().await;

        assert_eq!(ret_1, LinkProcessingResult::IGNORED);
        assert!(receiver_1.try_next().is_err());
        assert!(receiver_2.try_next().is_err());

        let ret_2 = handle_1.handle_cast(&fid_2, Message::Cast("".to_string()), &fid_1, &ts, 0).as_mut().await;

        assert_eq!(ret_2, LinkProcessingResult::FINAL);
        assert!(receiver_1.try_next().is_err());
        assert!(receiver_2.try_next().unwrap().is_some());
    }
}
