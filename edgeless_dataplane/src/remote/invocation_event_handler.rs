// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
pub struct InvocationEventHandler {
    pub node_id: edgeless_api::function_instance::NodeId,
    pub locals: std::sync::Arc<tokio::sync::Mutex<crate::local::local_router::NodeLocalRouter>>,
}

#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for InvocationEventHandler {
    async fn handle(&mut self, event: edgeless_api::invocation::Event) -> edgeless_api::invocation::LinkProcessingResult {
        if event.target.node_id == self.node_id {
            self.locals.lock().await.handle(event).await
        } else {
            edgeless_api::invocation::LinkProcessingResult::ERROR("Wrong Node ID".to_string())
        }
    }
}
