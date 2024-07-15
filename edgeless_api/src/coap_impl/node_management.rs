// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

use crate::node_management::HealthStatus;

#[async_trait::async_trait]
impl crate::node_management::NodeManagementAPI for super::CoapClient {
    async fn update_peers(&mut self, request: crate::node_management::UpdatePeersRequest) -> anyhow::Result<()> {
        match request {
            crate::node_management::UpdatePeersRequest::Add(id, url) => {
                let (_, ip, port) = crate::util::parse_http_host(&url).unwrap();
                let ip: std::net::Ipv4Addr = ip.parse().unwrap();
                let ip_bytes: [u8; 4] = ip.octets();
                match self
                    .call_with_reply(|token, addr, buffer| {
                        edgeless_api_core::coap_mapping::COAPEncoder::encode_peer_add(
                            addr,
                            &edgeless_api_core::node_registration::NodeId(id),
                            ip_bytes,
                            port,
                            token,
                            buffer,
                        )
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(anyhow::anyhow!(String::from_utf8(err).unwrap())),
                }
            }
            crate::node_management::UpdatePeersRequest::Del(id) => {
                match self
                    .call_with_reply(|token, addr, buffer| {
                        edgeless_api_core::coap_mapping::COAPEncoder::encode_peer_remove(
                            addr,
                            &edgeless_api_core::node_registration::NodeId(id),
                            token,
                            buffer,
                        )
                    })
                    .await
                {
                    Ok(_) => Ok(()),
                    Err(err) => Err(anyhow::anyhow!(String::from_utf8(err).unwrap())),
                }
            }
            crate::node_management::UpdatePeersRequest::Clear => {
                panic!("UpdatePeersRequest::Clear not implemented");
            }
        }
    }

    async fn keep_alive(&mut self) -> anyhow::Result<crate::node_management::HealthStatus> {
        match self
            .call_with_reply(|token, addr, buffer| edgeless_api_core::coap_mapping::COAPEncoder::encode_keepalive(addr, token, buffer))
            .await
        {
            Ok(_) => Ok(HealthStatus::invalid()),
            Err(err) => Err(anyhow::anyhow!(String::from_utf8(err).unwrap())),
        }
    }
}
