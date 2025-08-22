// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
pub struct RemoteRouter {
    pub receivers: std::collections::HashMap<edgeless_api::function_instance::NodeId, Box<dyn edgeless_api::invocation::InvocationAPI>>,
}

#[async_trait::async_trait]
impl edgeless_api::invocation::InvocationAPI for RemoteRouter {
    async fn handle(&mut self, event: edgeless_api::invocation::Event) -> edgeless_api::invocation::LinkProcessingResult {
        if let Some(node_client) = self.receivers.get_mut(&event.target.node_id) {
            match node_client.handle(event).await {
                edgeless_api::invocation::LinkProcessingResult::FINAL => return edgeless_api::invocation::LinkProcessingResult::FINAL,
                edgeless_api::invocation::LinkProcessingResult::IGNORED => return edgeless_api::invocation::LinkProcessingResult::IGNORED,
                edgeless_api::invocation::LinkProcessingResult::ERROR(e) => {
                    log::error!("Error while processing link: {:?}", e);
                    return edgeless_api::invocation::LinkProcessingResult::ERROR(e);
                }
            }
        } else {
            // we can not process this even, ignore it
            edgeless_api::invocation::LinkProcessingResult::IGNORED
        }
    }
}
