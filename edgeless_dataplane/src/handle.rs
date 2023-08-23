use futures::{SinkExt, StreamExt};

use crate::core::*;
use crate::node_local::*;
use crate::remote_node::*;

/// The main handle representing an element (identified by a `FunctionId`) across the dataplane.
/// The dataplane might require multiple links which are processed in a chain-like fashion.
#[derive(Clone)]
pub struct DataplaneHandle {
    slf: edgeless_api::function_instance::FunctionId,
    receiver: std::sync::Arc<tokio::sync::Mutex<futures::channel::mpsc::UnboundedReceiver<DataplaneEvent>>>,
    output_chain: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn DataPlaneLink>>>>,
    receiver_overwrites: std::sync::Arc<tokio::sync::Mutex<TemporaryReceivers>>,
    next_id: u64,
}

impl DataplaneHandle {
    async fn new(
        receiver_id: edgeless_api::function_instance::FunctionId,
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
    pub async fn send(&mut self, target: edgeless_api::function_instance::FunctionId, msg: String) {
        self.send_inner(target, Message::Cast(msg), 0).await;
    }

    // Send a `call` event and wait for the return event.
    // Internally, this sets up a receiver override to handle the message before it would be sent to the `receive_next` function.
    pub async fn call(&mut self, target: edgeless_api::function_instance::FunctionId, msg: String) -> CallRet {
        let (send, rec) = futures::channel::oneshot::channel::<(edgeless_api::function_instance::FunctionId, Message)>();
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
    pub async fn reply(&mut self, target: edgeless_api::function_instance::FunctionId, channel_id: u64, msg: CallRet) {
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

    async fn send_inner(&mut self, target: edgeless_api::function_instance::FunctionId, msg: Message, channel_id: u64) {
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
        std::collections::HashMap<u64, Option<futures::channel::oneshot::Sender<(edgeless_api::function_instance::FunctionId, Message)>>>,
}

#[derive(Clone)]
pub struct DataplaneProvider {
    local_provider: std::sync::Arc<tokio::sync::Mutex<NodeLocalLinkProvider>>,
    remote_provider: std::sync::Arc<tokio::sync::Mutex<RemoteLinkProvider>>,
}

impl DataplaneProvider {
    pub async fn new(_node_id: uuid::Uuid, invocation_url: String, peers: Vec<EdgelessDataplanePeerSettings>) -> Self {
        Self {
            local_provider: std::sync::Arc::new(tokio::sync::Mutex::new(NodeLocalLinkProvider::new())),
            remote_provider: std::sync::Arc::new(tokio::sync::Mutex::new(
                RemoteLinkProvider::new(
                    invocation_url,
                    peers
                        .iter()
                        .map(|peer_conf| (peer_conf.id, peer_conf.invocation_url.to_string()))
                        .collect(),
                )
                .await, // RemoteLinkProvider::new(std::collections::HashMap::from([(node_id, "http://127.0.0.1:7002".to_string())])).await,
            )),
        }
    }

    pub async fn get_handle_for(&mut self, target: edgeless_api::function_instance::FunctionId) -> DataplaneHandle {
        let (sender, receiver) = futures::channel::mpsc::unbounded::<DataplaneEvent>();
        let output_chain = vec![
            self.local_provider.lock().await.new_link(target.clone(), sender.clone()).await,
            self.remote_provider.lock().await.new_link(target.clone(), sender.clone()).await,
        ];
        DataplaneHandle::new(target, output_chain, receiver).await
    }
}
