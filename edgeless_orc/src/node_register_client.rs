// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::SinkExt;

pub struct NodeRegisterClient {
    node_register_client: Box<dyn edgeless_api::node_registration::NodeRegistrationAPI>,
}

#[allow(clippy::new_ret_no_self)]
impl NodeRegisterClient {
    pub fn new(
        sender: futures::channel::mpsc::UnboundedSender<super::node_register::NodeRegisterRequest>,
    ) -> Box<dyn edgeless_api::outer::node_register::NodeRegisterAPI + Send> {
        Box::new(NodeRegisterClient {
            node_register_client: Box::new(NodeRegisterInnerClient { sender }),
        })
    }
}

impl edgeless_api::outer::node_register::NodeRegisterAPI for NodeRegisterClient {
    fn node_registration_api(
        &mut self,
    ) -> Box<dyn edgeless_api::node_registration::NodeRegistrationAPI> {
        self.node_register_client.clone()
    }
}

#[derive(Clone)]
pub struct NodeRegisterInnerClient {
    sender: futures::channel::mpsc::UnboundedSender<super::node_register::NodeRegisterRequest>,
}

#[async_trait::async_trait]
impl edgeless_api::node_registration::NodeRegistrationAPI for NodeRegisterInnerClient {
    async fn update_node(
        &mut self,
        request: edgeless_api::node_registration::UpdateNodeRequest,
    ) -> anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse> {
        let (reply_sender, reply_receiver) = tokio::sync::oneshot::channel::<
            anyhow::Result<edgeless_api::node_registration::UpdateNodeResponse>,
        >();
        match self
            .sender
            .send(super::node_register::NodeRegisterRequest::UpdateNode(
                request.clone(),
                reply_sender,
            ))
            .await
        {
            Ok(_) => {}
            Err(_) => return Err(anyhow::anyhow!("NodeRegister Channel Error")),
        }
        let reply = reply_receiver.await;
        match reply {
            Ok(ret) => ret,
            Err(_) => Err(anyhow::anyhow!("NodeRegister Channel Error")),
        }
    }
}
