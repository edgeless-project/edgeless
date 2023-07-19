use edgeless_api::invocation::{InvocationAPI, LinkProcessingResult};
use futures::{SinkExt, StreamExt};

#[async_trait::async_trait]
trait DataPlaneLink: Send + Sync {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::FunctionId,
        msg: Message,
        src: &edgeless_api::function_instance::FunctionId,
        stream_id: u64,
    ) -> LinkProcessingResult;
}

struct NodeLocalLink {
    node_id: uuid::Uuid,
    router: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

#[derive(Clone, Debug)]
enum Message {
    Cast(String),
    Call(String),
    CallRet(String),
    CallNoRet,
    Err,
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

struct NodeLocalRouter {
    receivers:
        std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedSender<(edgeless_api::function_instance::FunctionId, u64, Message)>>,
}

struct RemoteRouter {
    receivers: std::collections::HashMap<uuid::Uuid, Box<dyn edgeless_api::invocation::InvocationAPI>>,
}

#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for RemoteRouter {
    async fn handle_event(&mut self, event: edgeless_api::invocation::Event) -> anyhow::Result<edgeless_api::invocation::LinkProcessingResult> {
        if let Some(node_client) = self.receivers.get_mut(&event.target.node_id) {
            node_client.handle_event(event).await.unwrap();
            Ok(edgeless_api::invocation::LinkProcessingResult::FINAL)
        } else {
            Ok(edgeless_api::invocation::LinkProcessingResult::PASSED)
        }
    }
}

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
            match sender.send((event.source.clone(), event.stream_id.clone(), msg)).await {
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

struct NodeLocalLinkProvider {
    router: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

impl NodeLocalLinkProvider {
    fn new() -> Self {
        Self {
            router: std::sync::Arc::new(tokio::sync::Mutex::new(NodeLocalRouter {
                receivers: std::collections::HashMap::<
                    uuid::Uuid,
                    futures::channel::mpsc::UnboundedSender<(edgeless_api::function_instance::FunctionId, u64, Message)>,
                >::new(),
            })),
        }
    }

    async fn new_link(
        &self,
        target: edgeless_api::function_instance::FunctionId,
        sender: futures::channel::mpsc::UnboundedSender<(edgeless_api::function_instance::FunctionId, u64, Message)>,
    ) -> Box<dyn DataPlaneLink> {
        self.router.lock().await.receivers.insert(target.function_id.clone(), sender);
        Box::new(NodeLocalLink {
            node_id: target.node_id.clone(),
            router: self.router.clone(),
        })
    }
}

struct RemoteLinkProvider {
    remotes: std::sync::Arc<tokio::sync::Mutex<RemoteRouter>>,
    locals: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

struct InvocationEventHandler {
    locals: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for InvocationEventHandler {
    async fn handle_event(&mut self, event: edgeless_api::invocation::Event) -> anyhow::Result<edgeless_api::invocation::LinkProcessingResult> {
        self.locals.lock().await.handle_event(event).await
    }
}

impl RemoteLinkProvider {
    async fn new(invocation_url: String, peers: std::collections::HashMap<uuid::Uuid, String>) -> Self {
        let locals = std::sync::Arc::new(tokio::sync::Mutex::new(NodeLocalRouter {
            receivers: std::collections::HashMap::<
                uuid::Uuid,
                futures::channel::mpsc::UnboundedSender<(edgeless_api::function_instance::FunctionId, u64, Message)>,
            >::new(),
        }));
        let remotes = std::sync::Arc::new(tokio::sync::Mutex::new(RemoteRouter {
            receivers: std::collections::HashMap::new(),
        }));

        let _server = tokio::spawn(edgeless_api::grpc_impl::invocation::InvocationAPIServer::run(
            Box::new(InvocationEventHandler { locals: locals.clone() }),
            invocation_url,
        ));

        let cloned_remotes = remotes.clone();
        tokio::spawn(async move {
            let mut peer_clients = std::collections::HashMap::<uuid::Uuid, Box<dyn edgeless_api::invocation::InvocationAPI>>::new();
            for (id, addr) in &peers {
                peer_clients.insert(*id, Box::new(edgeless_api::grpc_impl::invocation::InvocationAPIClient::new(&addr).await));
            }
            cloned_remotes.lock().await.receivers = peer_clients;
        });

        Self {
            remotes: remotes,
            locals: locals,
        }
    }

    async fn new_link(
        &self,
        target: edgeless_api::function_instance::FunctionId,
        sender: futures::channel::mpsc::UnboundedSender<(edgeless_api::function_instance::FunctionId, u64, Message)>,
    ) -> Box<dyn DataPlaneLink> {
        self.locals.lock().await.receivers.insert(target.function_id.clone(), sender);
        Box::new(RemoteLink {
            remotes: self.remotes.clone(),
        })
    }
}

struct RemoteLink {
    remotes: std::sync::Arc<tokio::sync::Mutex<RemoteRouter>>,
}

#[async_trait::async_trait]
impl DataPlaneLink for RemoteLink {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::FunctionId,
        msg: Message,
        src: &edgeless_api::function_instance::FunctionId,
        stream_id: u64,
    ) -> LinkProcessingResult {
        return self
            .remotes
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
    }
}

#[derive(Clone)]
pub struct DataPlaneChainWriteHandle {
    slf: edgeless_api::function_instance::FunctionId,
    output_chain: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn DataPlaneLink>>>>,
    overwrite_table: std::sync::Arc<tokio::sync::Mutex<TemporaryReceivers>>,
    next_id: u64,
}

pub struct DataPlaneChainHandle {
    receiver: futures::channel::mpsc::UnboundedReceiver<(edgeless_api::function_instance::FunctionId, u64, Message)>,
    output_chain: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn DataPlaneLink>>>>,
    slf: edgeless_api::function_instance::FunctionId,
    temporary_receivers: std::sync::Arc<tokio::sync::Mutex<TemporaryReceivers>>,
}

struct TemporaryReceivers {
    temporary_receivers:
        std::collections::HashMap<u64, Option<futures::channel::oneshot::Sender<(edgeless_api::function_instance::FunctionId, Message)>>>,
}

impl DataPlaneChainHandle {
    async fn new(
        receiver_id: edgeless_api::function_instance::FunctionId,
        output_chain: Vec<Box<dyn DataPlaneLink>>,
        receiver: futures::channel::mpsc::UnboundedReceiver<(edgeless_api::function_instance::FunctionId, u64, Message)>,
    ) -> Self {
        let (main_sender, main_receiver) = futures::channel::mpsc::unbounded::<(edgeless_api::function_instance::FunctionId, u64, Message)>();
        let receiver_overwrites = std::sync::Arc::new(tokio::sync::Mutex::new(TemporaryReceivers {
            temporary_receivers: std::collections::HashMap::new(),
        }));
        let clone_overwrites = receiver_overwrites.clone();

        tokio::spawn(async move {
            let mut receiver = receiver;
            let mut main_sender = main_sender;
            loop {
                if let Some((from, stream_id, msg)) = receiver.next().await {
                    if let Some(sender) = clone_overwrites.lock().await.temporary_receivers.get_mut(&stream_id) {
                        if let Some(sender) = sender.take() {
                            match sender.send((from.clone(), msg.clone())) {
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
                    match main_sender.send((from, stream_id, msg)).await {
                        Ok(_) => {}
                        Err(_) => {
                            break;
                        }
                    }
                }
            }
        });

        DataPlaneChainHandle {
            slf: receiver_id,
            receiver: main_receiver,
            output_chain: std::sync::Arc::new(tokio::sync::Mutex::new(output_chain)),
            temporary_receivers: receiver_overwrites,
        }
    }

    pub async fn receive_next(&mut self) -> (edgeless_api::function_instance::FunctionId, u64, String) {
        loop {
            if let Some((src, channel, msg)) = self.receiver.next().await {
                if let Message::Cast(raw_msg) = msg {
                    return (src, channel, raw_msg);
                }
                if let Message::Call(raw_msg) = msg {
                    return (src, channel, raw_msg);
                }
                log::error!("Unprocesses other message");
            }
        }
    }

    pub async fn new_write_handle(&mut self) -> DataPlaneChainWriteHandle {
        DataPlaneChainWriteHandle {
            slf: self.slf.clone(),
            output_chain: self.output_chain.clone(),
            overwrite_table: self.temporary_receivers.clone(),
            next_id: 1,
        }
    }
}

pub enum CallRet {
    NoReply,
    Reply(String),
    Err,
}

impl DataPlaneChainWriteHandle {
    pub async fn send(&mut self, target: edgeless_api::function_instance::FunctionId, msg: String) {
        self.send_inner(target, Message::Cast(msg), 0).await;
    }

    pub async fn call(&mut self, target: edgeless_api::function_instance::FunctionId, msg: String) -> CallRet {
        let (send, rec) = futures::channel::oneshot::channel::<(edgeless_api::function_instance::FunctionId, Message)>();
        let channel_id = self.next_id;
        self.next_id += 1;
        self.overwrite_table.lock().await.temporary_receivers.insert(channel_id, Some(send));
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

#[derive(Clone)]
pub struct DataPlaneChainProvider {
    local_provider: std::sync::Arc<tokio::sync::Mutex<NodeLocalLinkProvider>>,
    remote_provider: std::sync::Arc<tokio::sync::Mutex<RemoteLinkProvider>>,
}

impl DataPlaneChainProvider {
    pub async fn new(_node_id: uuid::Uuid, invocation_url: String, peers: Vec<crate::EdgelessNodeSettingsPeer>) -> Self {
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

    pub async fn get_chain_for(&mut self, target: edgeless_api::function_instance::FunctionId) -> DataPlaneChainHandle {
        let (sender, receiver) = futures::channel::mpsc::unbounded::<(edgeless_api::function_instance::FunctionId, u64, Message)>();
        let output_chain = vec![
            self.local_provider.lock().await.new_link(target.clone(), sender.clone()).await,
            self.remote_provider.lock().await.new_link(target.clone(), sender.clone()).await,
        ];
        DataPlaneChainHandle::new(target, output_chain, receiver).await
    }
}
