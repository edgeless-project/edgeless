// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-License-Identifier: MIT

struct ActiveMulticastLink {
    addr: std::net::Ipv4Addr,
    active_nodes: Vec<edgeless_api::function_instance::NodeId>,
}

pub struct MulticastController {
    pool_free: Vec<std::net::Ipv4Addr>,
    active: std::collections::HashMap<edgeless_api::link::LinkInstanceId, ActiveMulticastLink>,
}

impl MulticastController {
    pub fn new() -> MulticastController {
        let pool_free: Vec<_> = std::ops::Range { start: 153, end: 253 }
            .into_iter()
            .map(|i| std::net::Ipv4Addr::new(224, 0, 0, i))
            .collect();

        MulticastController {
            pool_free: pool_free,
            active: std::collections::HashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::link::LinkController for MulticastController {
    fn new_link(&mut self, nodes: Vec<edgeless_api::function_instance::NodeId>) -> anyhow::Result<edgeless_api::link::LinkInstanceId> {
        let id = edgeless_api::link::LinkInstanceId(uuid::Uuid::new_v4());
        let ip = self.pool_free.pop();

        if let Some(ip) = ip {
            self.active.insert(
                id.clone(),
                ActiveMulticastLink {
                    addr: ip.clone(),
                    active_nodes: nodes,
                },
            );
            Ok(id)
        } else {
            Err(anyhow::anyhow!("No Capacity"))
        }
    }

    fn config_for(&self, link: edgeless_api::link::LinkInstanceId, _node: edgeless_api::function_instance::NodeId) -> Option<Vec<u8>> {
        if let Some(active_link) = self.active.get(&link) {
            let cfg = crate::common::MulticastConfig {
                ip: active_link.addr.clone(),
                port: 9999,
            };
            Some(serde_json::to_string(&cfg).unwrap().into_bytes())
        } else {
            None
        }
    }

    fn remove_link(&mut self, id: edgeless_api::link::LinkInstanceId) {
        if let Some(active) = self.active.remove(&id) {
            self.pool_free.push(active.addr);
        }
    }

    async fn instantiate_control_plane(&mut self, _id: edgeless_api::link::LinkInstanceId) {
        // NOOP
    }
}
