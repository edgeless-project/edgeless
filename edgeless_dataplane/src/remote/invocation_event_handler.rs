use crate::local::local_router::NodeLocalRouter;
use edgeless_api::invocation::LinkProcessingResult;
use std::sync::Arc;
use tokio::sync::Mutex;

// Represents the event handler for the invocation API, which only handles events
// that are meant for the local node. Routes them to the NodeLocalRouter
pub struct InvocationEventHandler {
    pub node_id: edgeless_api::function_instance::NodeId,
    // Incoming Events are routed to correct function instances using the NodeLocalRouter
    pub local_router: Arc<Mutex<NodeLocalRouter>>,
}

#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for InvocationEventHandler {
    async fn handle(&mut self, event: edgeless_api::invocation::Event) -> edgeless_api::invocation::LinkProcessingResult {
        if event.target.node_id == self.node_id {
            let res = self.local_router.lock().await.handle(event).await;
            return res;
        } else {
            // if an event is not for us, we pass on it
            return LinkProcessingResult::IGNORED;
        }
    }
}
