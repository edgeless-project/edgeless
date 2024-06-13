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
    pub fn new(redis_url: &str, flushdb: bool) -> anyhow::Result<Self> {
        log::info!("creating Redis orchestrator proxy at URL {}", redis_url);

        // create the connection with the Redis server
        let mut connection = redis::Client::open(redis_url)?.get_connection()?;

        if flushdb {
            // flush the in-memory database upon construction
            let _ = redis::cmd("FLUSHDB").query(&mut connection)?;
        }

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

    fn retrieve_deploy_intents(&mut self) -> Vec<super::orchestrator::DeployIntent> {
        let mut intents = vec![];
        loop {
            let lpop_res = self.connection.lpop::<&str, Option<String>>("intents", None);

            match lpop_res {
                Ok(intent_key) => {
                    if let Some(intent_key) = intent_key {
                        let get_res = self.connection.get::<&str, Option<String>>(&intent_key);
                        match get_res {
                            Ok(intent_value) => match intent_value {
                                Some(intent_value) => match crate::orchestrator::DeployIntent::new(&intent_key, &intent_value) {
                                    Ok(intent) => intents.push(intent),
                                    Err(err) => log::warn!("invalid intent value '{}': {}", intent_value, err),
                                },
                                None => log::warn!("empty intent key '{}'", intent_key),
                            },
                            Err(err) => log::warn!("could not read intent '{}': {}", intent_key, err),
                        }
                    } else {
                        break;
                    }
                }
                Err(err) => log::warn!("could not pop from intents: {}", err),
            }
        }
        intents
    }
}

#[cfg(test)]
mod test {
    use crate::{orchestrator::DeployIntent, proxy::Proxy};

    use super::*;

    #[test]
    #[ignore]
    fn test_redis_retrieve_intents() {
        let redis_url = "redis://127.0.0.1:6379";

        // create the proxy, also flushing the db
        let mut proxy = ProxyRedis::new(redis_url, true).unwrap();

        // fill intents
        let component1 = uuid::Uuid::new_v4();
        let component2 = uuid::Uuid::new_v4();
        let component3 = uuid::Uuid::new_v4();
        let component4 = uuid::Uuid::new_v4();
        let node1 = uuid::Uuid::new_v4();
        let node2 = uuid::Uuid::new_v4();
        let intents = vec![
            DeployIntent::Migrate(component1, vec![]),
            DeployIntent::Migrate(component2, vec![node1]),
            DeployIntent::Migrate(component3, vec![node1, node2]),
            DeployIntent::Migrate(component4, vec![node1, node2, node2]),
        ];
        let mut connection = redis::Client::open(redis_url).unwrap().get_connection().unwrap();
        for intent in intents {
            assert!(connection.set::<&str, &str, String>(&intent.key(), &intent.value()).is_ok());
            assert!(connection.lpush::<&str, &str, usize>("intents", &intent.key()).is_ok());
        }

        // retrieve them
        for intent in proxy.retrieve_deploy_intents() {
            match intent {
                DeployIntent::Migrate(component, targets) => {
                    if component == component1 {
                        assert!(targets.is_empty());
                    } else if component == component2 {
                        assert!(targets.len() == 1);
                    } else if component == component3 {
                        assert!(targets.len() == 2);
                    } else if component == component4 {
                        assert!(targets.len() == 3);
                    } else {
                        panic!("unknown component: {}", component);
                    }
                }
            }
        }
    }
}
