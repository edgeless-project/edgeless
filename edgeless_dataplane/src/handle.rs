// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of
// Connected Mobility SPDX-FileCopyrightText: © 2023 Claudio Cicconetti
// <c.cicconetti@iit.cnr.it> SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use crate::core::*;
use crate::local::local_link::NodeLocalLinkProvider;
use crate::remote::remote_link::RemoteLinkProvider;
use edgeless_api::coap_impl::invocation::CoapInvocationServer;
use edgeless_api::function_instance::{self, InstanceId};
use edgeless_api::grpc_impl::invocation::InvocationAPIServer;
use edgeless_api::invocation::LinkProcessingResult;
use edgeless_api::util::parse_http_host;
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::channel::oneshot::Sender;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct DataplaneProvider {
    // manages communication with local function instances
    local_provider: Arc<Mutex<NodeLocalLinkProvider>>,
    // manages communication with remote peers
    remote_provider: Arc<Mutex<RemoteLinkProvider>>,
}

impl DataplaneProvider {
    pub async fn new(node_id: Uuid, invocation_url: String, invocation_url_coap: Option<String>) -> Self {
        let remote_provider = Arc::new(Mutex::new(RemoteLinkProvider::new(node_id).await));
        let (_, _, port) = parse_http_host(&invocation_url.clone()).unwrap();
        let remote_provider_clone = remote_provider.clone();

        // DataplaneProvider hosts the InvocationAPI server. RemoteLinkProvider
        // implements the InvocationAPI interface and is called whenever an
        // invocation is received by this InvocationAPI server.
        let _grpc_server = tokio::spawn(InvocationAPIServer::run(
            remote_provider_clone.lock().await.incoming_api().await,
            invocation_url,
        ));

        // COAP is used for edgeless embedded
        if let Some(invocation_url_coap) = invocation_url_coap {
            let (_, coap_ip, coap_port) = parse_http_host(&invocation_url_coap.clone()).unwrap();
            log::info!("Start COAP Invocation Server {}:{}", coap_ip, port);

            let _coap_server = tokio::spawn(CoapInvocationServer::run(
                remote_provider_clone.lock().await.incoming_api().await,
                std::net::SocketAddrV4::new(coap_ip.parse().unwrap(), coap_port),
            ));
        }

        Self {
            local_provider: Arc::new(Mutex::new(NodeLocalLinkProvider::new())),
            remote_provider,
        }
    }

    // Gets a handle for a given function instance. This means, that this handle
    // can be used by this instance to communicate with other instances in the
    // cluster (using cast, call, reply) and enables other instances to
    // communicate with this instance (incoming Dataplane Events can be received
    // on receive_next).
    pub async fn get_handle_for(&mut self, instance_id: InstanceId) -> DataplaneHandle {
        let (sender, receiver) = unbounded::<DataplaneEvent>();
        // the default output_chain consist of a local link and a remote link
        let output_chain = vec![
            // local link to communicate with other instances on the same node
            self.local_provider.lock().await.new_link(instance_id, sender.clone()).await,
            // remote link to communicate with other instances on different nodes
            self.remote_provider.lock().await.new_link(instance_id, sender.clone()).await,
        ];
        DataplaneHandle::new(instance_id, output_chain, receiver).await
    }

    /// Since dataplane is a full-mesh, when the node-agent receives an
    /// UpdatePeers request it adds them to the RemoteProvider, which in turn
    /// makes them available to all function instances.
    pub async fn add_peer(&mut self, peer: EdgelessDataplanePeerSettings) {
        log::debug!("add_peer: {}", peer.node_id);
        self.remote_provider
            .lock()
            .await
            .add_peer(peer.node_id, Self::connect_to_peer(&peer).await)
            .await;
    }

    pub async fn del_peer(&mut self, node_id: Uuid) {
        log::debug!("del_peer: {}", node_id);
        self.remote_provider.lock().await.del_peer(node_id).await;
    }

    // Connects to a remote peer either over grpc or coap
    async fn connect_to_peer(peer: &EdgelessDataplanePeerSettings) -> Box<dyn edgeless_api::invocation::InvocationAPI> {
        let (proto, url, port) = parse_http_host(&peer.invocation_url).unwrap();

        let client: Box<dyn edgeless_api::invocation::InvocationAPI> = match proto {
            edgeless_api::util::Proto::COAP => {
                Box::new(edgeless_api::coap_impl::CoapClient::new(std::net::SocketAddrV4::new(url.parse().unwrap(), port)).await)
            }
            _ => Box::new(edgeless_api::grpc_impl::invocation::InvocationAPIClient::new(&peer.invocation_url).await),
        };
        return client;
    }
}

fn timestamp_utc() -> function_instance::EventTimestamp {
    let now = chrono::Utc::now();
    function_instance::EventTimestamp {
        secs: now.timestamp(),
        nsecs: now.timestamp_subsec_nanos(),
    }
}
struct TemporaryReceivers {
    temporary_receivers: HashMap<u64, Sender<(InstanceId, Message)>>,
}
/// The main handle representing an element (identified by a `InstanceId`)
/// across the dataplane. The dataplane might require multiple links which are
/// processed in a chain-like fashion.
#[derive(Clone)]
pub struct DataplaneHandle {
    // an handle is owned by a function instance / balancer / other component
    // connected to the dataplane
    handle_owner: InstanceId,
    receiver: Arc<Mutex<UnboundedReceiver<DataplaneEvent>>>,
    // output_chain is iterated upon to send a cast from this instance
    // (handle_owner) to other instances either local or remote
    output_chain: Arc<Mutex<Vec<Box<dyn DataPlaneLink>>>>,
    receiver_overwrites: Arc<Mutex<TemporaryReceivers>>,
    next_id: u64,
}

impl DataplaneHandle {
    async fn new(receiver_id: InstanceId, output_chain: Vec<Box<dyn DataPlaneLink>>, receiver: UnboundedReceiver<DataplaneEvent>) -> Self {
        let (main_sender, main_receiver) = unbounded::<DataplaneEvent>();
        // receiver overwrites are used to implement Call over Casts - replies
        // are routed to them
        let receiver_overwrites = Arc::new(Mutex::new(TemporaryReceivers {
            temporary_receivers: HashMap::new(),
        }));
        let clone_overwrites = receiver_overwrites.clone();

        // This task intercepts the messages received and routes responses
        // towards temporary receivers while routing other events towards the
        // main receiver used in `receive_next`.
        tokio::spawn(async move {
            let mut receiver = receiver;
            let mut main_sender = main_sender;
            loop {
                if let Some(DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                    created,
                    metadata,
                }) = receiver.next().await
                {
                    // if there's a temporary receiver defined for this event,
                    // route it there. Temporarty receivers are used to
                    // implement calls.
                    if let Some(sender) = clone_overwrites.lock().await.temporary_receivers.remove(&channel_id) {
                        match sender.send((source_id, message.clone())) {
                            Ok(_) => {
                                continue;
                            }
                            Err(_) => {
                                log::error!("Tried to use expired overwrite send handle.");
                            }
                        }
                    }
                    // otherwise do the normal path (for all other events)
                    match main_sender
                        .send(DataplaneEvent {
                            source_id,
                            channel_id,
                            message,
                            created,
                            metadata,
                        })
                        .await
                    {
                        Ok(_) => {}
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
        });

        DataplaneHandle {
            handle_owner: receiver_id,
            receiver: Arc::new(Mutex::new(main_receiver)),
            output_chain: Arc::new(Mutex::new(output_chain)),
            receiver_overwrites,
            next_id: 1,
        }
    }

    /// Main receive function for receiving the next cast or call event. This is
    /// NOT used for processing replies to return values.
    pub async fn receive_next(&mut self) -> DataplaneEvent {
        loop {
            if let Some(DataplaneEvent {
                source_id,
                channel_id,
                message,
                created,
                metadata,
            }) = self.receiver.lock().await.next().await
            {
                if std::mem::discriminant(&message) == std::mem::discriminant(&Message::Cast("".to_string()))
                    || std::mem::discriminant(&message) == std::mem::discriminant(&Message::Call("".to_string()))
                {
                    return DataplaneEvent {
                        source_id,
                        channel_id,
                        message,
                        created,
                        metadata,
                    };
                } else {
                    log::warn!("Unknown DataplaneEvent")
                }
            }
        }
    }

    /// Send a `cast` event.
    pub async fn send(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        msg: String,
        metadata: &edgeless_api::function_instance::EventMetadata,
    ) {
        self.send_inner(target, Message::Cast(msg), timestamp_utc(), 0, metadata).await;
    }

    // Send a `call` event and wait for the return event.
    // Internally, this sets up a receiver override to handle the message before it would be sent to the `receive_next` function.
    pub async fn call(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        msg: String,
        metadata: &edgeless_api::function_instance::EventMetadata,
    ) -> CallRet {
        let (sender, receiver) = futures::channel::oneshot::channel::<(edgeless_api::function_instance::InstanceId, Message)>();
        let channel_id = self.next_id;
        self.next_id += 1;

        // Potential Leak: This is only received if a message is received (or
        // the handle is dropped)
        // TODO: fix it by specifying a timeout after which this receiver is garbage collected
        self.receiver_overwrites.lock().await.temporary_receivers.insert(channel_id, sender);
        self.send_inner(target, Message::Call(msg), timestamp_utc(), channel_id, metadata).await;
        match receiver.await {
            Ok((_src, msg)) => match msg {
                Message::CallRet(ret) => CallRet::Reply(ret),
                Message::CallNoRet => CallRet::NoReply,
                Message::Err(err_msg) => CallRet::Err(err_msg),
                _ => CallRet::Err("incompatible response to a call (cast or call)".to_owned()),
            },
            Err(_) => CallRet::Err("Cancelled receiver for call".to_owned()),
        }
    }

    // Reply to a `call` event using the `channel_id` used to send the request.
    pub async fn reply(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        channel_id: u64,
        msg: CallRet,
        metadata: &edgeless_api::function_instance::EventMetadata,
    ) {
        self.send_inner(
            target,
            match msg {
                CallRet::Reply(msg) => Message::CallRet(msg),
                CallRet::NoReply => Message::CallNoRet,
                CallRet::Err(err_msg) => Message::Err(err_msg),
            },
            function_instance::EventTimestamp::default(),
            channel_id,
            metadata,
        )
        .await;
    }

    async fn send_inner(
        &mut self,
        target: edgeless_api::function_instance::InstanceId,
        msg: Message,
        created: edgeless_api::function_instance::EventTimestamp,
        channel_id: u64,
        metadata: &edgeless_api::function_instance::EventMetadata,
    ) {
        log::info!("send_inner");
        let mut lck = self.output_chain.lock().await;
        for link in &mut lck.iter_mut() {
            match link.handle_cast(&target, msg.clone(), &self.handle_owner, &created, channel_id).await {
                LinkProcessingResult::FINAL => {
                    return;
                }
                LinkProcessingResult::IGNORED => {
                    log::debug!("{:?}: event ignored", msg);
                }
                LinkProcessingResult::ERROR(e) => {
                    log::error!("{:?}: error while handling an outgoing event: {}", msg, e)
                }
            }
        }
        // TODO: at least one link must send a final, otherwise the dataplane
        // event was not delivered successfully. If that's the case, immediately
        // deliver on the receiver and close it. needs to be finally decided and
        // implemented in a consistent manner
    }
}

#[cfg(test)]
mod test {
    use crate::handle::*;

    #[tokio::test]
    async fn local_normal_path() {
        let node_id = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id);
        let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00020u128, 0x42a42bdecaf00021u64);

        let mut provider = DataplaneProvider::new(node_id, "http://127.0.0.1:7096".to_string(), None).await;

        let mut handle_1 = provider.get_handle_for(fid_1).await;
        let mut handle_2 = provider.get_handle_for(fid_2).await;

        handle_1.send(fid_2, "Test".to_string(), &metad_1).await;

        let res = handle_2.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&res.message),
            std::mem::discriminant(&crate::core::Message::Cast("".to_string()))
        );

        assert_eq!(
            &res.metadata, &metad_1,
            "Handle 2 must receive the same metadata given by its parent through handle 1"
        );
    }

    #[tokio::test]
    async fn local_call_with_return() {
        let node_id = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id);
        let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf0001du128, 0x42a42bdecaf0001eu64);

        let mut provider = DataplaneProvider::new(node_id, "http://127.0.0.1:7097".to_string(), None).await;

        let mut handle_1 = provider.get_handle_for(fid_1).await;
        let mut handle_2 = provider.get_handle_for(fid_2).await;

        let return_handle = {
            let metad_1_cp = metad_1.clone();
            tokio::spawn(async move { handle_1.call(fid_2, "Test".to_string(), &metad_1_cp).await })
        };

        let req = handle_2.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&req.message),
            std::mem::discriminant(&crate::core::Message::Call("".to_string()))
        );
        assert_eq!(
            &req.metadata, &metad_1,
            "Handle 2 must receive the same metadata given by its parent through handle 1"
        );

        handle_2.reply(req.source_id, req.channel_id, CallRet::NoReply, &req.metadata).await;

        let repl = return_handle.await.unwrap();
        assert_eq!(std::mem::discriminant(&CallRet::NoReply), std::mem::discriminant(&repl));
    }

    #[tokio::test]
    async fn grpc_impl_e2e() {
        let node_id = uuid::Uuid::new_v4();
        let node_id_2 = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id);
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id_2);
        let metad_1 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf0001bu128, 0x42a42bdecaf0001cu64);
        let metad_2 = edgeless_api::function_instance::EventMetadata::from_uints(0x42a42bdecaf00019u128, 0x42a42bdecaf0001au64);

        let provider1_f = tokio::spawn(async move {
            let mut dataplane = DataplaneProvider::new(node_id, "http://127.0.0.1:7099".to_string(), None).await;
            dataplane
                .add_peer(EdgelessDataplanePeerSettings {
                    node_id: node_id_2,
                    invocation_url: "http://127.0.0.1:7098".to_string(),
                })
                .await;
            dataplane
        });

        let provider2_f = tokio::spawn(async move {
            let mut dataplane = DataplaneProvider::new(node_id_2, "http://127.0.0.1:7098".to_string(), None).await;
            dataplane
                .add_peer(EdgelessDataplanePeerSettings {
                    node_id,
                    invocation_url: "http://127.0.0.1:7099".to_string(),
                })
                .await;
            dataplane
        });

        // This test got stuck during initial testing. I suspect that this was
        // due to the use of common ports across the testsuite but the timeouts
        // should prevent it from blocking the entire testsuite if that was not
        // the reason (timeout will lead to failure).
        let (provider_1_r, provider_2_r) = futures::join!(
            tokio::time::timeout(tokio::time::Duration::from_secs(5), provider1_f),
            tokio::time::timeout(tokio::time::Duration::from_secs(5), provider2_f)
        );
        let mut provider_1 = provider_1_r.unwrap().unwrap();
        let mut provider_2 = provider_2_r.unwrap().unwrap();

        let mut handle_1 = provider_1.get_handle_for(fid_1).await;
        let mut handle_2 = provider_2.get_handle_for(fid_2).await;

        handle_1.send(fid_2, "Test".to_string(), &metad_1).await;
        let cast_req = handle_2.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&cast_req.message),
            std::mem::discriminant(&crate::core::Message::Cast("".to_string()))
        );
        assert_eq!(&cast_req.metadata, &metad_1);

        let cloned_id_1 = fid_1;
        let mut cloned_handle_2 = handle_2.clone();

        let return_handle = {
            let cloned_metad_2 = metad_2.clone();
            tokio::spawn(async move { cloned_handle_2.call(cloned_id_1, "Test".to_string(), &cloned_metad_2).await })
        };

        let call_req = handle_1.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&call_req.message),
            std::mem::discriminant(&crate::core::Message::Call("".to_string()))
        );
        assert_eq!(&call_req.metadata, &metad_2);
        handle_1
            .reply(call_req.source_id, call_req.channel_id, CallRet::NoReply, &call_req.metadata)
            .await;

        let repl = return_handle.await.unwrap();
        assert_eq!(std::mem::discriminant(&CallRet::NoReply), std::mem::discriminant(&repl));
    }
}
