// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use redis::Commands;

/// An orchestrator proxy that uses a Redis in-memory database to mirror
/// internal data structures and read orchestration intents.
pub struct ProxyRedis {
    connection: redis::Connection,
    node_uuids: std::collections::HashSet<uuid::Uuid>,
}

impl ProxyRedis {
    pub fn new(redis_url: &str) -> anyhow::Result<Self> {
        log::info!("creating Redis orchestrator proxy at URL {}", redis_url);

        // create the connection with the Redis server
        let mut connection = redis::Client::open(redis_url)?.get_connection()?;

        // flush the in-memory database upon construction
        let _ = redis::cmd("FLUSHDB").query(&mut connection)?;

        Ok(Self {
            connection,
            node_uuids: std::collections::HashSet::new(),
        })
    }
}

impl super::proxy::Proxy for ProxyRedis {
    fn update_nodes(&mut self, nodes: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ClientDesc>) {
        // serialize the nodes' capabilities and health status to Redis
        nodes
            .iter()
            .map(|(uuid, client_desc)| {
                self.connection
                    .hset::<&str, &str, &str, usize>(
                        format!("node:{}", uuid).as_str(),
                        "capabilities",
                        serde_json::to_string(&client_desc.capabilities).unwrap_or_default().as_str(),
                    )
                    .is_ok()
                    && self
                        .connection
                        .hset::<&str, &str, &str, usize>(
                            format!("node:{}", uuid).as_str(),
                            "health_status",
                            serde_json::to_string(&client_desc.health_status).unwrap_or_default().as_str(),
                        )
                        .is_ok()
            })
            .all(|x| x == true);

        // remove nodes that are not anymore in the orchestration domain
        let new_node_uuids = nodes.keys().cloned().collect::<std::collections::HashSet<uuid::Uuid>>();
        self.node_uuids
            .difference(&new_node_uuids)
            .map(|uuid| {
                self.connection
                    .hdel::<&str, Vec<&str>, usize>(format!("node:{}", uuid).as_str(), vec!["capabilities", "health_status"])
                    .is_ok()
            })
            .all(|x| x == true);

        // update the list of node UUIDs
        self.node_uuids = new_node_uuids;
    }
}
