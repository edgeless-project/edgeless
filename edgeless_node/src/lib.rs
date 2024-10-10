// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_api::orc::OrchestratorAPI;
use futures::Future;

pub mod agent;
pub mod base_runtime;
pub mod container_runner;
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
    /// The URL of the orchestrator to which this node registers.
    pub orchestrator_url: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeResourceSettings {
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
    /// The URL of DDA used by this node, used for communication via the DDA resources
    pub dda_url: Option<String>,
    /// If not empty, a DDA resource with that name is created.
    pub dda_provider: Option<String>,
    /// The ollama resource provider settings.
    pub ollama_provider: Option<OllamaProviderSettings>,
    /// If not empty, a kafka-egress resource provider with that name is created.
    /// The resource will connect to a remote Kafka server to stream the
    /// messages received on a given topic.
    pub kafka_egress_provider: Option<String>,
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
        }
    }
}

impl EdgelessNodeSettings {
    /// Create settings for a node with WASM run-time and no resources
    /// binding the given ports on the same address.
    pub fn new_without_resources(orchestrator_url: &str, node_address: &str, agent_port: u16, invocation_port: u16, metrics_port: u16) -> Self {
        let agent_url = format!("http://{}:{}", node_address, agent_port);
        let invocation_url = format!("http://{}:{}", node_address, invocation_port);
        let invocation_url_coap = Some(format!("coap://{}:{}", node_address, invocation_port));
        Self {
            general: EdgelessNodeGeneralSettings {
                node_id: uuid::Uuid::new_v4(),
                agent_url: agent_url.clone(),
                agent_url_announced: agent_url,
                invocation_url: invocation_url.clone(),
                invocation_url_announced: invocation_url,
                invocation_url_coap: invocation_url_coap.clone(),
                invocation_url_announced_coap: invocation_url_coap,
                orchestrator_url: orchestrator_url.to_string(),
            },
            telemetry: EdgelessNodeTelemetrySettings {
                metrics_url: format!("http://{}:{}", node_address, metrics_port),
                log_level: None,
                performance_samples: false,
            },
            wasm_runtime: Some(EdgelessNodeWasmRuntimeSettings { enabled: true }),
            container_runtime: None,
            resources: None,
            user_node_capabilities: None,
        }
    }
}

fn get_capabilities(runtimes: Vec<String>, user_node_capabilities: NodeCapabilitiesUser) -> edgeless_api::node_registration::NodeCapabilities {
    let mut sys = sysinfo::System::new();
    sys.refresh_all();
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

    edgeless_api::node_registration::NodeCapabilities {
        num_cpus: user_node_capabilities.num_cpus.unwrap_or(sys.cpus().len() as u32),
        model_name_cpu: user_node_capabilities.model_name_cpu.unwrap_or(model_name_cpu),
        clock_freq_cpu: user_node_capabilities.clock_freq_cpu.unwrap_or(clock_freq_cpu),
        num_cores: user_node_capabilities.num_cores.unwrap_or(sys.physical_core_count().unwrap_or(1) as u32),
        mem_size: user_node_capabilities.mem_size.unwrap_or(sys.total_memory() as u32 / (1024 * 1024)),
        labels: user_node_capabilities.labels.unwrap_or_default(),
        is_tee_running: user_node_capabilities.is_tee_running.unwrap_or(false),
        has_tpm: user_node_capabilities.has_tpm.unwrap_or(false),
        runtimes,
    }
}

pub async fn register_node(
    settings: EdgelessNodeGeneralSettings,
    capabilities: edgeless_api::node_registration::NodeCapabilities,
    resource_provider_specifications: Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
) {
    log::info!(
        "Registering this node '{}' on e-ORC {}, capabilities: {}",
        &settings.node_id,
        &settings.orchestrator_url,
        capabilities
    );
    match edgeless_api::grpc_impl::orc::OrchestratorAPIClient::new(&settings.orchestrator_url, None).await {
        Ok(mut orc_client) => match orc_client
            .node_registration_api()
            .update_node(edgeless_api::node_registration::UpdateNodeRequest::Registration(
                settings.node_id,
                match settings.agent_url_announced.is_empty() {
                    true => settings.agent_url.clone(),
                    false => settings.agent_url_announced.clone(),
                },
                match settings.invocation_url_announced.is_empty() {
                    true => settings.invocation_url.clone(),
                    false => settings.invocation_url_announced.clone(),
                },
                resource_provider_specifications,
                capabilities,
            ))
            .await
        {
            Ok(res) => match res {
                edgeless_api::node_registration::UpdateNodeResponse::ResponseError(err) => {
                    panic!("could not register to e-ORC {}: {}", &settings.orchestrator_url, err)
                }
                edgeless_api::node_registration::UpdateNodeResponse::Accepted => {
                    log::info!("this node '{}' registered to e-ORC '{}'", &settings.node_id, &settings.orchestrator_url)
                }
            },
            Err(err) => panic!("channel error when registering to e-ORC {}: {}", &settings.orchestrator_url, err),
        },
        Err(err) => panic!("could not connect to e-ORC {}: {}", &settings.orchestrator_url, err),
    }
}

async fn fill_resources(
    data_plane: edgeless_dataplane::handle::DataplaneProvider,
    node_id: uuid::Uuid,
    settings: &Option<EdgelessNodeResourceSettings>,
    provider_specifications: &mut Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
) -> std::collections::HashMap<String, agent::ResourceDesc> {
    let mut ret = std::collections::HashMap::<String, agent::ResourceDesc>::new();

    if let Some(settings) = settings {
        if let (Some(http_ingress_url), Some(provider_id)) = (&settings.http_ingress_url, &settings.http_ingress_provider) {
            if !http_ingress_url.is_empty() && !provider_id.is_empty() {
                let class_type = "http-ingress".to_string();
                log::info!("Creating resource '{}' at {}", provider_id, http_ingress_url);
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
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec!["new_request".to_string()],
                });
            }
        }

        if let Some(provider_id) = &settings.http_egress_provider {
            if !provider_id.is_empty() {
                log::info!("Creating resource '{}'", provider_id);
                let class_type = "http-egress".to_string();
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::http_egress::EgressResourceProvider::new(
                                data_plane.clone(),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec![],
                });
            }
        }

        if let Some(provider_id) = &settings.file_log_provider {
            if !provider_id.is_empty() {
                log::info!("Creating resource '{}'", provider_id);
                let class_type = "file-log".to_string();
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::file_log::FileLogResourceProvider::new(
                                data_plane.clone(),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec![],
                });
            }
        }

        if let Some(provider_id) = &settings.redis_provider {
            if !provider_id.is_empty() {
                log::info!("Creating resource '{}'", provider_id);
                let class_type = "redis".to_string();
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::redis::RedisResourceProvider::new(
                                data_plane.clone(),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec![],
                });
            }
        }

        if let (Some(dda_url), Some(provider_id)) = (&settings.dda_url, &settings.dda_provider) {
            if !dda_url.is_empty() && !provider_id.is_empty() {
                log::info!("Creating resource '{}' at {}", provider_id, dda_url);
                let class_type = "dda".to_string();
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::dda::DDAResourceProvider::new(data_plane.clone(), edgeless_api::function_instance::InstanceId::new(node_id))
                                .await,
                        ),
                    },
                );

                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec!["new_request".to_string()],
                });
            }
        }

        if let Some(settings) = &settings.ollama_provider {
            if !settings.host.is_empty() && !settings.provider.is_empty() {
                log::info!(
                    "Creating resource '{}' towards {}:{} (limit to {} messages per chat)",
                    settings.provider,
                    settings.host,
                    settings.port,
                    settings.messages_number_limit
                );
                let class_type = "ollama".to_string();
                ret.insert(
                    settings.provider.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::ollama::OllamaResourceProvider::new(
                                data_plane.clone(),
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
                    provider_id: settings.provider.clone(),
                    class_type,
                    outputs: vec!["out".to_string()],
                });
            }
        }

        if let Some(provider_id) = &settings.kafka_egress_provider {
            if !provider_id.is_empty() {
                log::info!("Creating resource '{}'", provider_id);
                let class_type = "kafka-egress".to_string();
                ret.insert(
                    provider_id.clone(),
                    agent::ResourceDesc {
                        class_type: class_type.clone(),
                        client: Box::new(
                            resources::kafka_egress::KafkaEgressResourceProvider::new(
                                data_plane.clone(),
                                edgeless_api::function_instance::InstanceId::new(node_id),
                            )
                            .await,
                        ),
                    },
                );
                provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                    provider_id: provider_id.clone(),
                    class_type,
                    outputs: vec![],
                });
            }
        }
    }

    ret
}

pub async fn create_metrics_collector_node(
    invocation_url: String,
    agent_url: String,
    metrics_collector_type: String,
    redis_url: Option<String>,
) -> (
    uuid::Uuid,
    std::pin::Pin<Box<dyn Future<Output = ()> + Send>>,
    futures::future::BoxFuture<'static, ()>,
    Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
) {
    // Create the data plane for the node.
    let node_id = uuid::Uuid::new_v4();
    let data_plane = edgeless_dataplane::handle::DataplaneProvider::new(node_id, invocation_url, None).await;

    // Create the metrics collector resource.
    let mut resource_provider_specifications = vec![];
    let mut resources: std::collections::HashMap<String, agent::ResourceDesc> = std::collections::HashMap::new();

    match metrics_collector_type.to_lowercase().as_str() {
        "redis" => match redis_url {
            Some(redis_url) => {
                match redis::Client::open(redis_url.clone()) {
                    Ok(client) => match client.get_connection() {
                        Ok(redis_connection) => {
                            let provider_id = "metrics-collector".to_string();
                            let class_type = "metrics-collector".to_string();
                            resource_provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
                                provider_id: provider_id.clone(),
                                class_type: class_type.clone(),
                                outputs: vec![],
                            });
                            resources.insert(
                                provider_id,
                                agent::ResourceDesc {
                                    class_type,
                                    client: Box::new(
                                        resources::metrics_collector::MetricsCollectorResourceProvider::new(
                                            data_plane.clone(),
                                            edgeless_api::function_instance::InstanceId::new(node_id),
                                            redis_connection,
                                        )
                                        .await,
                                    ),
                                },
                            );
                            log::info!("metrics collector connected to Redis at {}", redis_url);
                        }
                        Err(err) => log::error!("error when connecting to Redis at {}: {}", redis_url, err),
                    },
                    Err(err) => log::error!("error when creating a Redis client at {}: {}", redis_url, err),
                };
            }
            None => {
                log::warn!("redis_url not specified for a Redis metrics collector");
            }
        },
        _ => {
            log::info!("no metrics collector is used");
        }
    }

    // Create the agent of the node embedded in the orchestrator.
    let telemetry_performance_target = edgeless_telemetry::performance_target::PerformanceTargetInner::new();
    let (mut agent, agent_task) = agent::Agent::new(
        std::collections::HashMap::new(),
        resources,
        node_id,
        data_plane.clone(),
        telemetry_performance_target,
    );
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), agent_url);

    (node_id, agent_task, agent_api_server, resource_provider_specifications)
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
                let server_task = edgeless_api::grpc_impl::container_runtime::GuestAPIHostServer::run(
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
    )
    .await;

    // Create the agent.
    let runtimes = runners.keys().map(|x| x.to_string()).collect::<Vec<String>>();
    let (mut agent, agent_task) = agent::Agent::new(
        runners,
        resources,
        settings.general.node_id,
        data_plane.clone(),
        telemetry_performance_target,
    );
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.general.agent_url.clone());

    // Wait for all the tasks to complete.
    let _ = futures::join!(
        rust_runtime_task,
        container_runtime_task,
        agent_task,
        agent_api_server,
        register_node(
            settings.general,
            get_capabilities(runtimes, settings.user_node_capabilities.unwrap_or(NodeCapabilitiesUser::empty())),
            resource_provider_specifications
        )
    );
}

pub fn edgeless_node_default_conf() -> String {
    let caps = get_capabilities(vec!["RUST_WASM".to_string()], NodeCapabilitiesUser::empty());

    format!(
        "[general]\nnode_id = \"{}\"\n{}num_cpus = {}\nmodel_name_cpu = \"{}\"\nclock_freq_cpu = {}\nnum_cores = {}\nmem_size = {}\n{}",
        uuid::Uuid::new_v4(),
        r##"
agent_url = "http://127.0.0.1:7021"
agent_url_announced = ""
invocation_url = "http://127.0.0.1:7002"
invocation_url_announced = ""
invocation_url_coap = "coap://127.0.0.1:7002"
invocation_url_announced_coap = ""
orchestrator_url = "http://127.0.0.1:7011"

[telemetry]
metrics_url = "http://127.0.0.1:7003"
log_level = "info"
performance_samples = true

[wasm_runtime]
enabled = true

[container_runtime]
enabled = false
guest_api_host_url = "http://127.0.0.1:7100"

[resources]
http_ingress_url = "http://127.0.0.1:7035"
http_ingress_provider = "http-ingress-1"
http_egress_provider = "http-egress-1"
file_log_provider = "file-log-1"
redis_provider = "redis-1"
dda_url = "http://127.0.0.1:10000"
dda_provider = "dda-1"
kafka_egress_provider = "kafka-egress-1"

[resources.ollama_provider]
host = "localhost"
port = 11434
messages_number_limit = 30
provider = "ollama-1"

[user_node_capabilities]
"##,
        caps.num_cpus,
        caps.model_name_cpu,
        caps.clock_freq_cpu,
        caps.num_cores,
        caps.mem_size,
        r##"labels = []
is_tee_running = false
has_tpm = false"##
    )
}
