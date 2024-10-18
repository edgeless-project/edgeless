// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use futures::SinkExt;

pub struct ControllerClient {
    workflow_instance_client: Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>,
    node_registration_client: Box<dyn edgeless_api::node_registration::NodeRegistrationAPI>,
}

impl ControllerClient {
    pub fn new(sender: futures::channel::mpsc::UnboundedSender<super::ControllerRequest>) -> Box<dyn edgeless_api::controller::ControllerAPI + Send> {
        Box::new(ControllerClient {
            workflow_instance_client: Box::new(ControllerWorkflowInstanceClient { sender: sender.clone() }),
            node_registration_client: Box::new(ControllerNodeRegistrationClient { sender: sender.clone() }),
        })
    }
}

impl edgeless_api::controller::ControllerAPI for ControllerClient {
    fn workflow_instance_api(&mut self) -> Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI> {
        self.workflow_instance_client.clone()
    }

    fn node_registration_api(&mut self) -> Box<dyn edgeless_api::node_registration::NodeRegistrationAPI> {
        self.node_registration_client.clone()
    }
}

#[derive(Clone)]
pub struct ControllerWorkflowInstanceClient {
    sender: futures::channel::mpsc::UnboundedSender<super::ControllerRequest>,
}

#[derive(Clone)]
pub struct ControllerNodeRegistrationClient {
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
        match self.sender.send(super::ControllerRequest::START(request.clone(), reply_sender)).await {
            Ok(_) => {}
            Err(_) => return Err(anyhow::anyhow!("Controller Channel Error")),
        }
        let reply = reply_receiver.await;
        match reply {
            Ok(ret) => ret,
            Err(_) => Err(anyhow::anyhow!("Controller Channel Error")),
        }
    }
    async fn stop(&mut self, id: edgeless_api::workflow_instance::WorkflowId) -> anyhow::Result<()> {
        match self.sender.send(super::ControllerRequest::STOP(id)).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::anyhow!("Controller Channel Error")),
        }
    }
    async fn list(
        &mut self,
        id: edgeless_api::workflow_instance::WorkflowId,
    ) -> anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>> {
        let (reply_sender, reply_receiver) =
            tokio::sync::oneshot::channel::<anyhow::Result<Vec<edgeless_api::workflow_instance::WorkflowInstance>>>();
        match self.sender.send(super::ControllerRequest::LIST(id.clone(), reply_sender)).await {
            Ok(_) => {}
            Err(_) => return Err(anyhow::anyhow!("Controller Channel Error")),
        }
        let reply = reply_receiver.await;
        match reply {
            Ok(ret) => ret,
            Err(_) => Err(anyhow::anyhow!("Controller Channel Error")),
        }
    }

    async fn patch(&mut self, update: edgeless_api::common::PatchRequest) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait::async_trait]
impl edgeless_api::node_registration::NodeRegistrationAPI for ControllerNodeRegistrationClient {
    async fn update_node(
        &mut self,
        request: edgeless_api::node_registration::UpdateNodeRequest,
    ) -> anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse> {
        log::debug!("NodeRegistrationAPI::update_node() {:?}", request);
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse>>();
        if let Err(err) = self.sender.send(super::ControllerRequest::UPDATENODE(request, reply_sender)).await {
            return Err(anyhow::anyhow!("Controller channel error when updating a node: {}", err.to_string()));
        }
        match reply_receiver.await {
            Ok(res) => res,
            Err(err) => Err(anyhow::anyhow!("Controller channel error  when updating a node: {}", err.to_string())),
        }
    }
    async fn keep_alive(&mut self) {
        todo!()
    }
}
