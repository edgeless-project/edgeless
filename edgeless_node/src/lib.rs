// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use edgeless_api::orc::OrchestratorAPI;
use sysinfo::{ProcessorExt, SystemExt};

pub mod agent;
pub mod base_runtime;
pub mod resources;
pub mod state_management;
#[cfg(feature = "wasmtime")]
pub mod wasm_runner;
#[cfg(feature = "wasmi")]
pub mod wasmi_runner;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EdgelessNodeSettings {
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
    /// The URL exposed by this node to publish telemetry metrics collected.
    pub metrics_url: String,
    /// The URL of the orchestrator to which this node registers.
    pub orchestrator_url: String,
    /// If `http_ingress_provider` is not empty, this is the URL of the
    /// HTTP web server exposed by the http-ingress resource for this node.
    pub http_ingress_url: String,
    pub dda_sidecar_url: String,
    /// If not empty, a http-ingress resource with the given name is created.
    pub http_ingress_provider: String,
    /// If not empty, a http-egress resource with the given name is created.
    pub http_egress_provider: String,
    /// If not empty, a file-log resource with the given name is created.
    /// The resource will write on the local filesystem.
    pub file_log_provider: String,
    /// If not empty, a redis resource with the given name is created.
    /// The resource will connect to a remote Redis server to update the
    /// value of a given given, as specified in the resource configuration
    /// at run-time.
    pub redis_provider: String,
}

impl EdgelessNodeSettings {
    /// Create settings for a node with no resources binding the given ports on the same address.
    pub fn new_without_resources(orchestrator_url: &str, node_address: &str, agent_port: u16, invocation_port: u16, metrics_port: u16) -> Self {
        let agent_url = format!("http://{}:{}", node_address, agent_port);
        let invocation_url = format!("http://{}:{}", node_address, invocation_port);
        Self {
            node_id: uuid::Uuid::new_v4(),
            agent_url: agent_url.clone(),
            agent_url_announced: agent_url,
            invocation_url: invocation_url.clone(),
            invocation_url_announced: invocation_url,
            metrics_url: format!("http://{}:{}", node_address, metrics_port),
            orchestrator_url: orchestrator_url.to_string(),
            http_ingress_url: "".to_string(),
            http_ingress_provider: "".to_string(),
            http_egress_provider: "".to_string(),
            file_log_provider: "".to_string(),
            redis_provider: "".to_string(),
        }
    }
}

fn get_capabilities() -> edgeless_api::node_registration::NodeCapabilities {
    let s = sysinfo::System::new();
    let mut model_name_set = std::collections::HashSet::new();
    let mut clock_freq_cpu_set = std::collections::HashSet::new();
    for processor in s.get_processors() {
        model_name_set.insert(processor.get_brand());
        clock_freq_cpu_set.insert(processor.get_frequency());
    }
    let model_name_cpu = match model_name_set.iter().next() {
        Some(val) => val.to_string(),
        None => "".to_string(),
    };
    if model_name_set.len() > 1 {
        log::warn!("CPUs have different models, using: {}", model_name_cpu);
    }
    let clock_freq_cpu = match clock_freq_cpu_set.iter().next() {
        Some(val) => *val as f32,
        None => 0.0,
    };
    if clock_freq_cpu_set.len() > 1 {
        log::warn!("CPUs have different frequencies, using: {}", clock_freq_cpu);
    }
    edgeless_api::node_registration::NodeCapabilities {
        num_cpus: s.get_processors().len() as u32,
        model_name_cpu,
        clock_freq_cpu,
        num_cores: 1,
        mem_size: s.get_total_memory() as u32 / 1024,
    }
}

pub async fn register_node(
    settings: &EdgelessNodeSettings,
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
                settings.node_id.clone(),
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

/// TODO: (docs) this actually just creates the resource providers, maybe rename
/// creates resource providers, but does not call ::start on the resources?
async fn fill_resources(
    data_plane: edgeless_dataplane::handle::DataplaneProvider,
    settings: &EdgelessNodeSettings,
    provider_specifications: &mut Vec<edgeless_api::node_registration::ResourceProviderSpecification>,
) -> std::collections::HashMap<
    String,
    Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
> {
    // empty hashmap for resources
    let mut ret = std::collections::HashMap::<
        String,
        Box<dyn edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId>>,
    >::new();

    // http_ingress is only started if it's configured in the settings
    // there is one global task per node for all http ingress events! - they are
    // then forwarded to concrete instances of ingress resource
    if !settings.http_ingress_url.is_empty() && !settings.http_ingress_provider.is_empty() {
        log::info!("Creating resource 'http-ingress-1' at {}", &settings.http_ingress_url);
        ret.insert(
            settings.http_ingress_provider.clone(),
            // The global singleton task is started here
            resources::http_ingress::ingress_task(
                data_plane.clone(),
                edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
                settings.http_ingress_url.clone(),
            )
            .await,
        );
        provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
            provider_id: settings.http_ingress_provider.clone(),
            class_type: "http-ingress".to_string(),
            // TODO: (docs) what is the meaning of this?
            // in http_ingress example it is used to specify which function
            // should handle the new_request event to the ingress in a workflow
            // (entry point of http to the workflow)
            outputs: vec!["new_request".to_string()],
        });
    }

    if !settings.http_egress_provider.is_empty() {
        log::info!("Creating resource 'http-egress-1'");
        ret.insert(
            settings.http_egress_provider.clone(),
            Box::new(
                resources::http_egress::EgressResourceProvider::new(
                    data_plane.clone(),
                    edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
                )
                .await,
            ),
        );
        provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
            provider_id: settings.http_egress_provider.clone(),
            class_type: "http-egress".to_string(),
            outputs: vec![],
        });
    }

    if !settings.file_log_provider.is_empty() {
        log::info!("Creating resource 'file-log-1'");
        ret.insert(
            settings.file_log_provider.clone(),
            Box::new(
                resources::file_log::FileLogResourceProvider::new(
                    data_plane.clone(),
                    edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
                )
                .await,
            ),
        );
        provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
            provider_id: settings.file_log_provider.clone(),
            class_type: "file-log".to_string(),
            outputs: vec![],
        });
    }

    log::info!("Creating resource 'redis-1'");
    ret.insert(
        "redis-1".to_string(),
        Box::new(
            resources::redis::RedisResourceProvider::new(
                data_plane.clone(),
                edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
            )
            .await,
        ),
    );
    provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
        provider_id: "redis-1".to_string(),
        class_type: "redis".to_string(),
        outputs: vec![],
    });

    log::info!("Creating resource 'dda-1'");
    ret.insert(
        "dda-1".to_string(),
        Box::new(
            resources::dda::DDAResourceProvider::new(
                data_plane.clone(),
                edgeless_api::function_instance::InstanceId::new(settings.node_id.clone()),
            )
            .await,
        ),
    );
    provider_specifications.push(edgeless_api::node_registration::ResourceProviderSpecification {
        provider_id: "dda-1".to_string(),
        class_type: "dda".to_string(),
        outputs: vec![],
    });

    ret
}

pub async fn edgeless_node_main(settings: EdgelessNodeSettings) {
    log::info!("Starting Edgeless Node");
    log::debug!("Settings: {:?}", settings);

    // Create the state manager.
    let state_manager = Box::new(state_management::StateManager::new().await);

    // Create the data plane.
    let data_plane = edgeless_dataplane::handle::DataplaneProvider::new(settings.node_id.clone(), settings.invocation_url.clone()).await;

    // Create the telemetry provider.
    let telemetry_provider = edgeless_telemetry::telemetry_events::TelemetryProcessor::new(settings.metrics_url.clone())
        .await
        .expect(&format!("could not build the telemetry provider at URL {}", &settings.metrics_url));

    let mut runners = std::collections::HashMap::<String, Box<dyn crate::base_runtime::RuntimeAPI + Send>>::new();

    // Create the WebAssembly (Wasmtime) runner.
    #[allow(unused_variables)]
    #[cfg(feature = "wasmtime")]
    let rust_runtime_task = {
        let (wasmtime_runtime_client, mut wasmtime_runtime_task_s) =
            base_runtime::runtime::create::<wasm_runner::function_instance::WASMFunctionInstance>(
                data_plane.clone(),
                state_manager.clone(),
                Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                    ("FUNCTION_TYPE".to_string(), "RUST_WASM".to_string()),
                    ("WASM_RUNTIME".to_string(), "wasmtime".to_string()),
                    ("NODE_ID".to_string(), settings.node_id.to_string()),
                ]))),
            );
        runners.insert("RUST_WASM".to_string(), Box::new(wasmtime_runtime_client.clone()));
        tokio::spawn(async move {
            wasmtime_runtime_task_s.run().await;
        })
    };

    // Create the WebAssembly (Wasmi) runner.
    #[allow(unused_variables)]
    #[cfg(feature = "wasmi")]
    let rust_runtime_task = {
        let (wasmi_runtime_client, mut wasmi_runtime_task_s) = base_runtime::runtime::create::<wasmi_runner::WASMIFunctionInstance>(
            data_plane.clone(),
            state_manager.clone(),
            Box::new(telemetry_provider.get_handle(std::collections::BTreeMap::from([
                ("FUNCTION_TYPE".to_string(), "RUST_WASM".to_string()),
                ("WASM_RUNTIME".to_string(), "wasmi".to_string()),
                ("NODE_ID".to_string(), settings.node_id.to_string()),
            ]))),
        );
        runners.insert("RUST_WASM".to_string(), Box::new(wasmi_runtime_client.clone()));
        tokio::spawn(async move {
            wasmi_runtime_task_s.run().await;
        })
    };

    // Create the resources.
    let mut resource_provider_specifications = vec![];
    let resources = fill_resources(data_plane.clone(), &settings, &mut resource_provider_specifications).await;

    // Create the agent.
    let (mut agent, agent_task) = agent::Agent::new(runners, resources, settings.clone(), data_plane.clone());
    let agent_api_server = edgeless_api::grpc_impl::agent::AgentAPIServer::run(agent.get_api_client(), settings.agent_url.clone());

    // Wait for all the tasks to complete.
    let _ = futures::join!(
        rust_runtime_task,
        agent_task,
        agent_api_server,
        register_node(&settings, get_capabilities(), resource_provider_specifications)
    );
}

pub fn edgeless_node_default_conf() -> String {
    String::from(
        r##"node_id = "fda6ce79-46df-4f96-a0d2-456f720f606c"
agent_url = "http://127.0.0.1:7021"
agent_url_announced = ""
invocation_url = "http://127.0.0.1:7002"
invocation_url_announced = ""
metrics_url = "http://127.0.0.1:7003"
orchestrator_url = "http://127.0.0.1:7011"
http_ingress_url = "http://127.0.0.1:7035"
http_ingress_provider = "http-ingress-1"
http_egress_provider = "http-egress-1"
file_log_provider = "file-log-1"
redis_provider = "redis-1"
dda_sidecar_url = "http://127.0.0.1:10000"
"##,
    )
}
