// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use redis::Commands;
use std::io::Write;
use std::str::FromStr;

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

    // copy of data structures dumped to files
    mapping_to_instance_id: std::collections::HashMap<uuid::Uuid, Vec<edgeless_api::function_instance::InstanceId>>,
    node_capabilities: std::collections::HashMap<uuid::Uuid, String>,
    node_health_status: std::collections::HashMap<uuid::Uuid, String>,

    // dataset dumping stuff
    additional_fields: String,
    health_status_file: Option<std::fs::File>,
    capabilities_file: Option<std::fs::File>,
    mapping_to_instance_id_file: Option<std::fs::File>,
    performance_samples_file: Option<std::fs::File>,
}

impl ProxyRedis {
    ///
    /// Create a Redis EDGELESS orchestrator proxy.
    ///
    /// Parameters:
    /// - `redis_url`: the URL of the external Redis server.
    /// - `flushdb`: if true, then the Redis database is flushed upon creation.
    /// - `dataset_settings`: the settings to save samples to output files.
    ///
    pub fn new(redis_url: &str, flushdb: bool, dataset_settings: Option<crate::EdgelessOrcProxyDatasetSettings>) -> anyhow::Result<Self> {
        log::info!(
            "creating Redis orchestrator proxy at URL {} ({})",
            redis_url,
            if flushdb { "flush DB" } else { "do not flush DB" }
        );

        // create the connection with the Redis server
        let mut connection = redis::Client::open(redis_url)?.get_connection()?;

        if flushdb {
            // flush the in-memory database upon construction
            redis::cmd("FLUSHDB").query(&mut connection)?;
        }

        let additional_fields = match &dataset_settings {
            Some(dataset_settings) => dataset_settings.additional_fields.clone(),
            None => "".to_string(),
        };

        let (health_status_file, capabilities_file, mapping_to_instance_id_file, performance_samples_file) =
            if let Some(dataset_settings) = dataset_settings {
                if !dataset_settings.dataset_path.is_empty() {
                    ProxyRedis::open_files(dataset_settings.dataset_path, dataset_settings.append, dataset_settings.additional_header)
                } else {
                    (None, None, None, None)
                }
            } else {
                (None, None, None, None)
            };

        Ok(Self {
            connection,
            node_uuids: std::collections::HashSet::new(),
            resource_provider_ids: std::collections::HashSet::new(),
            active_instance_uuids: std::collections::HashSet::new(),
            dependency_uuids: std::collections::HashSet::new(),
            mapping_to_instance_id: std::collections::HashMap::new(),
            node_capabilities: std::collections::HashMap::new(),
            node_health_status: std::collections::HashMap::new(),
            additional_fields,
            health_status_file,
            capabilities_file,
            mapping_to_instance_id_file,
            performance_samples_file,
        })
    }

    fn open_files(
        dataset_path: String,
        append: bool,
        additional_header: String,
    ) -> (Option<std::fs::File>, Option<std::fs::File>, Option<std::fs::File>, Option<std::fs::File>) {
        let filenames = ["performance_samples", "mapping_to_instance_id", "capabilities", "health_status"];
        let headers = [
            "metric,identifier,value,timestamp".to_string(),
            "timestamp,logical_id,node_id,physical_id".to_string(),
            format!("timestamp,node_id,{}", edgeless_api::node_registration::NodeCapabilities::csv_header()),
            format!("timestamp,node_id,{}", edgeless_api::node_management::NodeHealthStatus::csv_header()),
        ];
        let mut outfiles = vec![];
        for (filename, header) in filenames.iter().zip(headers.iter()) {
            let filename = format!("{}{}.csv", dataset_path, filename);
            match ProxyRedis::open_file(filename.as_str(), append, header, &additional_header) {
                Ok(outfile) => outfiles.push(Some(outfile)),
                Err(err) => {
                    log::error!("could not open '{}' for writing: {}", filename, err);
                    outfiles.push(None);
                }
            };
        }
        assert_eq!(4, outfiles.len());
        (
            outfiles.pop().unwrap(),
            outfiles.pop().unwrap(),
            outfiles.pop().unwrap(),
            outfiles.pop().unwrap(),
        )
    }

    fn open_file(filename: &str, append: bool, header: &str, additional_header: &str) -> anyhow::Result<std::fs::File> {
        let write_header = !append
            || match std::fs::metadata(filename) {
                Ok(metadata) => metadata.len() == 0,
                Err(_) => true,
            };
        let mut outfile = std::fs::OpenOptions::new()
            .write(true)
            .append(append)
            .create(true)
            .truncate(!append)
            .open(filename)?;

        if write_header {
            writeln!(&mut outfile, "{},{}", additional_header, header)?;
        }

        Ok(outfile)
    }
}

// Data structure clone of ActiveInstance, which can be deserialized.
#[derive(Clone, serde::Deserialize, Debug)]
pub enum ActiveInstanceClone {
    // 0: request
    // 1: [ (node_id, int_fid) ]
    Function(edgeless_api::function_instance::SpawnFunctionRequest, Vec<String>),

    // 0: request
    // 1: (node_id, int_fid)
    Resource(edgeless_api::resource_configuration::ResourceInstanceSpecification, String),
}

fn string_to_instance_id(val: &str) -> anyhow::Result<edgeless_api::function_instance::InstanceId> {
    let tokens: Vec<&str> = val.split(' ').collect();
    if tokens.len() != 4 {
        anyhow::bail!("invalid number of tokens in InstanceId: {}", tokens.len());
    }

    let node_id = match uuid::Uuid::from_str(&tokens[1][0..tokens[1].len() - 1]) {
        Ok(val) => val,
        Err(err) => anyhow::bail!("invalid node_id in InstanceId: {}", err),
    };
    let function_id = match uuid::Uuid::from_str(&tokens[3][0..tokens[3].len() - 1]) {
        Ok(val) => val,
        Err(err) => anyhow::bail!("invalid function_id in InstanceId: {}", err),
    };
    Ok(edgeless_api::function_instance::InstanceId { node_id, function_id })
}

impl ProxyRedis {
    fn timestamp_now() -> String {
        let duration = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
        format!("{}.{}", duration.as_secs(), duration.subsec_millis())
    }

    fn fetch_instances(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, ActiveInstanceClone> {
        let mut instance_ids = vec![];
        for instance_key in self.connection.keys::<&str, Vec<String>>("instance:*").unwrap_or(vec![]) {
            let tokens: Vec<&str> = instance_key.split(':').collect();
            if tokens.len() == 2 {
                if let Ok(uuid) = edgeless_api::function_instance::ComponentId::parse_str(tokens[1]) {
                    instance_ids.push(uuid);
                }
            }
        }
        let mut instances = std::collections::HashMap::new();
        for instance_id in instance_ids {
            if let Ok(val) = self.connection.get::<String, String>(format!("instance:{}", instance_id)) {
                if let Ok(val) = serde_json::from_str::<ActiveInstanceClone>(&val) {
                    instances.insert(instance_id, val);
                }
            }
        }
        instances
    }
}

impl super::proxy::Proxy for ProxyRedis {
    fn update_nodes(&mut self, nodes: &std::collections::HashMap<uuid::Uuid, super::orchestrator::ClientDesc>) {
        let timestamp = ProxyRedis::timestamp_now();

        // serialize the nodes' capabilities and health status to Redis
        let mut new_node_capabilities = std::collections::HashMap::new();
        for (uuid, client_desc) in nodes {
            redis::pipe()
                .set::<&str, &str>(
                    format!("node:capabilities:{}", uuid).as_str(),
                    serde_json::to_string(&client_desc.capabilities).unwrap_or_default().as_str(),
                )
                .execute(&mut self.connection);
            let new_caps = client_desc.capabilities.to_csv();
            if let Some(outfile) = &mut self.capabilities_file {
                let write: bool = if let Some(old_caps) = self.node_capabilities.get(uuid) {
                    *old_caps != new_caps
                } else {
                    true
                };
                if write {
                    let _ = writeln!(outfile, "{},{},{},{}", self.additional_fields, timestamp, uuid, new_caps);
                }
            }
            new_node_capabilities.insert(*uuid, new_caps);
        }
        let _ = std::mem::replace(&mut self.node_capabilities, new_node_capabilities);

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
        let timestamp = ProxyRedis::timestamp_now();

        // serialize the active instances
        let mut new_mapping_to_instance_id = std::collections::HashMap::new();
        for (ext_fid, active_instance) in active_instances {
            let _ = self.connection.set::<&str, &str, usize>(
                format!("instance:{}", ext_fid).as_str(),
                serde_json::to_string(&active_instance).unwrap_or_default().as_str(),
            );
            let new_instance_ids = active_instance.instance_ids();
            if let Some(outfile) = &mut self.mapping_to_instance_id_file {
                let write = if let Some(old_instance_ids) = self.mapping_to_instance_id.get(ext_fid) {
                    *old_instance_ids != new_instance_ids
                } else {
                    true
                };
                if write {
                    let _ = writeln!(
                        outfile,
                        "{},{},{},{}",
                        self.additional_fields,
                        timestamp,
                        ext_fid,
                        new_instance_ids
                            .iter()
                            .map(|x| format!("{},{}", x.node_id, x.function_id))
                            .collect::<Vec<String>>()
                            .join(",")
                    );
                }
            }
            new_mapping_to_instance_id.insert(*ext_fid, new_instance_ids);
        }
        let _ = std::mem::replace(&mut self.mapping_to_instance_id, new_mapping_to_instance_id);

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

    fn push_keep_alive_responses(&mut self, keep_alive_responses: Vec<(uuid::Uuid, edgeless_api::node_management::KeepAliveResponse)>) {
        let timestamp = ProxyRedis::timestamp_now();

        // serialize the nodes' health status and performance samples to Redis
        let mut new_node_health_status = std::collections::HashMap::new();
        for (uuid, keep_alive_response) in keep_alive_responses {
            // Save health status.
            redis::pipe()
                .set::<&str, &str>(
                    format!("node:health:{}", uuid).as_str(),
                    serde_json::to_string(&keep_alive_response.health_status).unwrap_or_default().as_str(),
                )
                .execute(&mut self.connection);
            let new_health_status = keep_alive_response.health_status.to_csv();
            if let Some(outfile) = &mut self.health_status_file {
                let write = if let Some(old_health_status) = self.node_health_status.get(&uuid) {
                    *old_health_status != new_health_status
                } else {
                    true
                };
                if write {
                    let _ = writeln!(outfile, "{},{},{},{}", self.additional_fields, timestamp, &uuid, new_health_status);
                }
            }
            new_node_health_status.insert(uuid, new_health_status);

            // Save performance samples.
            for (function_id, values) in keep_alive_response.performance_samples.function_execution_times {
                let key = format!("performance:function_execution_time:{}", function_id);
                for value in values {
                    redis::pipe()
                        .rpush::<&str, &str>(&key, format!("{},{}", value, &timestamp).as_str())
                        .execute(&mut self.connection);

                    // Save to dataset output.
                    if let Some(outfile) = &mut self.performance_samples_file {
                        let _ = writeln!(
                            outfile,
                            "{},function_execution_time,{},{},{}",
                            self.additional_fields, function_id, value, &timestamp
                        );
                    }
                }
            }
        }
        let _ = std::mem::replace(&mut self.node_health_status, new_node_health_status);
    }

    fn add_deploy_intents(&mut self, intents: Vec<super::orchestrator::DeployIntent>) {
        for intent in intents {
            match intent {
                super::orchestrator::DeployIntent::Migrate(instance, nodes) => {
                    let key = format!("intent:migrate:{}", instance);
                    let _ = self
                        .connection
                        .set::<&str, &str, usize>(&key, &nodes.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","));
                    let _ = self.connection.rpush::<&str, &str, String>("intents", &key);
                }
            }
        }
    }

    fn retrieve_deploy_intents(&mut self) -> Vec<super::orchestrator::DeployIntent> {
        let mut intents = vec![];
        loop {
            let lpop_res = self.connection.lpop::<&str, Option<String>>("intents", None);

            match lpop_res {
                Ok(intent_key) => {
                    if let Some(intent_key) = intent_key {
                        let get_res = self.connection.get::<&str, Option<String>>(&intent_key);
                        let _ = self.connection.del::<&str, usize>(&intent_key);
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

    fn fetch_node_capabilities(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeCapabilities> {
        let mut capabilities = std::collections::HashMap::new();
        for node_key in self.connection.keys::<&str, Vec<String>>("node:capabilities:*").unwrap_or(vec![]) {
            let tokens: Vec<&str> = node_key.split(':').collect();
            assert_eq!(tokens.len(), 3);
            if let Ok(node_id) = edgeless_api::function_instance::NodeId::parse_str(tokens[2]) {
                if let Ok(val) = self.connection.get::<&str, String>(&node_key) {
                    if let Ok(val) = serde_json::from_str::<edgeless_api::node_registration::NodeCapabilities>(&val) {
                        capabilities.insert(node_id, val);
                    }
                }
            }
        }
        capabilities
    }

    fn fetch_node_health(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_management::NodeHealthStatus> {
        let mut health = std::collections::HashMap::new();
        for node_key in self.connection.keys::<&str, Vec<String>>("node:health:*").unwrap_or(vec![]) {
            let tokens: Vec<&str> = node_key.split(':').collect();
            assert_eq!(tokens.len(), 3);
            assert_eq!("node", tokens[0]);
            assert_eq!("health", tokens[1]);
            if let Ok(node_id) = edgeless_api::function_instance::NodeId::parse_str(tokens[2]) {
                if let Ok(val) = self.connection.get::<&str, String>(&node_key) {
                    if let Ok(val) = serde_json::from_str::<edgeless_api::node_management::NodeHealthStatus>(&val) {
                        health.insert(node_id, val);
                    }
                }
            }
        }
        health
    }

    fn fetch_performance_samples(&mut self) -> std::collections::HashMap<String, std::collections::HashMap<String, Vec<(f64, f64)>>> {
        let mut samples = std::collections::HashMap::new();
        for perf_key in self.connection.keys::<&str, Vec<String>>("performance:*").unwrap_or(vec![]) {
            let tokens: Vec<&str> = perf_key.split(':').collect();
            if tokens.len() != 3 {
                continue;
            }
            assert_eq!(tokens.len(), 3);
            assert_eq!("performance", tokens[0]);

            let entry = samples.entry(tokens[1].to_string()).or_insert(std::collections::HashMap::new());
            let sub_entry = entry.entry(tokens[2].to_string()).or_insert(vec![]);
            if let Ok(values) = self.connection.lrange::<&str, Vec<String>>(&perf_key, 0, -1) {
                for value in values {
                    let tokens: Vec<&str> = value.split(",").collect();
                    if tokens.len() != 2 {
                        continue;
                    }
                    if let (Ok(metric), Ok(timestamp)) = (tokens[0].parse::<f64>(), tokens[1].parse::<f64>()) {
                        sub_entry.push((metric, timestamp));
                    }
                }
            }
        }
        samples
    }

    fn fetch_function_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::NodeId>> {
        let mut instances = std::collections::HashMap::new();
        for (logical_id, instance) in self.fetch_instances() {
            if let ActiveInstanceClone::Function(_, instance_ids) = instance {
                instances.insert(
                    logical_id,
                    instance_ids
                        .iter()
                        .filter_map(|x| string_to_instance_id(x).ok())
                        .map(|x| x.node_id)
                        .collect(),
                );
            }
        }
        instances
    }

    fn fetch_instances_to_physical_ids(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::ComponentId>> {
        let mut instances = std::collections::HashMap::new();
        for (logical_id, instance) in self.fetch_instances() {
            match instance {
                ActiveInstanceClone::Function(_, instance_ids) => {
                    instances.insert(
                        logical_id,
                        instance_ids
                            .iter()
                            .filter_map(|x| string_to_instance_id(x).ok())
                            .map(|x| x.function_id)
                            .collect(),
                    );
                }
                ActiveInstanceClone::Resource(_, instance_id) => {
                    if let Ok(instance_id) = string_to_instance_id(&instance_id) {
                        instances.insert(logical_id, vec![instance_id.function_id]);
                    }
                }
            }
        }
        instances
    }

    fn fetch_resource_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::NodeId> {
        let mut instances = std::collections::HashMap::new();
        for (logical_id, instance) in self.fetch_instances() {
            if let ActiveInstanceClone::Resource(_, instance_id) = instance {
                if let Ok(instance_id) = string_to_instance_id(&instance_id) {
                    instances.insert(logical_id, instance_id.node_id);
                }
            }
        }
        instances
    }

    fn fetch_nodes_to_instances(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, Vec<crate::proxy::Instance>> {
        let mut nodes_mapping = std::collections::HashMap::new();
        for (logical_id, instance) in self.fetch_instances() {
            match instance {
                ActiveInstanceClone::Function(_, instance_ids) => {
                    for instance_id in instance_ids {
                        if let Ok(instance_id) = string_to_instance_id(&instance_id) {
                            let res = nodes_mapping.entry(instance_id.node_id).or_insert(vec![]);
                            res.push(crate::proxy::Instance::Function(logical_id));
                        }
                    }
                }
                ActiveInstanceClone::Resource(_, instance_id) => {
                    if let Ok(instance_id) = string_to_instance_id(&instance_id) {
                        let res = nodes_mapping.entry(instance_id.node_id).or_insert(vec![]);
                        res.push(crate::proxy::Instance::Resource(logical_id));
                    }
                }
            }
        }
        nodes_mapping
    }
}

#[cfg(test)]
mod test {
    use edgeless_api::function_instance::SpawnFunctionRequest;

    use crate::{orchestrator::DeployIntent, proxy::Proxy};

    use super::*;

    #[test]
    fn test_redis_proxy() {
        // Skip the test if there is no local Redis listening on default port.
        let mut redis_proxy = match ProxyRedis::new("redis://localhost:6379", true, None) {
            Ok(redis_proxy) => redis_proxy,
            Err(_) => {
                println!("the test cannot be run because there is no Redis reachable on localhost at port 6379");
                return;
            }
        };

        assert!(redis_proxy.fetch_function_instances_to_nodes().is_empty());
        assert!(redis_proxy.fetch_instances().is_empty());
        assert!(redis_proxy.fetch_node_capabilities().is_empty());
        assert!(redis_proxy.fetch_node_health().is_empty());
        assert!(redis_proxy.fetch_nodes_to_instances().is_empty());
        assert!(redis_proxy.fetch_resource_instances_to_nodes().is_empty());

        let mut active_instances = std::collections::HashMap::new();
        let node1_id = uuid::Uuid::new_v4(); // functions
        let node2_id = uuid::Uuid::new_v4(); // resources
        let mut logical_physical_ids = vec![];
        for _ in 0..10 {
            logical_physical_ids.push((uuid::Uuid::new_v4(), uuid::Uuid::new_v4()));
            active_instances.insert(
                logical_physical_ids.last().unwrap().0,
                crate::orchestrator::ActiveInstance::Function(
                    SpawnFunctionRequest {
                        instance_id: None,
                        code: edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: "fun".to_string(),
                            function_class_type: "class".to_string(),
                            function_class_version: "1.0".to_string(),
                            function_class_code: vec![],
                            function_class_outputs: vec![],
                        },
                        annotations: std::collections::HashMap::new(),
                        state_specification: edgeless_api::function_instance::StateSpecification {
                            state_id: uuid::Uuid::new_v4(),
                            state_policy: edgeless_api::function_instance::StatePolicy::NodeLocal,
                        },
                    },
                    vec![edgeless_api::function_instance::InstanceId {
                        node_id: node1_id,
                        function_id: logical_physical_ids.last().unwrap().1,
                    }],
                ),
            );
        }

        for _ in 0..5 {
            logical_physical_ids.push((uuid::Uuid::new_v4(), uuid::Uuid::new_v4()));
            active_instances.insert(
                logical_physical_ids.last().unwrap().0,
                crate::orchestrator::ActiveInstance::Resource(
                    edgeless_api::resource_configuration::ResourceInstanceSpecification {
                        class_type: "res".to_string(),
                        output_mapping: std::collections::HashMap::new(),
                        configuration: std::collections::HashMap::new(),
                    },
                    edgeless_api::function_instance::InstanceId {
                        node_id: node2_id,
                        function_id: logical_physical_ids.last().unwrap().1,
                    },
                ),
            );
        }

        redis_proxy.update_active_instances(&active_instances);

        let function_instances = redis_proxy.fetch_function_instances_to_nodes();
        assert_eq!(function_instances.len(), 10);
        for (_instance, nodes) in function_instances {
            assert_eq!(nodes.len(), 1);
            assert!(nodes.first().is_some());
            assert!(nodes.first().unwrap() == &node1_id);
        }

        let resources_instances = redis_proxy.fetch_resource_instances_to_nodes();
        assert_eq!(resources_instances.len(), 5);
        for (_instance, node) in resources_instances {
            assert!(node == node2_id);
        }

        let nodes = redis_proxy.fetch_nodes_to_instances();
        assert_eq!(nodes.len(), 2);
        let entry = nodes.get(&node1_id);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().len(), 10);
        let entry = nodes.get(&node2_id);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().len(), 5);

        let logical_to_physical = redis_proxy.fetch_instances_to_physical_ids();
        assert_eq!(logical_physical_ids.len(), logical_to_physical.len());
        for mapping in logical_to_physical {
            let logical = mapping.0;
            assert_eq!(1, mapping.1.len());
            let physical = mapping.1.first().unwrap();

            let elem = logical_physical_ids.iter().find(|x| x.0 == logical).unwrap();
            assert_eq!(logical, elem.0);
            assert_eq!(*physical, elem.1);
        }

        // Check health status and performance samples.
        let health_status = edgeless_api::node_management::NodeHealthStatus {
            cpu_usage: 1,
            cpu_load: 2,
            mem_free: 3,
            mem_used: 4,
            mem_total: 5,
            mem_available: 6,
            proc_cpu_usage: 7,
            proc_memory: 8,
            proc_vmemory: 9,
            load_avg_1: 10,
            load_avg_5: 11,
            load_avg_15: 12,
            tot_rx_bytes: 13,
            tot_rx_pkts: 14,
            tot_rx_errs: 15,
            tot_tx_bytes: 16,
            tot_tx_pkts: 17,
            tot_tx_errs: 18,
            disk_tot_space: 19,
            disk_free_space: 20,
            disk_tot_reads: 21,
            disk_tot_writes: 22,
        };
        let samples_1: Vec<f64> = vec![100.0, 101.0, 102.0, 103.0];
        let samples_2: Vec<f64> = vec![200.0, 201.0];
        let node_id_perf = uuid::Uuid::new_v4();
        let fid_perf_1 = uuid::Uuid::new_v4();
        let fid_perf_2 = uuid::Uuid::new_v4();
        let keep_alive_responses = vec![(
            node_id_perf,
            edgeless_api::node_management::KeepAliveResponse {
                health_status: health_status.clone(),
                performance_samples: edgeless_api::node_management::NodePerformanceSamples {
                    function_execution_times: std::collections::HashMap::from([(fid_perf_1, samples_1.clone()), (fid_perf_2, samples_2.clone())]),
                },
            },
        )];
        redis_proxy.push_keep_alive_responses(keep_alive_responses);

        let node_health_res = redis_proxy.fetch_node_health();
        assert_eq!(std::collections::HashMap::from([(node_id_perf, health_status)]), node_health_res);

        let samples = redis_proxy.fetch_performance_samples();
        let entry = samples.get("function_execution_time").unwrap();
        assert_eq!(2, entry.len());
        let samples_1_res = entry.get(&fid_perf_1.to_string()).unwrap();
        let samples_2_res = entry.get(&fid_perf_2.to_string()).unwrap();
        assert_eq!(samples_1, samples_1_res.iter().map(|x| x.0).collect::<Vec<f64>>());
        assert_eq!(samples_2, samples_2_res.iter().map(|x| x.0).collect::<Vec<f64>>());
    }

    #[test]
    #[ignore]
    fn test_redis_retrieve_intents() {
        let redis_url = "redis://127.0.0.1:6379";

        // create the proxy, also flushing the db
        let mut proxy = ProxyRedis::new(redis_url, true, None).unwrap();

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
            assert!(connection.rpush::<&str, &str, usize>("intents", &intent.key()).is_ok());
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
