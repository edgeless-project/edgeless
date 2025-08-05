// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

pub struct ClientDesc {
    pub agent_url: String,
    pub invocation_url: String,
    pub api: Box<dyn edgeless_api::outer::agent::AgentAPI + Send>,
    pub capabilities: edgeless_api::node_registration::NodeCapabilities,
    pub cordoned: bool,
}

impl ClientDesc {
    pub async fn from(request: &edgeless_api::node_registration::UpdateNodeRequest) -> anyhow::Result<Self> {
        // Return immediately if the node has an invalid agent or invocation URL.
        let urls_to_validate = vec![("agent", &request.agent_url), ("invocation", &request.invocation_url)];
        let mut proto_host_ports = vec![];
        for (label, url) in urls_to_validate {
            let (proto, host, port) = match edgeless_api::util::parse_http_host(url) {
                Ok((proto, host, url)) => (proto, host, url),
                Err(err) => {
                    anyhow::bail!("Invalid {} URL '{}' for node '{}': {}", label, request.agent_url, request.node_id, err);
                }
            };
            anyhow::ensure!(!host.is_empty(), "invalid empty host in {} URL", label);
            proto_host_ports.push((proto, host, port));
        }

        let (proto, host, port) = proto_host_ports.first().unwrap();

        Ok(crate::client_desc::ClientDesc {
            agent_url: request.agent_url.clone(),
            invocation_url: request.invocation_url.clone(),
            api: match proto {
                edgeless_api::util::Proto::COAP => {
                    let addr = std::net::SocketAddrV4::new(host.parse().unwrap(), *port);
                    Box::new(edgeless_api::coap_impl::CoapClient::new(addr).await)
                }
                edgeless_api::util::Proto::HTTP | edgeless_api::util::Proto::HTTPS => {
                    Box::new(edgeless_api::grpc_impl::outer::agent::AgentAPIClient::new(&request.agent_url))
                }
            },
            capabilities: request.capabilities.clone(),
            cordoned: false,
        })
    }

    pub fn to_string_short(&self) -> String {
        format!("agent_url {} invocation_url {}", self.agent_url, self.invocation_url)
    }
}
