// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

pub mod client;
mod deployment_state;
pub mod server;
#[cfg(test)]
pub mod test;

pub struct Controller {
    sender: futures::channel::mpsc::UnboundedSender<ControllerRequest>,
}

pub(crate) enum ControllerRequest {
    START(
        edgeless_api::workflow_instance::SpawnWorkflowRequest,
        // Reply Channel
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>>,
    ),
    STOP(edgeless_api::workflow_instance::WorkflowId),
    LIST(
        edgeless_api::workflow_instance::WorkflowId,
        // Reply Channel
        tokio::sync::oneshot::Sender<anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>>>,
    ),
    UPDATENODE(
        edgeless_api::node_registration::UpdateNodeRequest,
        // Reply Channel
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse>>,
    ),
}

#[derive(Clone)]
enum ComponentType {
    Function,
    Resource,
}

impl Controller {
    pub async fn new_from_config(
        controller_settings: crate::EdgelessConSettings,
    ) -> (Self, std::pin::Pin<Box<dyn futures::Future<Output = ()> + Send>>) {
        // Connect to all orchestrators.
        // let mut orc_clients = std::collections::HashMap::<String, Box<dyn edgeless_api::orc::OrchestratorAPI>>::new();
        // for orc in &controller_settings.orchestrators {
        //     match edgeless_api::grpc_impl::orc::OrchestratorAPIClient::new(&orc.orchestrator_url, Some(1)).await {
        //         Ok(val) => {
        //             orc_clients.insert(orc.domain_id.to_string(), Box::new(val));
        //         }
        //         Err(err) => {
        //             log::error!("Could not connect to e-ORC {}: {}", &orc.orchestrator_url, err);
        //         }
        //     }
        // }

        Self::new()
    }

    fn new() -> (Self, std::pin::Pin<Box<dyn futures::Future<Output = ()> + Send>>) {
        let (sender, receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            let mut controller_task = server::ControllerTask::new(receiver);
            controller_task.run().await;
        });

        (Controller { sender }, main_task)
    }

    pub fn get_api_client(&mut self) -> Box<dyn edgeless_api::controller::ControllerAPI + Send> {
        client::ControllerClient::new(self.sender.clone())
    }
}
