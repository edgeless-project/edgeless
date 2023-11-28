use futures::{SinkExt, StreamExt};

use crate::core::*;
use crate::node_local::*;
use crate::remote_node::*;

/// The main handle representing an element (identified by a `InstanceId`) across the dataplane.
/// The dataplane might require multiple links which are processed in a chain-like fashion.
#[derive(Clone)]
pub struct DataplaneHandle {
    slf: edgeless_api::function_instance::InstanceId,
    receiver: std::sync::Arc<tokio::sync::Mutex<futures::channel::mpsc::UnboundedReceiver<DataplaneEvent>>>,
    output_chain: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn DataPlaneLink>>>>,
    receiver_overwrites: std::sync::Arc<tokio::sync::Mutex<TemporaryReceivers>>,
    next_id: u64,
}

impl DataplaneHandle {
    async fn new(
        receiver_id: edgeless_api::function_instance::InstanceId,
        output_chain: Vec<Box<dyn DataPlaneLink>>,
        receiver: futures::channel::mpsc::UnboundedReceiver<DataplaneEvent>,
    ) -> Self {
        let (main_sender, main_receiver) = futures::channel::mpsc::unbounded::<DataplaneEvent>();
        let receiver_overwrites = std::sync::Arc::new(tokio::sync::Mutex::new(TemporaryReceivers {
            temporary_receivers: std::collections::HashMap::new(),
        }));

        let clone_overwrites = receiver_overwrites.clone();
        // This task intercepts the messages received and routes responses towards temporary receivers while routing other events towards the main receiver used in `receive_next`.
        tokio::spawn(async move {
            let mut receiver = receiver;
            let mut main_sender = main_sender;
            loop {
                if let Some(DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                }) = receiver.next().await
                {
                    if let Some(sender) = clone_overwrites.lock().await.temporary_receivers.get_mut(&channel_id) {
                        if let Some(sender) = sender.take() {
                            match sender.send((source_id.clone(), message.clone())) {
                                Ok(_) => {
                                    continue;
                                }
                                Err(_) => {
                                    log::error!("Tried to use expired overwrite send handle.");
                                }
                            }
                        } else {
                            log::error!("Tried to use expired overwrite send handle.");
                        }
                    }
                    match main_sender
                        .send(DataplaneEvent {
                            source_id,
                            channel_id,
                            message,
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
            slf: receiver_id,
            receiver: std::sync::Arc::new(tokio::sync::Mutex::new(main_receiver)),
            output_chain: std::sync::Arc::new(tokio::sync::Mutex::new(output_chain)),
            receiver_overwrites,
            next_id: 1,
        }
    }

    /// Main receive function for receiving the next cast or call event.
    /// This is NOT used for processing replies to return values.
    pub async fn receive_next(&mut self) -> DataplaneEvent {
        loop {
            if let Some(DataplaneEvent {
                source_id,
                channel_id,
                message,
            }) = self.receiver.lock().await.next().await
            {
                if std::mem::discriminant(&message) == std::mem::discriminant(&Message::Cast("".to_string()))
                    || std::mem::discriminant(&message) == std::mem::discriminant(&Message::Call("".to_string()))
                {
                    return DataplaneEvent {
                        source_id,
                        channel_id,
                        message,
                    };
                }
                log::error!("Unprocesses other message");
            }
        }
    }

    /// Send a `cast` event.
    pub async fn send(&mut self, target: edgeless_api::function_instance::InstanceId, msg: String) {
        self.send_inner(target, Message::Cast(msg), 0).await;
    }

    // Send a `call` event and wait for the return event.
    // Internally, this sets up a receiver override to handle the message before it would be sent to the `receive_next` function.
    pub async fn call(&mut self, target: edgeless_api::function_instance::InstanceId, msg: String) -> CallRet {
        let (send, rec) = futures::channel::oneshot::channel::<(edgeless_api::function_instance::InstanceId, Message)>();
        let channel_id = self.next_id;
        self.next_id += 1;
        self.receiver_overwrites.lock().await.temporary_receivers.insert(channel_id, Some(send));
        self.send_inner(target, Message::Call(msg), channel_id).await;
        match rec.await {
            Ok((_src, msg)) => match msg {
                Message::CallRet(ret) => CallRet::Reply(ret),
                Message::CallNoRet => CallRet::NoReply,
                _ => CallRet::Err,
            },
            Err(_) => CallRet::Err,
        }
    }

    // Reply to a `call` event using the `channel_id` used to send the request.
    pub async fn reply(&mut self, target: edgeless_api::function_instance::InstanceId, channel_id: u64, msg: CallRet) {
        self.send_inner(
            target,
            match msg {
                CallRet::Reply(msg) => Message::CallRet(msg),
                CallRet::NoReply => Message::CallNoRet,
                CallRet::Err => Message::Err,
            },
            channel_id,
        )
        .await;
    }

    async fn send_inner(&mut self, target: edgeless_api::function_instance::InstanceId, msg: Message, channel_id: u64) {
        let mut lck = self.output_chain.lock().await;
        for link in &mut lck.iter_mut() {
            match link.handle_send(&target, msg.clone(), &self.slf, channel_id).await {
                LinkProcessingResult::FINAL => {
                    return;
                }
                _ => {}
            }
        }
        log::info!("Unprocessed Message: {:?}->{:?}", self.slf, target);
    }
}

struct TemporaryReceivers {
    temporary_receivers:
        std::collections::HashMap<u64, Option<futures::channel::oneshot::Sender<(edgeless_api::function_instance::InstanceId, Message)>>>,
}

#[derive(Clone)]
pub struct DataplaneProvider {
    local_provider: std::sync::Arc<tokio::sync::Mutex<NodeLocalLinkProvider>>,
    remote_provider: std::sync::Arc<tokio::sync::Mutex<RemoteLinkProvider>>,
}

impl DataplaneProvider {
    pub async fn new(node_id: uuid::Uuid, invocation_url: String) -> Self {
        let remote_provider = std::sync::Arc::new(tokio::sync::Mutex::new(
            RemoteLinkProvider::new(node_id, std::collections::HashMap::new()).await,
        ));

        let (_, _, port) = edgeless_api::util::parse_http_host(&invocation_url.clone()).unwrap();

        let clone_provider = remote_provider.clone();
        let _server = tokio::spawn(edgeless_api::grpc_impl::invocation::InvocationAPIServer::run(
            clone_provider.lock().await.incomming_api().await,
            invocation_url,
        ));

        log::info!("coap port {}", port);

        let _coap_server = tokio::spawn(edgeless_api::coap_impl::CoapInvocationServer::run(
            clone_provider.lock().await.incomming_api().await,
            std::net::SocketAddrV4::new("0.0.0.0".parse().unwrap(), port),
        ));

        Self {
            local_provider: std::sync::Arc::new(tokio::sync::Mutex::new(NodeLocalLinkProvider::new())),
            remote_provider,
        }
    }

    pub async fn get_handle_for(&mut self, target: edgeless_api::function_instance::InstanceId) -> DataplaneHandle {
        let (sender, receiver) = futures::channel::mpsc::unbounded::<DataplaneEvent>();
        let output_chain = vec![
            self.local_provider.lock().await.new_link(target.clone(), sender.clone()).await,
            self.remote_provider.lock().await.new_link(target.clone(), sender.clone()).await,
        ];
        DataplaneHandle::new(target, output_chain, receiver).await
    }

    pub async fn add_peer(&mut self, peer: EdgelessDataplanePeerSettings) {
        self.remote_provider
            .lock()
            .await
            .add_peer(peer.node_id, Self::connect_peer(&peer).await)
            .await;
    }

    pub async fn del_peer(&mut self, node_id: uuid::Uuid) {
        self.remote_provider.lock().await.del_peer(node_id).await;
    }

    async fn connect_peer(target: &EdgelessDataplanePeerSettings) -> Box<dyn edgeless_api::invocation::InvocationAPI> {
        let (proto, url, port) = edgeless_api::util::parse_http_host(&target.invocation_url).unwrap();
        match proto {
            edgeless_api::util::Proto::COAP => {
                Box::new(edgeless_api::coap_impl::CoapClient::new(std::net::SocketAddrV4::new(url.parse().unwrap(), port)).await)
            }
            _ => Box::new(edgeless_api::grpc_impl::invocation::InvocationAPIClient::new(&target.invocation_url).await),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::handle::*;

    #[tokio::test]
    async fn local_normal_path() {
        let node_id = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id.clone());
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id.clone());

        let mut provider = DataplaneProvider::new(node_id, "http://127.0.0.1:7096".to_string()).await;

        let mut handle_1 = provider.get_handle_for(fid_1.clone()).await;
        let mut handle_2 = provider.get_handle_for(fid_2.clone()).await;

        handle_1.send(fid_2, "Test".to_string()).await;

        let res = handle_2.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&res.message),
            std::mem::discriminant(&crate::core::Message::Cast("".to_string()))
        );
    }

    #[tokio::test]
    async fn local_call_with_return() {
        let node_id = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id.clone());
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id.clone());

        let mut provider = DataplaneProvider::new(node_id, "http://127.0.0.1:7097".to_string()).await;

        let mut handle_1 = provider.get_handle_for(fid_1.clone()).await;
        let mut handle_2 = provider.get_handle_for(fid_2.clone()).await;

        let return_handle = tokio::spawn(async move { handle_1.call(fid_2, "Test".to_string()).await });

        let req = handle_2.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&req.message),
            std::mem::discriminant(&crate::core::Message::Call("".to_string()))
        );

        handle_2.reply(req.source_id, req.channel_id, CallRet::NoReply).await;

        let repl = return_handle.await.unwrap();
        assert_eq!(std::mem::discriminant(&CallRet::NoReply), std::mem::discriminant(&repl));
    }

    #[tokio::test]
    async fn grpc_impl_e2e() {
        let node_id = uuid::Uuid::new_v4();
        let node_id_2 = uuid::Uuid::new_v4();
        let fid_1 = edgeless_api::function_instance::InstanceId::new(node_id.clone());
        let fid_2 = edgeless_api::function_instance::InstanceId::new(node_id_2.clone());

        let provider1_f = tokio::spawn(async move {
            let mut dataplane = DataplaneProvider::new(node_id.clone(), "http://127.0.0.1:7099".to_string()).await;
            dataplane
                .add_peer(EdgelessDataplanePeerSettings {
                    node_id: node_id_2.clone(),
                    invocation_url: "http://127.0.0.1:7098".to_string(),
                })
                .await;
            dataplane
        });

        let provider2_f = tokio::spawn(async move {
            let mut dataplane = DataplaneProvider::new(node_id_2.clone(), "http://127.0.0.1:7098".to_string()).await;
            dataplane
                .add_peer(EdgelessDataplanePeerSettings {
                    node_id: node_id.clone(),
                    invocation_url: "http://127.0.0.1:7099".to_string(),
                })
                .await;
            dataplane
        });

        // This test got stuck during initial testing. I suspect that this was due to the use of common ports across the testsuite
        // but the timeouts should prevent it from blocking the entire testsuite if that was not the reason (timeout will lead to failure).
        let (provider_1_r, provider_2_r) = futures::join!(
            tokio::time::timeout(tokio::time::Duration::from_secs(5), provider1_f),
            tokio::time::timeout(tokio::time::Duration::from_secs(5), provider2_f)
        );
        let mut provider_1 = provider_1_r.unwrap().unwrap();
        let mut provider_2 = provider_2_r.unwrap().unwrap();

        let mut handle_1 = provider_1.get_handle_for(fid_1.clone()).await;
        let mut handle_2 = provider_2.get_handle_for(fid_2.clone()).await;

        handle_1.send(fid_2.clone(), "Test".to_string()).await;
        let cast_req = handle_2.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&cast_req.message),
            std::mem::discriminant(&crate::core::Message::Cast("".to_string()))
        );

        let cloned_id_1 = fid_1.clone();
        let mut cloned_handle_2 = handle_2.clone();

        let return_handle = tokio::spawn(async move { cloned_handle_2.call(cloned_id_1, "Test".to_string()).await });

        let call_req = handle_1.receive_next().await;
        assert_eq!(
            std::mem::discriminant(&call_req.message),
            std::mem::discriminant(&crate::core::Message::Call("".to_string()))
        );
        handle_1.reply(call_req.source_id, call_req.channel_id, CallRet::NoReply).await;

        let repl = return_handle.await.unwrap();
        assert_eq!(std::mem::discriminant(&CallRet::NoReply), std::mem::discriminant(&repl));
    }
}
