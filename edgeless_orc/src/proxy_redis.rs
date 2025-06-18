// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use core::f64;
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

    // last update timestamps
    last_update_timestamps: std::collections::HashMap<crate::proxy::Category, String>,

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

        // Create the connection with the Redis server
        let mut connection = redis::Client::open(redis_url)?.get_connection()?;

        // Flush the in-memory database upon construction
        if flushdb {
            let _ = redis::cmd("FLUSHDB").query::<String>(&mut connection)?;
        }

        // Open dataset files
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
            last_update_timestamps: std::collections::HashMap::new(),
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
            "identifier,metric,timestamp,value".to_string(),
            "timestamp,logical_id,workflow_id,node_id,physical_id".to_string(),
            format!("timestamp,node_id,{}", edgeless_api::node_registration::NodeCapabilities::csv_header()),
            format!("timestamp,node_id,{}", edgeless_api::node_registration::NodeHealthStatus::csv_header()),
        ];

        let mut outfiles = vec![];
        for (filename, header) in filenames.iter().zip(headers.iter()) {
            let filename = format!("{}{}.csv", dataset_path, filename);

            // create the path to write the file, if needed
            let path = std::path::Path::new(&filename);
            let mut ancestors = path.ancestors();
            ancestors.next();
            let base_dir = ancestors.next().unwrap();
            let res = if let Err(err) = std::fs::create_dir_all(base_dir) {
                log::warn!("could not create the directory where to dump the dataset '{}': {}", filename, err);
                None
            } else {
                match ProxyRedis::open_file(filename.as_str(), append, header, &additional_header) {
                    Ok(outfile) => Some(outfile),
                    Err(err) => {
                        log::error!("could not open '{}' for writing: {}", filename, err);
                        None
                    }
                }
            };
            outfiles.push(res);
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
    // 1: [ (node_id, lid) ]
    Function(edgeless_api::function_instance::SpawnFunctionRequest, Vec<String>),

    // 0: request
    // 1: (node_id, lid)
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
    fn timestamp_now() -> f64 {
        let now = chrono::Utc::now();
        now.timestamp() as f64 + now.timestamp_subsec_nanos() as f64 / 1e9
    }

    fn fetch_instances(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, crate::active_instance::ActiveInstance> {
        self.local_timestamp_update(&crate::proxy::Category::ActiveInstances);
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
                    instances.insert(
                        instance_id,
                        match val {
                            ActiveInstanceClone::Function(spawn_req, instance_ids_str) => {
                                let mut instance_ids = vec![];
                                for instance_id_str in instance_ids_str {
                                    if let Ok(instance_id) = string_to_instance_id(&instance_id_str) {
                                        instance_ids.push(instance_id);
                                    }
                                }
                                if !instance_ids.is_empty() {
                                    crate::active_instance::ActiveInstance::Function(spawn_req, instance_ids)
                                } else {
                                    continue;
                                }
                            }
                            ActiveInstanceClone::Resource(spawn_req, instance_id_str) => {
                                if let Ok(instance_id) = string_to_instance_id(&instance_id_str) {
                                    crate::active_instance::ActiveInstance::Resource(spawn_req, instance_id)
                                } else {
                                    continue;
                                }
                            }
                        },
                    );
                }
            }
        }
        instances
    }

    fn get_last_update(&mut self, category: &crate::proxy::Category) -> String {
        let category_name = match category {
            crate::proxy::Category::NodeCapabilities => "node:capabilities",
            crate::proxy::Category::ResourceProviders => "provider",
            crate::proxy::Category::ActiveInstances => "instance",
            crate::proxy::Category::DependencyGraph => "dependency",
        };
        self.connection
            .get::<String, String>(format!("{}:last_update", category_name))
            .unwrap_or_default()
    }

    fn local_timestamp_update(&mut self, category: &crate::proxy::Category) {
        let redis_timestamp = self.get_last_update(category);
        self.last_update_timestamps.insert(category.clone(), redis_timestamp);
    }
}

impl super::proxy::Proxy for ProxyRedis {
    fn update_nodes(&mut self, nodes: &std::collections::HashMap<uuid::Uuid, crate::client_desc::ClientDesc>) {
        let timestamp = ProxyRedis::timestamp_now();

        // update the timestamp when the nodes were updated
        let _ = redis::Cmd::set(String::from("node:capabilities:last_update"), timestamp).exec(&mut self.connection);

        // serialize the nodes' capabilities and health status to Redis
        let mut new_node_capabilities = std::collections::HashMap::new();
        for (uuid, client_desc) in nodes {
            let _ = redis::Cmd::set(
                format!("node:capabilities:{}", uuid).as_str(),
                serde_json::to_string(&client_desc.capabilities).unwrap_or_default().as_str(),
            )
            .exec(&mut self.connection);
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
            let _ = redis::pipe()
                .del(format!("node:capabilities:{}", uuid).as_str())
                .ignore()
                .del(format!("node:health:{}", uuid).as_str())
                .exec(&mut self.connection);
        });

        // update the list of node UUIDs
        self.active_instance_uuids = new_active_instance_uuids;
    }

    fn update_resource_providers(&mut self, resource_providers: &std::collections::HashMap<String, crate::resource_provider::ResourceProvider>) {
        // update the timestamp when the resource providers were updated
        let _ = redis::Cmd::set(String::from("provider:last_update"), ProxyRedis::timestamp_now()).exec(&mut self.connection);

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

    fn update_active_instances(&mut self, active_instances: &std::collections::HashMap<uuid::Uuid, crate::active_instance::ActiveInstance>) {
        let timestamp = ProxyRedis::timestamp_now();

        // update the timestamp when the active instances were updated
        let _ = redis::Cmd::set(String::from("instance:last_update"), timestamp).exec(&mut self.connection);

        // serialize the active instances
        let mut new_mapping_to_instance_id = std::collections::HashMap::new();
        for (lid, active_instance) in active_instances {
            let _ = self.connection.set::<&str, &str, usize>(
                format!("instance:{}", lid).as_str(),
                serde_json::to_string(&active_instance).unwrap_or_default().as_str(),
            );
            let new_instance_ids = active_instance.instance_ids();
            if let Some(outfile) = &mut self.mapping_to_instance_id_file {
                let write = if let Some(old_instance_ids) = self.mapping_to_instance_id.get(lid) {
                    *old_instance_ids != new_instance_ids
                } else {
                    true
                };
                if write {
                    let _ = writeln!(
                        outfile,
                        "{},{},{},{},{}",
                        self.additional_fields,
                        timestamp,
                        lid,
                        active_instance.workflow_id(),
                        new_instance_ids
                            .iter()
                            .map(|x| format!("{},{}", x.node_id, x.function_id))
                            .collect::<Vec<String>>()
                            .join(",")
                    );
                }
            }
            new_mapping_to_instance_id.insert(*lid, new_instance_ids);
        }
        let _ = std::mem::replace(&mut self.mapping_to_instance_id, new_mapping_to_instance_id);

        // remove instances that are not active anymore
        let new_node_uuids = active_instances.keys().cloned().collect::<std::collections::HashSet<uuid::Uuid>>();
        self.node_uuids.difference(&new_node_uuids).for_each(|lid| {
            let _ = self.connection.del::<&str, usize>(format!("instance:{}", lid).as_str());
        });

        // update the list of active instance ext fids
        self.node_uuids = new_node_uuids;
    }

    fn update_dependency_graph(&mut self, dependency_graph: &std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>>) {
        // update the timestamp when the dependency graph was updated
        let _ = redis::Cmd::set(String::from("dependency:last_update"), ProxyRedis::timestamp_now()).exec(&mut self.connection);

        // serialize the dependency graph
        for (lid, dependencies) in dependency_graph {
            let _ =
                redis::Cmd::set(format!("dependency:{}", lid), serde_json::to_string(&dependencies).unwrap_or_default()).exec(&mut self.connection);
        }

        // remove dependencies that do not exist anymore
        let new_dependency_uuids = dependency_graph.keys().cloned().collect::<std::collections::HashSet<uuid::Uuid>>();
        self.dependency_uuids.difference(&new_dependency_uuids).for_each(|lid| {
            let _ = redis::Cmd::del(format!("dependency:{}", lid)).exec(&mut self.connection);
        });

        // update the list of active instance ext fids
        self.dependency_uuids = new_dependency_uuids;
    }

    fn update_domain_info(&mut self, domain_info: &crate::domain_info::DomainInfo) {
        let _ = self.connection.set::<&str, &str, usize>("domain_info:domain_id", &domain_info.domain_id);
    }

    fn push_node_health(&mut self, node_id: &uuid::Uuid, node_health: edgeless_api::node_registration::NodeHealthStatus) {
        let timestamp = ProxyRedis::timestamp_now();

        // Save to Redis.
        let _ = redis::Cmd::zadd(
            format!("node:health:{}", node_id).as_str(),
            serde_json::to_string(&node_health).unwrap_or_default().as_str(),
            timestamp,
        )
        .exec(&mut self.connection);
        let new_health_status = node_health.to_csv();

        // Save to dataset output.
        if let Some(outfile) = &mut self.health_status_file {
            let write = if let Some(old_health_status) = self.node_health_status.get(node_id) {
                *old_health_status != new_health_status
            } else {
                true
            };
            if write {
                let _ = writeln!(outfile, "{},{},{},{}", self.additional_fields, timestamp, node_id, new_health_status);
            }
        }
    }

    fn push_performance_samples(&mut self, _node_id: &uuid::Uuid, performance_samples: edgeless_api::node_registration::NodePerformanceSamples) {
        let all_sample_series = vec![
            ("function_execution_time", &performance_samples.function_execution_times),
            ("function_transfer_time", &performance_samples.function_transfer_times),
        ];
        for (name, series) in all_sample_series {
            for (function_id, values) in series {
                let key = format!("performance:{}:{}", function_id, name);
                for value in values {
                    // Save to Redis.
                    let _ = redis::Cmd::zadd(&key, value.to_string(), value.score()).exec(&mut self.connection);

                    // Save to dataset output.
                    if let Some(outfile) = &mut self.performance_samples_file {
                        let _ = writeln!(
                            outfile,
                            "{},{},{},{}",
                            self.additional_fields,
                            function_id,
                            name,
                            value.to_string().replacen(":", ",", 1)
                        );
                    }
                }
            }
        }

        for (function_id, log_entries) in &performance_samples.function_log_entries {
            for log_entry in log_entries {
                let key = format!("performance:{}:{}", function_id, log_entry.target);

                // Save to Redis.
                let _ = redis::Cmd::zadd(&key, log_entry.to_string(), log_entry.score()).exec(&mut self.connection);

                // Save to dataset output.
                if let Some(outfile) = &mut self.performance_samples_file {
                    let _ = writeln!(
                        outfile,
                        "{},{},{},{}",
                        self.additional_fields,
                        function_id,
                        log_entry.target,
                        log_entry.to_string().replacen(":", ",", 1)
                    );
                }
            }
        }
    }

    fn add_deploy_intents(&mut self, intents: Vec<crate::deploy_intent::DeployIntent>) {
        for intent in intents {
            match intent {
                crate::deploy_intent::DeployIntent::Migrate(instance, nodes) => {
                    let key = format!("intent:migrate:{}", instance);
                    let _ = self
                        .connection
                        .set::<&str, &str, usize>(&key, &nodes.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(","));
                    let _ = self.connection.rpush::<&str, &str, String>("intents", &key);
                }
            }
        }
    }

    fn retrieve_deploy_intents(&mut self) -> Vec<crate::deploy_intent::DeployIntent> {
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
                                Some(intent_value) => match crate::deploy_intent::DeployIntent::new(&intent_key, &intent_value) {
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

    fn fetch_domain_info(&mut self) -> crate::domain_info::DomainInfo {
        if let Ok(domain_id) = self.connection.get::<&str, String>("domain_info:domain_id") {
            crate::domain_info::DomainInfo { domain_id }
        } else {
            crate::domain_info::DomainInfo::default()
        }
    }

    fn fetch_node_capabilities(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeCapabilities> {
        self.local_timestamp_update(&crate::proxy::Category::NodeCapabilities);
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

    fn fetch_resource_providers(&mut self) -> std::collections::HashMap<String, crate::resource_provider::ResourceProvider> {
        self.local_timestamp_update(&crate::proxy::Category::ResourceProviders);
        let mut resource_providers = std::collections::HashMap::new();
        for node_key in self.connection.keys::<&str, Vec<String>>("provider:*").unwrap_or(vec![]) {
            if let Some((provider, provider_id)) = node_key.split_once(':') {
                assert_eq!("provider", provider);
                if let Ok(val) = self.connection.get::<&str, String>(&node_key) {
                    if let Ok(val) = serde_json::from_str::<crate::resource_provider::ResourceProvider>(&val) {
                        resource_providers.insert(provider_id.to_string(), val);
                    }
                }
            } else {
                panic!("invalid Redis key for a resource provider: {}", node_key);
            }
        }
        resource_providers
    }

    fn fetch_node_health(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, edgeless_api::node_registration::NodeHealthStatus> {
        let mut health = std::collections::HashMap::new();
        for node_key in self.connection.keys::<&str, Vec<String>>("node:health:*").unwrap_or(vec![]) {
            let tokens: Vec<&str> = node_key.split(':').collect();
            assert_eq!(tokens.len(), 3);
            assert_eq!("node", tokens[0]);
            assert_eq!("health", tokens[1]);
            if let Ok(node_id) = edgeless_api::function_instance::NodeId::parse_str(tokens[2]) {
                if let Ok(values) = self.connection.zrange::<&str, Vec<String>>(&node_key, -1, -1) {
                    assert!(
                        values.len() <= 1,
                        "Invalid number of elements for ZRANGE command that should return 0 or 1 elements"
                    );
                    if let Some(value) = values.first() {
                        if let Ok(val) = serde_json::from_str::<edgeless_api::node_registration::NodeHealthStatus>(value) {
                            health.insert(node_id, val);
                        }
                    }
                }
            }
        }
        health
    }

    fn fetch_node_healths(&mut self) -> crate::proxy::NodeHealthStatuses {
        let mut healths = std::collections::HashMap::new();
        for node_key in self.connection.keys::<&str, Vec<String>>("node:health:*").unwrap_or(vec![]) {
            let tokens: Vec<&str> = node_key.split(':').collect();
            assert_eq!(tokens.len(), 3);
            assert_eq!("node", tokens[0]);
            assert_eq!("health", tokens[1]);
            let mut health_history = vec![];
            if let Ok(node_id) = edgeless_api::function_instance::NodeId::parse_str(tokens[2]) {
                if let Ok(values) =
                    self.connection
                        .zrangebyscore_withscores::<&str, f64, f64, Vec<(String, f64)>>(&node_key, f64::NEG_INFINITY, f64::INFINITY)
                {
                    for (value, timestamp) in values {
                        if let Some(timestamp) = chrono::DateTime::from_timestamp(timestamp as i64, (timestamp.fract() * 1e9) as u32) {
                            if let Ok(val) = serde_json::from_str::<edgeless_api::node_registration::NodeHealthStatus>(&value) {
                                health_history.push((timestamp, val));
                            }
                        }
                    }
                }
                healths.insert(node_id, health_history);
            }
        }
        healths
    }

    fn fetch_performance_samples(&mut self) -> std::collections::HashMap<String, crate::proxy::PerformanceSamples> {
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

            if let Ok(values) = self
                .connection
                .zrangebyscore::<&str, f64, f64, Vec<String>>(&perf_key, f64::NEG_INFINITY, f64::INFINITY)
            {
                for value in values {
                    let tokens: Vec<&str> = value.split(":").collect();
                    if tokens.len() != 2 {
                        continue;
                    }
                    if let Ok(timestamp) = tokens[0].parse::<f64>() {
                        let secs = timestamp as i64;
                        let nsecs = (timestamp.fract() * 1e9) as u32;
                        if let Some(date_time) = chrono::DateTime::from_timestamp(secs, nsecs) {
                            sub_entry.push((date_time, tokens[1].to_string()));
                        }
                    }
                }
            }
        }
        samples
    }

    fn fetch_function_instance_requests(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::SpawnFunctionRequest> {
        let mut instances = std::collections::HashMap::new();
        for (logical_id, instance) in self.fetch_instances() {
            if let crate::active_instance::ActiveInstance::Function(spawn_function_req, _instance_ids) = instance {
                instances.insert(logical_id, spawn_function_req);
            }
        }
        instances
    }

    fn fetch_resource_instance_configurations(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::resource_configuration::ResourceInstanceSpecification>
    {
        let mut instances = std::collections::HashMap::new();
        for (logical_id, instance) in self.fetch_instances() {
            if let crate::active_instance::ActiveInstance::Resource(resource_specification, _instance_id) = instance {
                instances.insert(logical_id, resource_specification);
            }
        }
        instances
    }

    fn fetch_function_instances_to_nodes(
        &mut self,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, Vec<edgeless_api::function_instance::NodeId>> {
        let mut instances = std::collections::HashMap::new();
        for (logical_id, instance) in self.fetch_instances() {
            if let crate::active_instance::ActiveInstance::Function(_, instance_ids) = instance {
                instances.insert(logical_id, instance_ids.iter().map(|x| x.node_id).collect());
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
                crate::active_instance::ActiveInstance::Function(_, instance_ids) => {
                    instances.insert(logical_id, instance_ids.iter().map(|x| x.function_id).collect());
                }
                crate::active_instance::ActiveInstance::Resource(_, instance_id) => {
                    instances.insert(logical_id, vec![instance_id.function_id]);
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
            if let crate::active_instance::ActiveInstance::Resource(_, instance_id) = instance {
                instances.insert(logical_id, instance_id.node_id);
            }
        }
        instances
    }

    fn fetch_nodes_to_instances(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::NodeId, Vec<crate::proxy::Instance>> {
        let mut nodes_mapping = std::collections::HashMap::new();
        for (logical_id, instance) in self.fetch_instances() {
            match instance {
                crate::active_instance::ActiveInstance::Function(_, instance_ids) => {
                    for instance_id in instance_ids {
                        let res = nodes_mapping.entry(instance_id.node_id).or_insert(vec![]);
                        res.push(crate::proxy::Instance::Function(logical_id));
                    }
                }
                crate::active_instance::ActiveInstance::Resource(_, instance_id) => {
                    let res = nodes_mapping.entry(instance_id.node_id).or_insert(vec![]);
                    res.push(crate::proxy::Instance::Resource(logical_id));
                }
            }
        }
        nodes_mapping
    }

    fn fetch_dependency_graph(&mut self) -> std::collections::HashMap<uuid::Uuid, std::collections::HashMap<String, uuid::Uuid>> {
        self.local_timestamp_update(&crate::proxy::Category::DependencyGraph);
        let mut dependency_graph = std::collections::HashMap::new();
        for node_key in self.connection.keys::<&str, Vec<String>>("dependency:*").unwrap_or(vec![]) {
            let tokens: Vec<&str> = node_key.split(':').collect();
            assert_eq!(tokens.len(), 2);
            assert_eq!("dependency", tokens[0]);
            if let Ok(lid) = uuid::Uuid::parse_str(tokens[1]) {
                if let Ok(val) = self.connection.get::<&str, String>(&node_key) {
                    if let Ok(val) = serde_json::from_str::<std::collections::HashMap<String, uuid::Uuid>>(&val) {
                        dependency_graph.insert(lid, val);
                    }
                }
            }
        }
        dependency_graph
    }

    fn fetch_logical_id_to_workflow_id(&mut self) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, String> {
        self.fetch_instances()
            .iter()
            .map(|(logical_id, instance)| (*logical_id, instance.workflow_id()))
            .collect()
    }

    fn updated(&mut self, category: crate::proxy::Category) -> bool {
        let redis_timestamp = self.get_last_update(&category);
        let local_timestamp = self.last_update_timestamps.entry(category).or_default();
        *local_timestamp != redis_timestamp
    }

    fn garbage_collection(&mut self, period: tokio::time::Duration) {
        let remove_timestamp = chrono::Utc::now() - period;
        let remove_timestamp = remove_timestamp.timestamp() as f64 + remove_timestamp.timestamp_subsec_nanos() as f64 / 1e9;
        log::debug!("proxy garbage collection: removing data until {}", remove_timestamp);
        let key_patterns = vec!["performance:*", "node:health:*"];
        for key_pattern in key_patterns {
            for key in self.connection.keys::<&str, Vec<String>>(key_pattern).unwrap_or(vec![]) {
                let _ = self
                    .connection
                    .zrembyscore::<&str, f64, f64, ()>(&key, f64::NEG_INFINITY, remove_timestamp);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use edgeless_api::function_instance::SpawnFunctionRequest;

    use crate::{active_instance, deploy_intent::DeployIntent, proxy::Proxy};

    use super::*;

    fn get_proxy() -> Option<ProxyRedis> {
        // Skip the test if there is no local Redis listening on default port.
        match ProxyRedis::new("redis://localhost:6379", true, None) {
            Ok(redis_proxy) => return Some(redis_proxy),
            Err(_) => {
                println!("the test cannot be run because there is no Redis reachable on localhost at port 6379");
            }
        };
        None
    }

    #[serial_test::serial]
    #[test]
    fn test_redis_proxy_ctor() {
        let mut redis_proxy = match get_proxy() {
            Some(redis_proxy) => redis_proxy,
            None => return,
        };
        assert!(redis_proxy.fetch_dependency_graph().is_empty());
        assert!(redis_proxy.fetch_function_instance_requests().is_empty());
        assert!(redis_proxy.fetch_function_instances_to_nodes().is_empty());
        assert!(redis_proxy.fetch_instances().is_empty());
        assert!(redis_proxy.fetch_instances_to_physical_ids().is_empty());
        assert!(redis_proxy.fetch_node_capabilities().is_empty());
        assert!(redis_proxy.fetch_node_health().is_empty());
        assert!(redis_proxy.fetch_nodes_to_instances().is_empty());
        assert!(redis_proxy.fetch_performance_samples().is_empty());
        assert!(redis_proxy.fetch_resource_instance_configurations().is_empty());
        assert!(redis_proxy.fetch_resource_instances_to_nodes().is_empty());
        assert!(redis_proxy.fetch_resource_providers().is_empty());

        assert!(!redis_proxy.updated(crate::proxy::Category::ActiveInstances));
        assert!(!redis_proxy.updated(crate::proxy::Category::NodeCapabilities));
        assert!(!redis_proxy.updated(crate::proxy::Category::DependencyGraph));
        assert!(!redis_proxy.updated(crate::proxy::Category::ResourceProviders));
    }

    #[serial_test::serial]
    #[test]
    fn test_redis_proxy_instances() {
        let mut redis_proxy = match get_proxy() {
            Some(redis_proxy) => redis_proxy,
            None => return,
        };

        let mut active_instances = std::collections::HashMap::new();
        let node1_id = uuid::Uuid::new_v4(); // functions
        let node2_id = uuid::Uuid::new_v4(); // resources
        let mut logical_physical_ids = vec![];
        for _ in 0..10 {
            logical_physical_ids.push((uuid::Uuid::new_v4(), uuid::Uuid::new_v4()));
            active_instances.insert(
                logical_physical_ids.last().unwrap().0,
                crate::active_instance::ActiveInstance::Function(
                    SpawnFunctionRequest {
                        code: edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: "fun".to_string(),
                            function_class_type: "class".to_string(),
                            function_class_version: "1.0".to_string(),
                            function_class_code: vec![],
                            function_class_outputs: vec!["out1".to_string(), "out2".to_string()],
                        },
                        annotations: std::collections::HashMap::new(),
                        state_specification: edgeless_api::function_instance::StateSpecification {
                            state_id: uuid::Uuid::new_v4(),
                            state_policy: edgeless_api::function_instance::StatePolicy::NodeLocal,
                        },
                        workflow_id: "workflow_1".to_string(),
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
                crate::active_instance::ActiveInstance::Resource(
                    edgeless_api::resource_configuration::ResourceInstanceSpecification {
                        class_type: "res".to_string(),
                        configuration: std::collections::HashMap::from([
                            ("key1".to_string(), "val1".to_string()),
                            ("key2".to_string(), "val2".to_string()),
                        ]),
                        workflow_id: "workflow_1".to_string(),
                    },
                    edgeless_api::function_instance::InstanceId {
                        node_id: node2_id,
                        function_id: logical_physical_ids.last().unwrap().1,
                    },
                ),
            );
        }

        assert!(!redis_proxy.updated(crate::proxy::Category::ActiveInstances));
        redis_proxy.update_active_instances(&active_instances);
        assert!(redis_proxy.updated(crate::proxy::Category::ActiveInstances));

        let mut function_requests_expected = std::collections::HashMap::new();
        for (lid, instance) in &active_instances {
            if let active_instance::ActiveInstance::Function(req, _) = instance {
                function_requests_expected.insert(*lid, req.clone());
            }
        }
        assert_eq!(function_requests_expected, redis_proxy.fetch_function_instance_requests());
        assert!(!redis_proxy.updated(crate::proxy::Category::ActiveInstances));

        let mut resource_configurations_expected = std::collections::HashMap::new();
        for (lid, instance) in &active_instances {
            if let active_instance::ActiveInstance::Resource(spec, _) = instance {
                resource_configurations_expected.insert(*lid, spec.clone());
            }
        }
        assert_eq!(resource_configurations_expected, redis_proxy.fetch_resource_instance_configurations());

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
    }

    #[serial_test::serial]
    #[test]
    fn test_redis_proxy_health_and_performance_samples() {
        let mut redis_proxy = match get_proxy() {
            Some(redis_proxy) => redis_proxy,
            None => return,
        };

        let mut sample_cnt = 0;
        let mut new_sample = |value| {
            sample_cnt += 2;
            edgeless_api::node_registration::Sample {
                timestamp_sec: sample_cnt as i64,
                timestamp_ns: (sample_cnt + 1) as u32,
                sample: value,
            }
        };

        let mut log_cnt = 0;
        let mut new_log = |value| {
            log_cnt += 2;
            edgeless_api::node_registration::FunctionLogEntry {
                timestamp_sec: log_cnt as i64,
                timestamp_ns: (log_cnt + 1) as u32,
                target: String::from("target"),
                message: format!("value={}", value),
            }
        };

        // Check health status and performance samples.
        let mut health_status = edgeless_api::node_registration::NodeHealthStatus {
            mem_free: 3,
            mem_used: 4,
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
            disk_free_space: 20,
            disk_tot_reads: 21,
            disk_tot_writes: 22,
            gpu_load_perc: 23,
            gpu_temp_cels: 24,
            active_power: 25,
        };
        let samples_1_values: Vec<f64> = vec![100.0, 101.0, 102.0, 103.0];
        let samples_2_values: Vec<f64> = vec![200.0, 201.0];
        let samples_1: Vec<edgeless_api::node_registration::Sample> = samples_1_values.iter().map(|x| new_sample(*x)).collect();
        let samples_2: Vec<edgeless_api::node_registration::Sample> = samples_2_values.iter().map(|x| new_sample(*x)).collect();
        let log_1_values: Vec<f64> = vec![100.0, 101.0, 102.0, 103.0];
        let log_2_values: Vec<f64> = vec![200.0, 201.0];
        let log_1: Vec<edgeless_api::node_registration::FunctionLogEntry> = log_1_values.iter().map(|x| new_log(*x)).collect();
        let log_2: Vec<edgeless_api::node_registration::FunctionLogEntry> = log_2_values.iter().map(|x| new_log(*x)).collect();
        let node_id_perf = uuid::Uuid::new_v4();
        let fid_perf_1 = uuid::Uuid::new_v4();
        let fid_perf_2 = uuid::Uuid::new_v4();
        redis_proxy.push_node_health(&node_id_perf, health_status.clone());
        health_status.mem_free += 1;
        std::thread::sleep(std::time::Duration::from_millis(10));
        redis_proxy.push_node_health(&node_id_perf, health_status.clone());
        health_status.mem_free += 1;
        std::thread::sleep(std::time::Duration::from_millis(10));
        redis_proxy.push_node_health(&node_id_perf, health_status.clone());
        redis_proxy.push_performance_samples(
            &node_id_perf,
            edgeless_api::node_registration::NodePerformanceSamples {
                function_execution_times: std::collections::HashMap::from([(fid_perf_1, samples_1.clone()), (fid_perf_2, samples_2.clone())]),
                function_transfer_times: std::collections::HashMap::from([(fid_perf_1, samples_1.clone()), (fid_perf_2, samples_2.clone())]),
                function_log_entries: std::collections::HashMap::from([(fid_perf_1, log_1.clone()), (fid_perf_2, log_2.clone())]),
            },
        );

        // Node's last health.
        let node_health_res = redis_proxy.fetch_node_health();
        assert_eq!(std::collections::HashMap::from([(node_id_perf, health_status.clone())]), node_health_res);

        // Node's health history.
        let node_healths_res = redis_proxy.fetch_node_healths();
        let health_history = node_healths_res.get(&node_id_perf).unwrap();
        assert_eq!(3, health_history.len());
        health_status.mem_free -= 2;
        assert_eq!(health_status, health_history[0].1);
        health_status.mem_free += 1;
        assert_eq!(health_status, health_history[1].1);
        health_status.mem_free += 1;
        assert_eq!(health_status, health_history[2].1);
        assert!(health_history[0].0 < health_history[1].0);
        assert!(health_history[1].0 < health_history[2].0);

        // Performance samples
        let samples = redis_proxy.fetch_performance_samples();

        let entry = samples.get(&fid_perf_1.to_string()).unwrap();
        assert_eq!(3, entry.len());
        let actual_values = entry.get("function_execution_time").unwrap();
        assert_eq!(
            samples_1_values.iter().map(|x| x.to_string()).collect::<Vec<String>>(),
            actual_values.iter().map(|x| x.1.clone()).collect::<Vec<String>>()
        );
        let actual_values = entry.get("function_transfer_time").unwrap();
        assert_eq!(
            samples_1_values.iter().map(|x| x.to_string()).collect::<Vec<String>>(),
            actual_values.iter().map(|x| x.1.clone()).collect::<Vec<String>>()
        );
        let actual_values = entry.get("target").unwrap();
        assert_eq!(
            log_1_values.iter().map(|x| format!("value={}", x)).collect::<Vec<String>>(),
            actual_values.iter().map(|x| x.1.clone()).collect::<Vec<String>>()
        );

        let entry = samples.get(&fid_perf_2.to_string()).unwrap();
        assert_eq!(3, entry.len());
        let actual_values = entry.get("function_execution_time").unwrap();
        assert_eq!(
            samples_2_values.iter().map(|x| x.to_string()).collect::<Vec<String>>(),
            actual_values.iter().map(|x| x.1.clone()).collect::<Vec<String>>()
        );
        let actual_values = entry.get("function_transfer_time").unwrap();
        assert_eq!(
            samples_2_values.iter().map(|x| x.to_string()).collect::<Vec<String>>(),
            actual_values.iter().map(|x| x.1.clone()).collect::<Vec<String>>()
        );
        let actual_values = entry.get("target").unwrap();
        assert_eq!(
            log_2_values.iter().map(|x| format!("value={}", x)).collect::<Vec<String>>(),
            actual_values.iter().map(|x| x.1.clone()).collect::<Vec<String>>()
        );

        // Purge the proxy.
        std::thread::sleep(std::time::Duration::from_millis(10));
        redis_proxy.garbage_collection(tokio::time::Duration::from_millis(1));
        assert!(redis_proxy.fetch_node_healths().is_empty());
        assert!(redis_proxy.fetch_performance_samples().is_empty());
        assert!(redis_proxy.fetch_node_health().is_empty());
    }

    #[serial_test::serial]
    #[test]
    fn test_redis_proxy_resource_providers() {
        let mut redis_proxy = match get_proxy() {
            Some(redis_proxy) => redis_proxy,
            None => return,
        };

        // Check nodes and resource providers.
        let mut resource_providers = std::collections::HashMap::new();
        resource_providers.insert(
            "provider1".to_string(),
            crate::resource_provider::ResourceProvider {
                class_type: "class1".to_string(),
                node_id: uuid::Uuid::new_v4(),
                outputs: vec!["out".to_string()],
            },
        );

        assert!(!redis_proxy.updated(crate::proxy::Category::ResourceProviders));
        redis_proxy.update_resource_providers(&resource_providers);
        assert!(redis_proxy.updated(crate::proxy::Category::ResourceProviders));

        assert_eq!(resource_providers, redis_proxy.fetch_resource_providers());
        assert!(!redis_proxy.updated(crate::proxy::Category::ResourceProviders));
    }
    #[serial_test::serial]
    #[test]
    fn test_redis_proxy_node_capabilities() {
        let mut redis_proxy = match get_proxy() {
            Some(redis_proxy) => redis_proxy,
            None => return,
        };

        let node_id = uuid::Uuid::new_v4();
        let mut nodes = std::collections::HashMap::new();
        let (mock_node_sender, _mock_node_receiver) = futures::channel::mpsc::unbounded::<crate::orchestrator::test::MockAgentEvent>();
        nodes.insert(
            node_id.clone(),
            crate::client_desc::ClientDesc {
                agent_url: "http://127.0.0.1:10000".to_string(),
                invocation_url: "http://127.0.0.1:10001".to_string(),
                api: Box::new(crate::orchestrator::test::MockNode {
                    node_id: node_id.clone(),
                    sender: mock_node_sender,
                }) as Box<dyn edgeless_api::outer::agent::AgentAPI + Send>,
                capabilities: edgeless_api::node_registration::NodeCapabilities::minimum(),
            },
        );
        assert!(!redis_proxy.updated(crate::proxy::Category::NodeCapabilities));
        redis_proxy.update_nodes(&nodes);
        assert!(redis_proxy.updated(crate::proxy::Category::NodeCapabilities));

        let mut nodes_expected = std::collections::HashMap::new();
        nodes_expected.insert(node_id.clone(), edgeless_api::node_registration::NodeCapabilities::minimum());

        assert_eq!(nodes_expected, redis_proxy.fetch_node_capabilities());
        assert!(!redis_proxy.updated(crate::proxy::Category::NodeCapabilities));
    }

    #[serial_test::serial]
    #[test]
    fn test_redis_proxy_dependency_graph() {
        let mut redis_proxy = match get_proxy() {
            Some(redis_proxy) => redis_proxy,
            None => return,
        };

        let mut dependency_graph = std::collections::HashMap::new();
        for _ in 0..10 {
            let mut dependencies = std::collections::HashMap::new();
            for j in 0..5 {
                dependencies.insert(format!("out-{}", j), uuid::Uuid::new_v4());
            }
            dependency_graph.insert(uuid::Uuid::new_v4(), dependencies);
        }

        assert!(!redis_proxy.updated(crate::proxy::Category::DependencyGraph));
        redis_proxy.update_dependency_graph(&dependency_graph);
        assert!(redis_proxy.updated(crate::proxy::Category::DependencyGraph));

        assert_eq!(dependency_graph, redis_proxy.fetch_dependency_graph());
        assert!(!redis_proxy.updated(crate::proxy::Category::DependencyGraph));
    }

    #[serial_test::serial]
    #[test]
    fn test_redis_proxy_domain_info() {
        let domain_info = crate::domain_info::DomainInfo {
            domain_id: String::from("my-domain"),
        };
        let mut redis_proxy = match get_proxy() {
            Some(redis_proxy) => redis_proxy,
            None => return,
        };
        redis_proxy.update_domain_info(&domain_info);

        assert_eq!(domain_info, redis_proxy.fetch_domain_info());
    }

    #[serial_test::serial]
    #[test]
    fn test_redis_proxy_intents() {
        // Skip the test if there is no local Redis listening on default port.
        let mut redis_proxy = match ProxyRedis::new("redis://localhost:6379", true, None) {
            Ok(redis_proxy) => redis_proxy,
            Err(_) => {
                println!("the test cannot be run because there is no Redis reachable on localhost at port 6379");
                return;
            }
        };

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
        redis_proxy.add_deploy_intents(intents);

        // retrieve them
        for intent in redis_proxy.retrieve_deploy_intents() {
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
