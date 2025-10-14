// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use futures::SinkExt;

pub mod client;
pub mod controller_task;
mod deployment_state;
pub mod domain_register_client;
#[cfg(test)]
pub mod test;

pub struct Controller {
    workflow_instance_sender: futures::channel::mpsc::UnboundedSender<ControllerRequest>,
    domain_register_sender: futures::channel::mpsc::UnboundedSender<DomainRegisterRequest>,
}

pub(crate) enum ControllerRequest {
    Start(
        edgeless_api::workflow_instance::SpawnWorkflowRequest,
        // Reply Channel
        tokio::sync::oneshot::Sender<
            anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>,
        >,
    ),
    Stop(edgeless_api::workflow_instance::WorkflowId),
    List(
        // Reply Channel
        tokio::sync::oneshot::Sender<
            anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowId>>,
        >,
    ),
    Inspect(
        edgeless_api::workflow_instance::WorkflowId,
        // Reply Channel
        tokio::sync::oneshot::Sender<anyhow::Result<edgeless_api::workflow_instance::WorkflowInfo>>,
    ),
    Domains(
        String,
        // Reply Channel
        tokio::sync::oneshot::Sender<
            anyhow::Result<
                std::collections::HashMap<
                    String,
                    edgeless_api::domain_registration::DomainCapabilities,
                >,
            >,
        >,
    ),
    Migrate(
        edgeless_api::workflow_instance::MigrateWorkflowRequest,
        // Reply Channel
        tokio::sync::oneshot::Sender<
            anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>,
        >,
    ),
}

pub(crate) enum DomainRegisterRequest {
    Update(
        edgeless_api::domain_registration::UpdateDomainRequest,
        // Reply Channel
        tokio::sync::oneshot::Sender<
            anyhow::Result<edgeless_api::domain_registration::UpdateDomainResponse>,
        >,
    ),
}

pub(crate) enum InternalRequest {
    Refresh(
        // Reply Channel
        tokio::sync::oneshot::Sender<()>,
    ),
}

#[derive(Clone)]
enum ComponentType {
    Function,
    Resource,
}

type Task = std::pin::Pin<Box<dyn futures::Future<Output = ()> + Send>>;

impl Controller {
    pub fn new(persistence_filename: String) -> (Self, Task, Task) {
        let (workflow_instance_sender, workflow_instance_receiver) =
            futures::channel::mpsc::unbounded();
        let (domain_register_sender, domain_register_receiver) =
            futures::channel::mpsc::unbounded();
        let (internal_sender, internal_receiver) = futures::channel::mpsc::unbounded();

        let main_task = Box::pin(async move {
            let mut controller_task = controller_task::ControllerTask::new(
                persistence_filename,
                workflow_instance_receiver,
                domain_register_receiver,
                internal_receiver,
            );
            controller_task.run().await;
        });

        let refresh_task = Box::pin(async move {
            let mut sender = internal_sender;
            loop {
                let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<()>();
                let _ = sender.send(InternalRequest::Refresh(reply_sender)).await;
                let _ = reply_receiver.await;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        (
            Controller {
                workflow_instance_sender,
                domain_register_sender,
            },
            main_task,
            refresh_task,
        )
    }

    pub fn get_workflow_instance_client(
        &mut self,
    ) -> Box<dyn edgeless_api::outer::controller::ControllerAPI + Send> {
        client::ControllerClient::new(self.workflow_instance_sender.clone())
    }

    pub fn get_domain_register_client(
        &mut self,
    ) -> Box<dyn edgeless_api::outer::domain_register::DomainRegisterAPI + Send> {
        domain_register_client::DomainRegisterClient::new(self.domain_register_sender.clone())
    }
}
