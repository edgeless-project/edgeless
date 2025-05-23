// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::SinkExt;

pub struct ControllerClient {
    workflow_instance_client: Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>,
}

#[allow(clippy::new_ret_no_self)]
impl ControllerClient {
    pub fn new(
        sender: futures::channel::mpsc::UnboundedSender<super::ControllerRequest>,
    ) -> Box<dyn edgeless_api::outer::controller::ControllerAPI + Send> {
        Box::new(ControllerClient {
            workflow_instance_client: Box::new(ControllerWorkflowInstanceClient { sender }),
        })
    }
}

impl edgeless_api::outer::controller::ControllerAPI for ControllerClient {
    fn workflow_instance_api(&mut self) -> Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI> {
        self.workflow_instance_client.clone()
    }
}

#[derive(Clone)]
pub struct ControllerWorkflowInstanceClient {
    sender: futures::channel::mpsc::UnboundedSender<super::ControllerRequest>,
}

#[async_trait::async_trait]
impl edgeless_api::workflow_instance::WorkflowInstanceAPI for ControllerWorkflowInstanceClient {
    async fn start(
        &mut self,
        request: edgeless_api::workflow_instance::SpawnWorkflowRequest,
    ) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse> {
        let (reply_sender, reply_receiver) =
            tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>>();
        if let Err(err) = self.sender.send(super::ControllerRequest::Start(request.clone(), reply_sender)).await {
            anyhow::bail!("Controller Channel Error: {}", err);
        }
        match reply_receiver.await {
            Ok(ret) => ret,
            Err(err) => Err(anyhow::anyhow!("Controller Channel Error: {}", err)),
        }
    }
    async fn stop(&mut self, id: edgeless_api::workflow_instance::WorkflowId) -> anyhow::Result<()> {
        match self.sender.send(super::ControllerRequest::Stop(id)).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow::anyhow!("Controller Channel Error: {}", err)),
        }
    }
    async fn list(&mut self) -> anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowId>> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowId>>>();
        if let Err(err) = self.sender.send(super::ControllerRequest::List(reply_sender)).await {
            anyhow::bail!("Controller Channel Error: {}", err);
        }
        match reply_receiver.await {
            Ok(ret) => ret,
            Err(err) => Err(anyhow::anyhow!("Controller Channel Error: {}", err)),
        }
    }
    async fn inspect(&mut self, id: edgeless_api::workflow_instance::WorkflowId) -> anyhow::Result<edgeless_api::workflow_instance::WorkflowInfo> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::workflow_instance::WorkflowInfo>>();
        if let Err(err) = self.sender.send(super::ControllerRequest::Inspect(id.clone(), reply_sender)).await {
            anyhow::bail!("Controller Channel Error: {}", err);
        }
        match reply_receiver.await {
            Ok(ret) => ret,
            Err(err) => Err(anyhow::anyhow!("Controller Channel Error: {}", err)),
        }
    }
    async fn domains(
        &mut self,
        domain_id: String,
    ) -> anyhow::Result<std::collections::HashMap<String, edgeless_api::domain_registration::DomainCapabilities>> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<
            anyhow::Result<std::collections::HashMap<String, edgeless_api::domain_registration::DomainCapabilities>>,
        >();
        if let Err(err) = self.sender.send(super::ControllerRequest::Domains(domain_id.clone(), reply_sender)).await {
            anyhow::bail!("Controller Channel Error: {}", err);
        }
        match reply_receiver.await {
            Ok(ret) => ret,
            Err(err) => Err(anyhow::anyhow!("Controller Channel Error: {}", err)),
        }
    }
    async fn migrate(
        &mut self,
        request: edgeless_api::workflow_instance::MigrateWorkflowRequest,
    ) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse> {
        let (reply_sender, reply_receiver) =
            tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowResponse>>();
        if let Err(err) = self.sender.send(super::ControllerRequest::Migrate(request.clone(), reply_sender)).await {
            anyhow::bail!("Controller Channel Error: {}", err);
        }
        match reply_receiver.await {
            Ok(ret) => ret,
            Err(err) => Err(anyhow::anyhow!("Controller Channel Error: {}", err)),
        }
    }
}
