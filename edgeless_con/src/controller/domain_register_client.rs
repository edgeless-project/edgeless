// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use futures::SinkExt;

pub struct DomainRegisterClient {
    domain_register_client: Box<dyn edgeless_api::domain_registration::DomainRegistrationAPI>,
}

#[allow(clippy::new_ret_no_self)]
impl DomainRegisterClient {
    pub fn new(
        sender: futures::channel::mpsc::UnboundedSender<super::DomainRegisterRequest>,
    ) -> Box<dyn edgeless_api::outer::domain_register::DomainRegisterAPI + Send> {
        Box::new(DomainRegisterClient {
            domain_register_client: Box::new(DomainRegisterInnerClient { sender }),
        })
    }
}

impl edgeless_api::outer::domain_register::DomainRegisterAPI for DomainRegisterClient {
    fn domain_registration_api(&mut self) -> Box<dyn edgeless_api::domain_registration::DomainRegistrationAPI> {
        self.domain_register_client.clone()
    }
}

#[derive(Clone)]
pub struct DomainRegisterInnerClient {
    sender: futures::channel::mpsc::UnboundedSender<super::DomainRegisterRequest>,
}

#[async_trait::async_trait]
impl edgeless_api::domain_registration::DomainRegistrationAPI for DomainRegisterInnerClient {
    async fn update_domain(
        &mut self,
        request: edgeless_api::domain_registration::UpdateDomainRequest,
    ) -> anyhow::Result<edgeless_api::domain_registration::UpdateDomainResponse> {
        let (reply_sender, reply_receiver) =
            tokio::sync::oneshot::channel::<anyhow::Result<edgeless_api::domain_registration::UpdateDomainResponse>>();
        match self
            .sender
            .send(super::DomainRegisterRequest::Update(request.clone(), reply_sender))
            .await
        {
            Ok(_) => {}
            Err(_) => return Err(anyhow::anyhow!("DomainRegister Channel Error")),
        }
        let reply = reply_receiver.await;
        match reply {
            Ok(ret) => ret,
            Err(_) => Err(anyhow::anyhow!("DomainRegister Channel Error")),
        }
    }
}
