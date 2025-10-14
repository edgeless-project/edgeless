// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use std::str::FromStr;

#[async_trait::async_trait]
trait NodeRegistrationHelper {
    async fn register(
        &mut self,
        update: crate::node_registration::UpdateNodeRequest,
    ) -> anyhow::Result<crate::node_registration::UpdateNodeResponse>;
}

#[async_trait::async_trait]
impl crate::node_registration::NodeRegistrationAPI for super::CoapClient {
    async fn update_node(
        &mut self,
        request: crate::node_registration::UpdateNodeRequest,
    ) -> anyhow::Result<crate::node_registration::UpdateNodeResponse> {
        self.register(request).await
    }
}

#[async_trait::async_trait]
impl NodeRegistrationHelper for super::CoapClient {
    async fn register(
        &mut self,
        update: crate::node_registration::UpdateNodeRequest,
    ) -> anyhow::Result<crate::node_registration::UpdateNodeResponse> {
        let mut encoded_resources = heapless::Vec::new();

        for resource in &update.resource_providers {
            let mut outputs = heapless::Vec::new();

            for output in &resource.outputs {
                outputs
                    .push(output.as_str())
                    .map_err(|_| anyhow::anyhow!("Too many outputs"))?;
            }

            encoded_resources
                .push(
                    edgeless_api_core::node_registration::ResourceProviderSpecification {
                        provider_id: &resource.provider_id,
                        class_type: &resource.class_type,
                        outputs,
                    },
                )
                .map_err(|_| anyhow::anyhow!("Too many outputs"))?;
        }

        let encoded_registration = edgeless_api_core::node_registration::EncodedNodeRegistration {
            node_id: edgeless_api_core::node_registration::NodeId(update.node_id),
            agent_url: heapless::String::<256>::from_str(update.agent_url.as_str()).unwrap(),
            invocation_url: heapless::String::<256>::from_str(update.invocation_url.as_str())
                .unwrap(),
            resources: encoded_resources,
        };

        let res = self
            .call_with_reply(|token, addr, buffer| {
                edgeless_api_core::coap_mapping::COAPEncoder::encode_node_registration(
                    addr,
                    &encoded_registration,
                    token,
                    &mut buffer[..],
                )
            })
            .await;
        match res {
            Ok(_) => return Ok(crate::node_registration::UpdateNodeResponse::Accepted),
            Err(err) => {
                return Err(anyhow::anyhow!(core::str::from_utf8(&err)
                    .unwrap()
                    .to_string()))
            }
        }
    }
}
