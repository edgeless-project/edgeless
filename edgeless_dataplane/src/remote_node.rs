// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use crate::core::*;
use crate::node_local::NodeLocalRouter;
use edgeless_api::function_instance::{ComponentId, NodeId};
use edgeless_api::invocation::InvocationAPI;

// Link allowing to send messages to a remote node using the InvocationAPI.
struct RemoteLink {
    remotes: std::sync::Arc<tokio::sync::Mutex<RemoteRouter>>,
}

#[async_trait::async_trait]
impl DataPlaneLink for RemoteLink {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::InstanceId,
        msg: Message,
        src: &edgeless_api::function_instance::InstanceId,
        created: &edgeless_api::function_instance::EventTimestamp,
        stream_id: u64,
        metadata: &edgeless_api::function_instance::EventMetadata,
    ) -> LinkProcessingResult {
        return self
            .remotes
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
                    Message::Err => edgeless_api::invocation::EventData::Err,
                },
                created: *created,
                metadata: metadata.clone(),
            })
            .await;
    }
}

pub struct RemoteRouter {
    receivers: std::collections::HashMap<NodeId, Box<dyn edgeless_api::invocation::InvocationAPI>>,
}

pub struct RemoteLinkProvider {
    own_node_id: edgeless_api::function_instance::NodeId,
    remotes: std::sync::Arc<tokio::sync::Mutex<RemoteRouter>>,
    locals: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

struct InvocationEventHandler {
    node_id: edgeless_api::function_instance::NodeId,
    locals: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for InvocationEventHandler {
    async fn handle(&mut self, event: edgeless_api::invocation::Event) -> edgeless_api::invocation::LinkProcessingResult {
        if event.target.node_id == self.node_id {
            self.locals.lock().await.handle(event).await
        } else {
            LinkProcessingResult::ERROR("Wrong Node ID".to_string())
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for RemoteRouter {
    async fn handle(&mut self, event: edgeless_api::invocation::Event) -> edgeless_api::invocation::LinkProcessingResult {
        if let Some(node_client) = self.receivers.get_mut(&event.target.node_id) {
            match node_client.handle(event).await {
                LinkProcessingResult::FINAL => return LinkProcessingResult::FINAL,
                LinkProcessingResult::IGNORED => return LinkProcessingResult::IGNORED,
                LinkProcessingResult::ERROR(e) => {
                    log::error!("Error while processing link: {:?}", e);
                    return LinkProcessingResult::ERROR(e);
                }
            }
        } else {
            // we can not process this even, ignore it
            edgeless_api::invocation::LinkProcessingResult::IGNORED
        }
    }
}

impl RemoteLinkProvider {
    pub async fn new(own_node_id: edgeless_api::function_instance::NodeId) -> Self {
        let locals = std::sync::Arc::new(tokio::sync::Mutex::new(NodeLocalRouter {
            receivers: std::collections::HashMap::<ComponentId, futures::channel::mpsc::UnboundedSender<DataplaneEvent>>::new(),
        }));

        let remotes = std::sync::Arc::new(tokio::sync::Mutex::new(RemoteRouter {
            receivers: std::collections::HashMap::new(),
        }));

        Self {
            own_node_id,
            remotes,
            locals,
        }
    }

    pub async fn new_link(
        &self,
        target: edgeless_api::function_instance::InstanceId,
        sender: futures::channel::mpsc::UnboundedSender<DataplaneEvent>,
    ) -> Box<dyn DataPlaneLink> {
        self.locals.lock().await.receivers.insert(target.function_id, sender);
        Box::new(RemoteLink {
            remotes: self.remotes.clone(),
        })
    }

    pub async fn incomming_api(&mut self) -> Box<dyn edgeless_api::invocation::InvocationAPI> {
        Box::new(InvocationEventHandler {
            node_id: self.own_node_id,
            locals: self.locals.clone(),
        })
    }

    pub async fn add_peer(&mut self, peer_id: NodeId, peer_api: Box<dyn edgeless_api::invocation::InvocationAPI>) {
        self.remotes.lock().await.receivers.insert(peer_id, peer_api);
    }

    pub async fn del_peer(&mut self, peer_id: NodeId) {
        self.remotes.lock().await.receivers.remove(&peer_id);
    }
}

#[cfg(test)]
mod test {
    use futures::SinkExt;

    use crate::remote_node::*;

    #[tokio::test]
    async fn incomming_message() {
        let node_id = uuid::Uuid::new_v4();
        let node_id_2 = uuid::Uuid::new_v4();
        let node_id_3 = uuid::Uuid::new_v4();
        let fid_target = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_source = edgeless_api::function_instance::InstanceId::new(node_id_2);
        let fid_wrong_component_id = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_wrong_node_id = edgeless_api::function_instance::InstanceId {
            node_id: node_id_3,
            function_id: fid_target.function_id,
        };
        let created = edgeless_api::function_instance::EventTimestamp::default();

        let mut provider = RemoteLinkProvider::new(node_id).await;
        let mut api = provider.incomming_api().await;

        let (sender_1, mut receiver_1) = futures::channel::mpsc::unbounded::<crate::core::DataplaneEvent>();
        provider.new_link(fid_target, sender_1).await;

        api.handle(edgeless_api::invocation::Event {
            target: fid_wrong_component_id,
            source: fid_source,
            stream_id: 0,
            data: edgeless_api::invocation::EventData::Cast("Test".to_string()),
            created: created.clone(),
            metadata: edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00015u128, 0x42a42bdecaf00016u64),
        })
        .await;

        assert!(receiver_1.try_next().is_err());

        // assert!(api
        //     .handle(edgeless_api::invocation::Event {
        //         target: fid_wrong_node_id,
        //         source: fid_source,
        //         stream_id: 0,
        //         data: edgeless_api::invocation::EventData::Cast("Test".to_string()),
        //         created: created.clone(),
        //         metadata: edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00013u128, 0x42a42bdecaf00014u64),
        //     })
        //     .await
        //     .is_err());

        assert!(receiver_1.try_next().is_err());

        let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00011u128, 0x42a42bdecaf00012u64);
        api.handle(edgeless_api::invocation::Event {
            target: fid_target,
            source: fid_source,
            stream_id: 0,
            data: edgeless_api::invocation::EventData::Cast("Test".to_string()),
            created: created.clone(),
            metadata: metad_1.clone(),
        })
        .await;

        let result = receiver_1.try_next();
        assert!(result.as_ref().is_ok_and(|o| o.is_some()));
        let result = result.unwrap().unwrap();
        assert_eq!(&metad_1, &result.metadata)
    }

    struct MockInvocationAPI {
        own_node_id: edgeless_api::function_instance::NodeId,
        events: futures::channel::mpsc::UnboundedSender<edgeless_api::invocation::Event>,
    }

    #[async_trait::async_trait]
    impl edgeless_api::invocation::InvocationAPI for MockInvocationAPI {
        async fn handle(&mut self, event: edgeless_api::invocation::Event) -> LinkProcessingResult {
            self.events.send(event.clone()).await.unwrap();
            if event.target.node_id == self.own_node_id {
                LinkProcessingResult::FINAL
            } else {
                LinkProcessingResult::IGNORED
            }
        }
    }

    #[tokio::test]
    async fn outgoing_message() {
        let node_id = uuid::Uuid::new_v4();

        let node_id_2 = uuid::Uuid::new_v4();
        let node_id_3 = uuid::Uuid::new_v4();
        let fid_source = edgeless_api::function_instance::InstanceId::new(node_id);
        let metad_source = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf0000fu128, 0x42a42bdecaf00010u64);

        let fid_target = edgeless_api::function_instance::InstanceId::new(node_id_2);
        let fid_wrong_component_id = edgeless_api::function_instance::InstanceId::new(node_id_2);
        let fid_wrong_node_id = edgeless_api::function_instance::InstanceId {
            node_id: node_id_3,
            function_id: fid_target.function_id,
        };

        let (api_sender_node_2, mut api_receiver_node_2) = futures::channel::mpsc::unbounded::<edgeless_api::invocation::Event>();
        let node_2_api: Box<dyn edgeless_api::invocation::InvocationAPI> = Box::new(MockInvocationAPI {
            own_node_id: node_id_2,
            events: api_sender_node_2,
        });
        let mut provider = RemoteLinkProvider::new(node_id).await;
        provider.add_peer(node_id_2, node_2_api).await;
        // let mut api = provider.incomming_api().await;
        let created = edgeless_api::function_instance::EventTimestamp::default();

        let (sender_1, _receiver_1) = futures::channel::mpsc::unbounded::<crate::core::DataplaneEvent>();
        let mut link = provider.new_link(fid_source, sender_1).await;

        let res = link
            .handle_send(&fid_target, Message::Cast("Test".to_string()), &fid_source, &created, 0, &metad_source)
            .await;
        assert_eq!(res, LinkProcessingResult::FINAL);
        assert!(api_receiver_node_2.try_next().unwrap().is_some());

        let res = link
            .handle_send(
                &fid_wrong_component_id,
                Message::Cast("Test".to_string()),
                &fid_source,
                &created,
                0,
                &metad_source,
            )
            .await;
        assert_eq!(res, LinkProcessingResult::FINAL);
        assert!(api_receiver_node_2.try_next().unwrap().is_some());

        let res = link
            .handle_send(
                &fid_wrong_node_id,
                Message::Cast("Test".to_string()),
                &fid_source,
                &created,
                0,
                &metad_source,
            )
            .await;
        assert_eq!(res, LinkProcessingResult::IGNORED);
        assert!(api_receiver_node_2.try_next().is_err());

        let res = link
            .handle_send(&fid_target, Message::Cast("Test".to_string()), &fid_source, &created, 0, &metad_source)
            .await;
        assert_eq!(res, LinkProcessingResult::FINAL);
        assert!(api_receiver_node_2.try_next().unwrap().is_some());
    }
}
