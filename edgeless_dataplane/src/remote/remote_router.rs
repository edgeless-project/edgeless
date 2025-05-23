use edgeless_api::function_instance::NodeId;

pub struct RemoteRouter {
    pub receivers: std::collections::HashMap<NodeId, Box<dyn edgeless_api::invocation::InvocationAPI>>,
}

#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for RemoteRouter {
    async fn handle(&mut self, event: edgeless_api::invocation::Event) -> edgeless_api::invocation::LinkProcessingResult {
        // if we are aware of a receiver for which this event is targetted, we
        // pass it there
        if let Some(node_client) = self.receivers.get_mut(&event.target.node_id) {
            log::debug!("Sending to a remote client");
            return node_client.handle(event).await;
        } else {
            return edgeless_api::invocation::LinkProcessingResult::IGNORED;
        }
    }
}
