use crate::core::*;
use crate::node_local::NodeLocalRouter;
use edgeless_api::invocation::InvocationAPI;

// Link allowing to send messages to a remote node using the InvocationAPI.
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

pub struct RemoteRouter {
    receivers: std::collections::HashMap<uuid::Uuid, Box<dyn edgeless_api::invocation::InvocationAPI>>,
}

pub struct RemoteLinkProvider {
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

impl RemoteLinkProvider {
    pub async fn new(invocation_url: String, peers: std::collections::HashMap<uuid::Uuid, String>) -> Self {
        let locals = std::sync::Arc::new(tokio::sync::Mutex::new(NodeLocalRouter {
            receivers: std::collections::HashMap::<uuid::Uuid, futures::channel::mpsc::UnboundedSender<DataplaneEvent>>::new(),
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

    pub async fn new_link(
        &self,
        target: edgeless_api::function_instance::FunctionId,
        sender: futures::channel::mpsc::UnboundedSender<DataplaneEvent>,
    ) -> Box<dyn DataPlaneLink> {
        self.locals.lock().await.receivers.insert(target.function_id.clone(), sender);
        Box::new(RemoteLink {
            remotes: self.remotes.clone(),
        })
    }
}
