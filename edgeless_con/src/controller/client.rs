// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::SinkExt;

pub struct ControllerClient {
    workflow_instance_client: Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>,
}

#[allow(clippy::new_ret_no_self)]
impl ControllerClient {
    pub fn new(sender: futures::channel::mpsc::UnboundedSender<super::ControllerRequest>) -> Box<dyn edgeless_api::controller::ControllerAPI + Send> {
        Box::new(ControllerClient {
            workflow_instance_client: Box::new(ControllerWorkflowInstanceClient { sender }),
        })
    }
}

impl edgeless_api::controller::ControllerAPI for ControllerClient {
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
        match self.sender.send(super::ControllerRequest::Start(request.clone(), reply_sender)).await {
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
        match self.sender.send(super::ControllerRequest::Stop(id)).await {
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
        match self.sender.send(super::ControllerRequest::List(id.clone(), reply_sender)).await {
            Ok(_) => {}
            Err(_) => return Err(anyhow::anyhow!("Controller Channel Error")),
        }
        let reply = reply_receiver.await;
        match reply {
            Ok(ret) => ret,
            Err(_) => Err(anyhow::anyhow!("Controller Channel Error")),
        }
    }
}
