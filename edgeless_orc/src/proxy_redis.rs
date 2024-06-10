// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use redis::Commands;

/// An orchestrator proxy that uses a Redis in-memory database to mirror
/// internal data structures and read orchestration intents.
///
/// The whole database is flushed upon initialiation.
///
/// The following keys are written:
/// - node:capabilities::UUID, where UUID is the node identifier
/// - node:health::UUID, where UUID is the node identifier
/// - provider::ID, where ID is the resource provider identifier
/// - instance::UUID, where UUID is the logical function/resource identifier
/// - dependency::UUID, where UUID is the logical function/resource identifier
///
/// All the values are JSON structures.
///
pub struct ProxyRedis {
    connection: redis::Connection,
    node_uuids: std::collections::HashSet<uuid::Uuid>,
    resource_provider_ids: std::collections::HashSet<String>,
    active_instance_uuids: std::collections::HashSet<uuid::Uuid>,
    dependency_uuids: std::collections::HashSet<uuid::Uuid>,
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
            resource_provider_ids: std::collections::HashSet::new(),
            active_instance_uuids: std::collections::HashSet::new(),
            dependency_uuids: std::collections::HashSet::new(),
        })
    }
}

impl super::proxy::Proxy for ProxyRedis {
    fn update_nodes(&mut self, nodes: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ClientDesc>) {
        // serialize the nodes' capabilities and health status to Redis
        for (uuid, client_desc) in nodes {
            redis::pipe()
                .set::<&str, &str>(
                    format!("node:capabilities:{}", uuid).as_str(),
                    serde_json::to_string(&client_desc.capabilities).unwrap_or_default().as_str(),
                )
                .set::<&str, &str>(
                    format!("node:health:{}", uuid).as_str(),
                    serde_json::to_string(&client_desc.health_status).unwrap_or_default().as_str(),
                )
                .execute(&mut self.connection);
        }

        // remove nodes that are not anymore in the orchestration domain
        let new_active_instance_uuids = nodes.keys().cloned().collect::<std::collections::HashSet<uuid::Uuid>>();
        self.active_instance_uuids.difference(&new_active_instance_uuids).for_each(|uuid| {
            redis::pipe()
                .del(format!("node:capabilities:{}", uuid).as_str())
                .del(format!("node:health:{}", uuid).as_str())
                .execute(&mut self.connection);
        });

        // update the list of node UUIDs
        self.active_instance_uuids = new_active_instance_uuids;
    }

    fn update_resource_providers(&mut self, resource_providers: &std::collections::HashMap<String, super::orchestrator::ResourceProvider>) {
        // serialize the resource providers
        for (provider_id, resource_provider) in resource_providers {
            let _ = self.connection.set::<&str, &str, usize>(
                format!("provider:{}", provider_id).as_str(),
                serde_json::to_string(&resource_provider).unwrap_or_default().as_str(),
            );
        }

        // remove resource providers that are not anymore present
        let new_resource_provider_ids = resource_providers.keys().cloned().collect::<std::collections::HashSet<String>>();
        self.resource_provider_ids.difference(&new_resource_provider_ids).for_each(|provider_id| {
            let _ = self.connection.del::<&str, usize>(format!("provider:{}", provider_id).as_str());
        });

        // update the list of the resource provider identifiers
        self.resource_provider_ids = new_resource_provider_ids;
    }

    fn update_active_instances(&mut self, active_instances: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ActiveInstance>) {
        // serialize the active instances
        for (ext_fid, active_instance) in active_instances {
            let _ = self.connection.set::<&str, &str, usize>(
                format!("instance:{}", ext_fid).as_str(),
                serde_json::to_string(&active_instance).unwrap_or_default().as_str(),
            );
        }

        // remove instances that are not active anymore
        let new_node_uuids = active_instances.keys().cloned().collect::<std::collections::HashSet<uuid::Uuid>>();
        self.node_uuids.difference(&new_node_uuids).for_each(|ext_fid| {
            let _ = self.connection.del::<&str, usize>(format!("instance:{}", ext_fid).as_str());
        });

        // update the list of active instance ext fids
        self.node_uuids = new_node_uuids;
    }

    fn update_dependency_graph(&mut self, dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>) {
        // serialize the dependency graph
        for (ext_fid, dependencies) in dependency_graph {
            let _ = self.connection.set::<&str, &str, usize>(
                format!("dependency:{}", ext_fid).as_str(),
                serde_json::to_string(&dependencies).unwrap_or_default().as_str(),
            );
        }

        // remove dependencies that do not exist anymore
        let new_dependency_uuids = dependency_graph.keys().cloned().collect::<std::collections::HashSet<uuid::Uuid>>();
        self.dependency_uuids.difference(&new_dependency_uuids).for_each(|ext_fid| {
            let _ = self.connection.del::<&str, usize>(format!("dependency:{}", ext_fid).as_str());
        });

        // update the list of active instance ext fids
        self.dependency_uuids = new_dependency_uuids;
    }
}
