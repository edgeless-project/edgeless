use std::vec;

use futures::{SinkExt, StreamExt};

enum LinkProcessingResult {
    FINAL,
    // PROCESSED,
    PASSED,
}

#[async_trait::async_trait]
trait DataPlaneLink: Send + Sync {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::FunctionId,
        msg: &str,
        src: &edgeless_api::function_instance::FunctionId,
    ) -> LinkProcessingResult;
}

struct NodeLocalLink {
    node_id: uuid::Uuid,
    router: std::sync::Arc<tokio::sync::Mutex<NodeLocalRouter>>,
}

#[async_trait::async_trait]
impl DataPlaneLink for NodeLocalLink {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::FunctionId,
        msg: &str,
        src: &edgeless_api::function_instance::FunctionId,
    ) -> LinkProcessingResult {
        if target.node_id == self.node_id {
            return self.router.lock().await.handle_send(target, msg, src).await;
        } else {
            return LinkProcessingResult::PASSED;
        }
    }
}

struct NodeLocalRouter {
    receivers: std::collections::HashMap<uuid::Uuid, futures::channel::mpsc::UnboundedSender<(edgeless_api::function_instance::FunctionId, String)>>,
}

impl NodeLocalRouter {
    async fn handle_send(
        &mut self,
        target: &edgeless_api::function_instance::FunctionId,
        msg: &str,
        src: &edgeless_api::function_instance::FunctionId,
    ) -> LinkProcessingResult {
        if let Some(sender) = self.receivers.get_mut(&target.function_id) {
            match sender.send((src.clone(), msg.to_string())).await {
                Ok(_) => {}
                Err(_) => {
                    log::debug!("Remove old receiver.");
                    self.receivers.remove(&target.function_id);
                }
            }
            return LinkProcessingResult::FINAL;
        }
        LinkProcessingResult::PASSED
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
                    futures::channel::mpsc::UnboundedSender<(edgeless_api::function_instance::FunctionId, String)>,
                >::new(),
            })),
        }
    }
}

impl NodeLocalLinkProvider {
    async fn new_link(
        &self,
        target: edgeless_api::function_instance::FunctionId,
        sender: futures::channel::mpsc::UnboundedSender<(edgeless_api::function_instance::FunctionId, String)>,
    ) -> Box<dyn DataPlaneLink> {
        self.router.lock().await.receivers.insert(target.function_id.clone(), sender);
        Box::new(NodeLocalLink {
            node_id: target.node_id.clone(),
            router: self.router.clone(),
        })
    }
}

#[derive(Clone)]
pub struct DataPlaneChainWriteHandle {
    slf: edgeless_api::function_instance::FunctionId,
    output_chain: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn DataPlaneLink>>>>,
}

pub struct DataPlaneChainHandle {
    receiver: futures::channel::mpsc::UnboundedReceiver<(edgeless_api::function_instance::FunctionId, String)>,
    output_chain: std::sync::Arc<tokio::sync::Mutex<Vec<Box<dyn DataPlaneLink>>>>,
    slf: edgeless_api::function_instance::FunctionId,
}

impl DataPlaneChainHandle {
    async fn new(
        receiver_id: edgeless_api::function_instance::FunctionId,
        output_chain: Vec<Box<dyn DataPlaneLink>>,
        receiver: futures::channel::mpsc::UnboundedReceiver<(edgeless_api::function_instance::FunctionId, String)>,
    ) -> Self {
        DataPlaneChainHandle {
            slf: receiver_id,
            receiver,
            output_chain: std::sync::Arc::new(tokio::sync::Mutex::new(output_chain)),
        }
    }

    pub async fn receive_next(&mut self) -> (edgeless_api::function_instance::FunctionId, String) {
        loop {
            if let Some(val) = self.receiver.next().await {
                return val;
            }
        }
    }

    pub async fn new_write_handle(&mut self) -> DataPlaneChainWriteHandle {
        DataPlaneChainWriteHandle {
            slf: self.slf.clone(),
            output_chain: self.output_chain.clone(),
        }
    }
}

impl DataPlaneChainWriteHandle {
    pub async fn send(&mut self, target: edgeless_api::function_instance::FunctionId, msg: String) {
        let mut lck = self.output_chain.lock().await;
        for link in &mut lck.iter_mut() {
            match link.handle_send(&target, &msg, &self.slf).await {
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
}

impl DataPlaneChainProvider {
    pub fn new() -> Self {
        Self {
            local_provider: std::sync::Arc::new(tokio::sync::Mutex::new(NodeLocalLinkProvider::new())),
        }
    }

    pub async fn get_chain_for(&mut self, target: edgeless_api::function_instance::FunctionId) -> DataPlaneChainHandle {
        let (sender, receiver) = futures::channel::mpsc::unbounded::<(edgeless_api::function_instance::FunctionId, String)>();
        let output_chain = vec![self.local_provider.lock().await.new_link(target.clone(), sender.clone()).await];
        DataPlaneChainHandle::new(target, output_chain, receiver).await
    }
}
