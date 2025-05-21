// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use resources::resource_provider_specs::ResourceProviderSpecs;

pub mod agent;
pub mod base_runtime;
pub mod container_runner;
pub mod gpu_info;
pub mod node_subscriber;
pub mod power_info;
pub mod resources;
pub mod state_management;
#[cfg(feature = "wasmtime")]
pub mod wasm_runner;
#[cfg(feature = "wasmi")]
pub mod wasmi_runner;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeSettings {
    /// General settings.
    pub general: EdgelessNodeGeneralSettings,
    /// Telemetry settings.
    pub telemetry: EdgelessNodeTelemetrySettings,
    /// WASM run-time settings. Disabled if not present.
    pub wasm_runtime: Option<EdgelessNodeWasmRuntimeSettings>,
    /// Container run-time settings.  Disabled if not present.
    pub container_runtime: Option<EdgelessNodeContainerRuntimeSettings>,
    /// Resource settings.
    pub resources: Option<EdgelessNodeResourceSettings>,
    /// User-specific capabilities.
    pub user_node_capabilities: Option<NodeCapabilitiesUser>,
    /// Power information settings.
    pub power_info: Option<EdgelessNodePowerInfoSettings>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeTelemetrySettings {
    /// The URL exposed by this node to publish telemetry metrics collected.
    pub metrics_url: String,
    /// Log level to use for telemetry events, if enabled.
    pub log_level: Option<String>,
    /// True if performance samples are sent to the orchestrator as part of health status responses to keep-alive polls.
    pub performance_samples: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeWasmRuntimeSettings {
    /// True if WASM is enabled.
    pub enabled: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeContainerRuntimeSettings {
    /// True if the container run-time is enabled.
    pub enabled: bool,
    /// End-point of the gRPC server to use for the GuestAPIHost interface.
    pub guest_api_host_url: String,
}

impl Default for EdgelessNodeContainerRuntimeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            guest_api_host_url: String::from("http://127.0.0.1:7100"),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeGeneralSettings {
    /// The UUID of this node.
    pub node_id: uuid::Uuid,
    /// The URL of the agent of this node used for creating the local server.
    pub agent_url: String,
    /// The agent URL announced by the node.
    /// It is the end-point used by the orchestrator to manage the node.
    /// It can be different from `agent_url`, e.g., for NAT traversal.
    pub agent_url_announced: String,
    /// The URL of the dataplane of this node, used for event dispatching.
    pub invocation_url: String,
    /// The invocation URL announced by the node.
    /// It can be different from `agent_url`, e.g., for NAT traversal.
    pub invocation_url_announced: String,
    /// The COAP URL of the dataplane of this node, used for event dispatching.
    pub invocation_url_coap: Option<String>,
    /// The COAP invocation URL announced by the node.
    /// It can be different from `agent_url`, e.g., for NAT traversal.
    pub invocation_url_announced_coap: Option<String>,
    /// The URL of the node register server.
    pub node_register_url: String,
    /// The interval at which the node refreshes subscription, s.
    pub subscription_refresh_interval_sec: u64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeResourceSettings {
    /// If true, prepend the hostname to the resource name.
    pub prepend_hostname: bool,
    /// If `http_ingress_provider` is not empty, this is the URL of the
    /// HTTP web server exposed by the http-ingress resource for this node.
    pub http_ingress_url: Option<String>,
    /// If not empty, a http-ingress resource provider with that name is created.
    pub http_ingress_provider: Option<String>,
    /// If not empty, a http-egress resource provider with that name is created.
    pub http_egress_provider: Option<String>,
    /// If not empty, a file-log resource provider with that name is created.
    /// The resource will write on the local filesystem.
    pub file_log_provider: Option<String>,
    /// If not empty, a redis resource provider with that name is created.
    /// The resource will connect to a remote Redis server to update the
    /// value of a given given, as specified in the resource configuration
    /// at run-time.
    pub redis_provider: Option<String>,
    /// If not empty, a DDA resource with that name is created.
    pub dda_provider: Option<String>,
    /// The ollama resource provider settings.
    pub ollama_provider: Option<OllamaProviderSettings>,
    /// The serverless resource provider settings.
    pub serverless_provider: Option<Vec<ServerlessProviderSettings>>,
    /// If not empty, a kafka-egress resource provider with that name is created.
    /// The resource will connect to a remote Kafka server to stream the
    /// messages received on a given topic.
    pub kafka_egress_provider: Option<String>,
    /// The metrics collector resource provider settings.
    pub metrics_collector_provider: Option<MetricsCollectorProviderSettings>,
    /// The sqlx resource provider.
    pub sqlx_provider: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct OllamaProviderSettings {
    /// The address of the ollama server.
    pub host: String,
    /// The port of the ollama server.
    pub port: u16,
    /// The maximum number of messages in the history of the ollama resource.
    pub messages_number_limit: u16,
    /// If not empty, an ollama resource provider with that name is created.
    pub provider: String,
}

impl Default for OllamaProviderSettings {
    fn default() -> Self {
        Self {
            host: String::from("localhost"),
            port: 11434,
            messages_number_limit: 30,
            provider: String::default(),
        }
    }
}
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct ServerlessProviderSettings {
    /// The resource provider class type.
    pub class_type: String,
    /// The resource provider version.
    pub version: String,
    /// The serverless function entry point as an HTTP URL.
    pub function_url: String,
    /// The resource provider name, if not empty.
    pub provider: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MetricsCollectorProviderSettings {
    /// Type of the metrics collector that is used to store run-time
    /// measurements from function instances.
    pub collector_type: String,
    /// If collector_type is "Redis" then this is the URL of the Redis server.
    pub redis_url: Option<String>,
    /// If not empty, a metrics collector resource provider with that name is created.
    pub provider: String,
}

impl Default for MetricsCollectorProviderSettings {
    fn default() -> Self {
        Self {
            collector_type: String::from("None"),
            redis_url: Some(String::from("redis://localhost:6379")),
            provider: String::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodePowerInfoSettings {
    /// The endpoint IP:port of the Modbus server.
    pub modbus_endpoint: String,
    /// The index of the PDU outlet to query.
    pub outlet_number: u16,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct NodeCapabilitiesUser {
    pub num_cpus: Option<u32>,
    pub model_name_cpu: Option<String>,
    pub clock_freq_cpu: Option<f32>,
    pub num_cores: Option<u32>,
    pub mem_size: Option<u32>,
    pub labels: Option<Vec<String>>,
    pub is_tee_running: Option<bool>,
    pub has_tpm: Option<bool>,
    pub disk_tot_space: Option<u32>,
    pub num_gpus: Option<u32>,
    pub model_name_gpu: Option<String>,
    pub mem_size_gpu: Option<u32>,
}

impl NodeCapabilitiesUser {
    pub fn empty() -> Self {
        Self {
            num_cpus: None,
            model_name_cpu: None,
            clock_freq_cpu: None,
            num_cores: None,
            mem_size: None,
            labels: None,
            is_tee_running: None,
            has_tpm: None,
            disk_tot_space: None,
            num_gpus: None,
            model_name_gpu: None,
            mem_size_gpu: None,
        }
    }
}

impl Default for NodeCapabilitiesUser {
    fn default() -> Self {
        let caps = get_capabilities(vec!["RUST_WASM".to_string()], NodeCapabilitiesUser::empty());
        Self {
            num_cpus: Some(caps.num_cpus),
            model_name_cpu: Some(caps.model_name_cpu),
            clock_freq_cpu: Some(caps.clock_freq_cpu),
            num_cores: Some(caps.num_cores),
            mem_size: Some(caps.mem_size),
            labels: Some(caps.labels),
            is_tee_running: Some(caps.is_tee_running),
            has_tpm: Some(caps.has_tpm),
            disk_tot_space: Some(caps.disk_tot_space),
            num_gpus: Some(caps.num_gpus),
            model_name_gpu: Some(caps.model_name_gpu),
            mem_size_gpu: Some(caps.mem_size_gpu),
        }
    }
}

fn get_capabilities(runtimes: Vec<String>, user_node_capabilities: NodeCapabilitiesUser) -> edgeless_api::node_registration::NodeCapabilities {
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        log::warn!("sysinfo does not support (yet) this OS");
    }

    let mut sys = sysinfo::System::new();
    sys.refresh_all();

    let mut disks = sysinfo::Disks::new();
    disks.refresh_list();
    disks.refresh();
    let unique_total_space = disks
        .iter()
        .map(|x| (x.name().to_str().unwrap_or_default(), x.total_space()))
        .collect::<std::collections::BTreeMap<&str, u64>>();

    let mut model_name_set = std::collections::HashSet::new();
    let mut clock_freq_cpu_set = std::collections::HashSet::new();
    for processor in sys.cpus() {
        model_name_set.insert(processor.brand());
        clock_freq_cpu_set.insert(processor.frequency());
    }
    let model_name_cpu = match model_name_set.iter().next() {
        Some(val) => val.to_string(),
        None => "".to_string(),
    };
    if model_name_set.len() > 1 {
        log::debug!("CPUs have different models, using: {}", model_name_cpu);
    }
    let clock_freq_cpu = match clock_freq_cpu_set.iter().next() {
        Some(val) => *val as f32,
        None => 0.0,
    };
    if clock_freq_cpu_set.len() > 1 {
        log::debug!("CPUs have different frequencies, using: {}", clock_freq_cpu);
    }

    // Retrieve user labels and add default ones, if not already present.
    let mut labels = user_node_capabilities.labels.unwrap_or_default();
    labels.push(format!("hostname={}", sysinfo::System::host_name().unwrap_or_default()));
    labels.sort();
    labels.dedup();

    edgeless_api::node_registration::NodeCapabilities {
        num_cpus: user_node_capabilities.num_cpus.unwrap_or(sys.cpus().len() as u32),
        model_name_cpu: user_node_capabilities.model_name_cpu.unwrap_or(model_name_cpu),
        clock_freq_cpu: user_node_capabilities.clock_freq_cpu.unwrap_or(clock_freq_cpu),
        num_cores: user_node_capabilities.num_cores.unwrap_or(sys.physical_core_count().unwrap_or(1) as u32),
        mem_size: user_node_capabilities.mem_size.unwrap_or((sys.total_memory() / (1024 * 1024)) as u32),
        labels,
        is_tee_running: user_node_capabilities.is_tee_running.unwrap_or(false),
        has_tpm: user_node_capabilities.has_tpm.unwrap_or(false),
        runtimes,
        disk_tot_space: user_node_capabilities
            .disk_tot_space
            .unwrap_or((unique_total_space.values().sum::<u64>() / (1024 * 1024)) as u32),
        num_gpus: user_node_capabilities.num_gpus.unwrap_or(crate::gpu_info::get_num_gpus() as u32),
        model_name_gpu: user_node_capabilities.model_name_gpu.unwrap_or(crate::gpu_info::get_model_name_gpu()),
        mem_size_gpu: user_node_capabilities
            .mem_size_gpu
            .unwrap_or((crate::gpu_info::get_mem_size_gpu() / (1024)) as u32),
    }
}

async fn fill_resources(
    data_plane: edgeless_dataplane::handle::DataplaneProvider,
    node_id: uuid::Uuid,
    settings: &Option<EdgelessNodeResourceSettings>,
    provider_specifications: &mut Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
    telemetry_provider: &edgeless_telemetry::telemetry_events::TelemetryProcessor,
) -> std::collections::HashMap<String, agent::ResourceDesc> {
    let mut ret = std::collections::HashMap::<String, agent::ResourceDesc>::new();

    if let Some(settings) = settings {
        let hostname = sysinfo::System::host_name().unwrap_or_default();
        let make_provider_id = |x: &str| {
            if settings.prepend_hostname {
                format!("{}-{}", hostname, x)
            } else {
                x.to_string()
            }
        };

        if let (Some(http_ingress_url), Some(provider_id)) = (&settings.http_ingress_url, &settings.http_ingress_provider) {
            if !http_ingress_url.is_empty() && !provider_id.is_empty() {
                let class_type = resources::http_ingress::HttpIngressResourceSpec {}.class_type();
                let provider_id = make_provider_id(provider_id);
                log::info!("Creating http-ingress resource provider '{}' at {}", provider_id, http_ingress_url);
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: resources::http_ingress::ingress_task(
                            data_plane.clone(),
                            edgeless_api::function_instance::InstanceId::new(node_id),
                            http_ingress_url.clone(),
                        )
                        .await,
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id,
                    class_type,
                    outputs: resources::http_ingress::HttpIngressResourceSpec {}.outputs(),
                });
            }
        }

        if let Some(provider_id) = &settings.http_egress_provider {
            if !provider_id.is_empty() {
                log::info!("Creating http-egress resource provider '{}'", provider_id);
                let class_type = resources::http_egress::HttpEgressResourceSpec {}.class_type();
                let provider_id = make_provider_id(provider_id);
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::http_egress::EgressResourceProvider::new(
                                data_plane.clone(),
                                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                    ("RESOURCE_CLASS_TYPE".to_string(), class_type.clone()),
                                    ("RESOURCE_PROVIDER_ID".to_string(), provider_id.clone()),
                                    ("NODE_ID".to_string(), node_id.to_string()),
                                ]))),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id,
                    class_type,
                    outputs: resources::http_egress::HttpEgressResourceSpec {}.outputs(),
                });
            }
        }

        if let Some(provider_id) = &settings.file_log_provider {
            if !provider_id.is_empty() {
                log::info!("Creating file-log resource provider '{}'", provider_id);
                let class_type = resources::file_log::FileLogResourceSpec {}.class_type();
                let provider_id = make_provider_id(provider_id);
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::file_log::FileLogResourceProvider::new(
                                data_plane.clone(),
                                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                    ("RESOURCE_CLASS_TYPE".to_string(), class_type.clone()),
                                    ("RESOURCE_PROVIDER_ID".to_string(), provider_id.clone()),
                                    ("NODE_ID".to_string(), node_id.to_string()),
                                ]))),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id,
                    class_type,
                    outputs: resources::file_log::FileLogResourceSpec {}.outputs(),
                });
            }
        }

        if let Some(provider_id) = &settings.redis_provider {
            if !provider_id.is_empty() {
                log::info!("Creating redis resource provider '{}'", provider_id);
                let class_type = resources::redis::RedisResourceSpec {}.class_type();
                let provider_id = make_provider_id(provider_id);
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::redis::RedisResourceProvider::new(
                                data_plane.clone(),
                                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                    ("RESOURCE_CLASS_TYPE".to_string(), class_type.clone()),
                                    ("RESOURCE_PROVIDER_ID".to_string(), provider_id.clone()),
                                    ("NODE_ID".to_string(), node_id.to_string()),
                                ]))),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id,
                    class_type,
                    outputs: resources::redis::RedisResourceSpec {}.outputs(),
                });
            }
        }

        if let Some(provider_id) = &settings.dda_provider {
            if !provider_id.is_empty() {
                log::info!("Creating dda resource provider '{}'", provider_id);
                let class_type = resources::dda::DdaResourceSpec {}.class_type();
                let provider_id = make_provider_id(provider_id);
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::dda::DDAResourceProvider::new(
                                data_plane.clone(),
                                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                    ("RESOURCE_CLASS_TYPE".to_string(), class_type.clone()),
                                    ("RESOURCE_PROVIDER_ID".to_string(), provider_id.clone()),
                                    ("NODE_ID".to_string(), node_id.to_string()),
                                ]))),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );

                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id,
                    class_type,
                    outputs: resources::dda::DdaResourceSpec {}.outputs(),
                });
            }
        }

        if let Some(settings) = &settings.ollama_provider {
            if !settings.host.is_empty() && !settings.provider.is_empty() {
                log::info!(
                    "Creating ollama resource provider '{}' towards {}:{} (limit to {} messages per chat)",
                    settings.provider,
                    settings.host,
                    settings.port,
                    settings.messages_number_limit
                );
                let class_type = resources::ollama::OllamaResourceSpec {}.class_type();
                let provider_id = make_provider_id(&settings.provider);
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::ollama::OllamaResourceProvider::new(
                                data_plane.clone(),
                                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                    ("RESOURCE_CLASS_TYPE".to_string(), class_type.clone()),
                                    ("RESOURCE_PROVIDER_ID".to_string(), settings.provider.clone()),
                                    ("NODE_ID".to_string(), node_id.to_string()),
                                ]))),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                                &settings.host,
                                settings.port,
                                settings.messages_number_limit,
                            )
                            .await,
                        ),
                    },
                );

                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id,
                    class_type,
                    outputs: resources::ollama::OllamaResourceSpec {}.outputs(),
                });
            }
        }

        if let Some(serverless_providers) = &settings.serverless_provider {
            for settings in serverless_providers {
                let mut is_function_url_valid = false;
                if let Ok((proto, address, _port)) = edgeless_api::util::parse_http_host(&settings.function_url) {
                    if !address.is_empty() && proto == edgeless_api::util::Proto::HTTP || proto == edgeless_api::util::Proto::HTTPS {
                        is_function_url_valid = true;
                    }
                }
                if is_function_url_valid && !settings.provider.is_empty() {
                    let provider_spec = resources::serverless::ServerlessResourceProviderSpec::new(&settings.class_type, &settings.version);
                    let provider_id = make_provider_id(&settings.provider);
                    log::info!(
                        "Creating '{}' (version {}) serverless resource provider '{}' at HTTP URL '{}'",
                        settings.class_type,
                        settings.version,
                        provider_id,
                        settings.function_url
                    );
                    ret.insert(
                        provider_id.clone(),
                        agent::ResourceDesc {
                            class_type: settings.class_type.clone(),
                            client: Box::new(
                                resources::serverless::ServerlessResourceProvider::new(
                                    data_plane.clone(),
                                    Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                        ("RESOURCE_CLASS_TYPE".to_string(), settings.class_type.clone()),
                                        ("RESOURCE_PROVIDER_ID".to_string(), settings.provider.clone()),
                                        ("NODE_ID".to_string(), node_id.to_string()),
                                    ]))),
                                    edgeless_api::function_instance::InstanceId::new(node_id),
                                    settings.function_url.clone(),
                                )
                                .await,
                            ),
                        },
                    );

                    provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                        provider_id,
                        class_type: settings.class_type.clone(),
                        outputs: provider_spec.outputs(),
                    });
                }
            }
        }

        if let Some(provider_id) = &settings.kafka_egress_provider {
            if !provider_id.is_empty() {
                #[cfg(feature = "rdkafka")]
                {
                    log::info!("Creating kakfa-egress resource provider '{}'", provider_id);
                    let class_type = resources::kafka_egress::KafkaEgressResourceSpec {}.class_type();
                    let provider_id = make_provider_id(provider_id);
                    ret.insert(
                        provider_id.clone(),
                        agent::ResourceDesc {
                            class_type: class_type.clone(),
                            client: Box::new(
                                resources::kafka_egress::KafkaEgressResourceProvider::new(
                                    data_plane.clone(),
                                    Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                        ("RESOURCE_CLASS_TYPE".to_string(), class_type.clone()),
                                        ("RESOURCE_PROVIDER_ID".to_string(), provider_id.clone()),
                                        ("NODE_ID".to_string(), node_id.to_string()),
                                    ]))),
                                    edgeless_api::function_instance::InstanceId::new(node_id),
                                )
                                .await,
                            ),
                        },
                    );
                    provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                        provider_id,
                        class_type,
                        outputs: resources::kafka_egress::KafkaEgressResourceSpec {}.outputs(),
                    });
                }
                #[cfg(not(feature = "rdkafka"))]
                log::error!(
                    "Could not create resource provider '{}' because rdkafka was disabled at compile time",
                    provider_id
                );
            }
        }

        if let Some(settings) = &settings.metrics_collector_provider {
            if !settings.provider.is_empty() {
                match settings.collector_type.to_lowercase().as_str() {
                    "redis" => match &settings.redis_url {
                        Some(redis_url) => {
                            match redis::Client::open(redis_url.clone()) {
                                Ok(client) => match client.get_connection() {
                                    Ok(redis_connection) => {
                                        let class_type = resources::metrics_collector::MetricsCollectorResourceSpec {}.class_type();
                                        let provider_id = make_provider_id(&settings.provider);
                                        log::info!(
                                            "Creating metrics-collector resource provider '{}' connected to a Redis server at {}",
                                            settings.provider,
                                            redis_url
                                        );
                                        ret.insert(
                                            provider_id.clone(),
                                            agent::ResourceDesc {
                                                class_type: class_type.clone(),
                                                client: Box::new(
                                                    resources::metrics_collector::MetricsCollectorResourceProvider::new(
                                                        data_plane.clone(),
                                                        Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                                            ("RESOURCE_CLASS_TYPE".to_string(), class_type.clone()),
                                                            ("RESOURCE_PROVIDER_ID".to_string(), settings.provider.clone()),
                                                            ("NODE_ID".to_string(), node_id.to_string()),
                                                        ]))),
                                                        edgeless_api::function_instance::InstanceId::new(node_id),
                                                        redis_connection,
                                                    )
                                                    .await,
                                                ),
                                            },
                                        );
                                        provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                                            provider_id,
                                            class_type,
                                            outputs: vec![],
                                        });

                                        log::info!("metrics collector connected to Redis at {}", redis_url);
                                    }
                                    Err(err) => log::error!("error when connecting to Redis at {}: {}", redis_url, err),
                                },
                                Err(err) => log::error!("error when creating a Redis client at {}: {}", redis_url, err),
                            };
                        }
                        None => {
                            log::error!("redis_url not specified for a Redis metrics collector");
                        }
                    },
                    _ => {
                        log::error!("unknown  metrics collector type");
                    }
                }
            }
        }

        if let Some(provider_id) = &settings.sqlx_provider {
            if !provider_id.is_empty() {
                log::info!("Creating resource '{}'", provider_id);
                let class_type = "sqlx".to_string();
                let provider_id = make_provider_id(provider_id);
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::sqlx::SqlxResourceProvider::new(
                                data_plane.clone(),
                                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                    ("RESOURCE_CLASS_TYPE".to_string(), class_type.clone()),
                                    ("RESOURCE_PROVIDER_ID".to_string(), provider_id.clone()),
                                    ("NODE_ID".to_string(), node_id.to_string()),
                                ]))),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id,
                    class_type,
                    outputs: vec![],
                });
            }
        }
    }

    ret
}

pub async fn edgeless_node_main(settings: EdgelessNodeSettings) {
    log::info!("Starting Edgeless Node");
    log::debug!("Settings: {:?}", settings);

    // Create the state manager.
    let state_manager = Box::new(state_management::StateManager::new().await);

    // Create the data plane.
    let data_plane = edgeless_dataplane::handle::DataplaneProvider::new(
        settings.general.node_id,
        settings.general.invocation_url.clone(),
        settings.general.invocation_url_coap.clone(),
    )
    .await;

    // Create the performance target.
    let telemetry_performance_target = edgeless_telemetry::performance_target::PerformanceTargetInner::new();

    // Create the telemetry provider.
    let telemetry_provider = match edgeless_telemetry::telemetry_events::TelemetryProcessor::new(
        settings.telemetry.metrics_url.clone(),
        settings.telemetry.log_level,
        if settings.telemetry.performance_samples {
            Some(telemetry_performance_target.clone())
        } else {
            None
        },
    )
    .await
    {
        Ok(telemetry_provider) => telemetry_provider,
        Err(err) => panic!("could not build the telemetry provider: {}", err),
    };

    // List of runners supported by this node to be filled below depending on
    // the node's configuration.
    let mut runners = std::collections::HashMap::<String, Box<dyn crate::base_runtime::RuntimeAPI + Send>>::new();

    // Create the WASM run-time, if needed.
    let rust_runtime_task = match settings.wasm_runtime {
        Some(wasm_runtime_settings) => {
            match wasm_runtime_settings.enabled {
                true => {
                    // Create the WebAssembly (Wasmtime) runner.
                    #[allow(unused_variables)]
                    #[cfg(feature = "wasmtime")]
                    {
                        let (wasmtime_runtime_client, mut wasmtime_runtime_task_s) =
                            base_runtime::runtime::create::<wasm_runner::function_instance::WASMFunctionInstance>(
                                data_plane.clone(),
                                state_manager.clone(),
                                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                    ("FUNCTION_TYPE".to_string(), "RUST_WASM".to_string()),
                                    ("WASM_RUNTIME".to_string(), "wasmtime".to_string()),
                                    ("NODE_ID".to_string(), settings.general.node_id.to_string()),
                                ]))),
                                std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(crate::wasm_runner::runtime::WasmRuntime::new()))),
                            );
                        runners.insert("RUST_WASM".to_string(), Box::new(wasmtime_runtime_client.clone()));
                        tokio::spawn(async move {
                            wasmtime_runtime_task_s.run().await;
                        })
                    }

                    // Create the WebAssembly (Wasmi) runner.
                    #[allow(unused_variables)]
                    #[cfg(feature = "wasmi")]
                    {
                        let (wasmi_runtime_client, mut wasmi_runtime_task_s) = base_runtime::runtime::create::<wasmi_runner::WASMIFunctionInstance>(
                            data_plane.clone(),
                            state_manager.clone(),
                            Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                                ("FUNCTION_TYPE".to_string(), "RUST_WASM".to_string()),
                                ("WASM_RUNTIME".to_string(), "wasmi".to_string()),
                                ("NODE_ID".to_string(), settings.general.node_id.to_string()),
                            ]))),
                            std::sync::Arc::new(tokio::sync::Mutex::new(Box::new(crate::wasmi_runner::runtime::WasmiRuntime::new()))),
                        );
                        runners.insert("RUST_WASM".to_string(), Box::new(wasmi_runtime_client.clone()));
                        tokio::spawn(async move {
                            wasmi_runtime_task_s.run().await;
                        })
                    }
                }
                false => tokio::spawn(async {}),
            }
        }
        None => tokio::spawn(async {}),
    };

    // Create the container run-time, if needed.
    let container_runtime_task = match settings.container_runtime {
        Some(container_runtime_settings) => match container_runtime_settings.enabled {
            true => {
                let (container_runtime, container_runtime_task, container_runtime_api) = container_runner::container_runtime::ContainerRuntime::new(
                    std::collections::HashMap::from([("guest_api_host_url".to_string(), container_runtime_settings.guest_api_host_url.clone())]),
                );
                let server_task = edgeless_api::grpc_impl::outer::container_runtime::GuestAPIHostServer::run(
                    container_runtime_api,
                    container_runtime_settings.guest_api_host_url,
                );

                let (container_runtime_client, mut container_runtime_task_s) =
                    base_runtime::runtime::create::<container_runner::function_instance::ContainerFunctionInstance>(
                        data_plane.clone(),
                        state_manager.clone(),
                        Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                            ("FUNCTION_TYPE".to_string(), "CONTAINER".to_string()),
                            ("NODE_ID".to_string(), settings.general.node_id.to_string()),
                        ]))),
                        container_runtime.clone(),
                    );
                runners.insert("CONTAINER".to_string(), Box::new(container_runtime_client.clone()));
                tokio::spawn(async move {
                    futures::join!(container_runtime_task_s.run(), container_runtime_task, server_task);
                })
            }
            false => tokio::spawn(async {}),
        },
        None => tokio::spawn(async {}),
    };

    // Create the resources.
    let mut resource_provider_specifications = vec![];
    let resources = fill_resources(
        data_plane.clone(),
        settings.general.node_id,
        &settings.resources,
        &mut resource_provider_specifications,
        &telemetry_provider,
    )
    .await;

    // Create the agent.
    let runtimes = runners.keys().map(|x| x.to_string()).collect::<Vec<String>>();
    let (mut agent, agent_task) = agent::Agent::new(runners, resources, settings.general.node_id, data_plane.clone());
    let agent_api_server = edgeless_api::grpc_impl::outer::agent::AgentAPIServer::run(agent.get_api_client(), settings.general.agent_url.clone());

    // Create the component that subscribes to the node register to
    // notify updates (periodically refreshed).
    let (_subscriber, subscriber_task, refresh_task) = node_subscriber::NodeSubscriber::new(
        settings.general,
        resource_provider_specifications.clone(),
        get_capabilities(runtimes, settings.user_node_capabilities.unwrap_or(NodeCapabilitiesUser::empty())),
        settings.power_info,
        telemetry_performance_target,
    )
    .await;

    // Wait for all the tasks to complete.
    let _ = futures::join!(
        rust_runtime_task,
        container_runtime_task,
        agent_task,
        agent_api_server,
        subscriber_task,
        refresh_task,
    );
}

pub fn edgeless_node_default_conf() -> String {
    let node_conf = EdgelessNodeSettings {
        general: EdgelessNodeGeneralSettings {
            node_id: uuid::Uuid::new_v4(),
            agent_url: String::from("http://127.0.0.1:7005"),
            agent_url_announced: String::from("http://127.0.0.1:7005"),
            invocation_url: String::from("http://127.0.0.1:7006"),
            invocation_url_announced: String::from("http://127.0.0.1:7006"),
            invocation_url_coap: None,
            invocation_url_announced_coap: None,
            node_register_url: String::from("http://127.0.0.1:7004"),
            subscription_refresh_interval_sec: 2,
        },
        telemetry: EdgelessNodeTelemetrySettings {
            metrics_url: String::from("http://127.0.0.1:7007"),
            log_level: Some(String::default()),
            performance_samples: false,
        },
        wasm_runtime: Some(EdgelessNodeWasmRuntimeSettings { enabled: true }),
        container_runtime: Some(EdgelessNodeContainerRuntimeSettings::default()),
        resources: Some(EdgelessNodeResourceSettings {
            prepend_hostname: true,
            http_ingress_url: Some(String::from("http://127.0.0.1:7008")),
            http_ingress_provider: Some("http-ingress-1".to_string()),
            http_egress_provider: Some("http-egress-1".to_string()),
            file_log_provider: Some("file-log-1".to_string()),
            redis_provider: Some("redis-1".to_string()),
            dda_provider: Some("dda-1".to_string()),
            ollama_provider: Some(OllamaProviderSettings::default()),
            serverless_provider: Some(vec![ServerlessProviderSettings::default()]),
            kafka_egress_provider: Some(String::default()),
            metrics_collector_provider: Some(MetricsCollectorProviderSettings::default()),
            sqlx_provider: Some(String::from("sqlite://sqlite.db")),
        }),
        user_node_capabilities: Some(NodeCapabilitiesUser::default()),
        power_info: None,
    };
    toml::to_string(&node_conf).expect("Wrong")
}
