// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use std::path::Path;

use clap::Parser;
use edgeless_node::{EdgelessNodeGeneralSettings, EdgelessNodeResourceSettings, EdgelessNodeSettings, EdgelessNodeTelemetrySettings};
use std::fs;
use uuid::Uuid;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    /// Generate templates instead of running the services.
    #[arg(long, short)]
    templates: bool,
    /// Directory in which to save the configuration files.
    #[arg(long, default_value_t = String::from("./"))]
    config_path: String,
    /// When generating templates, add this number of nodes per domain.
    #[arg(long, short, default_value_t = 1)]
    num_of_nodes: u32,
    /// When generating templates, add a metrics-collector node.
    /// This flag also automatically enables a Redis proxy at redis://127.0.0.1:6379.
    #[arg(long, default_value_t = false)]
    metrics_collector: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    if args.templates {
        return generate_configs(args.config_path, args.num_of_nodes, args.metrics_collector);
    }

    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let mut async_tasks = vec![];

    edgeless_inabox::edgeless_inabox_main(&async_runtime, &mut async_tasks)?;

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}

/// Generates configs for a minimal in-a-box edgeless cluster with
/// number_of_nodes nodes in the directory. If directory is non-empty, it
/// fails.
fn generate_configs(config_path: String, number_of_nodes: u32, metrics_collector: bool) -> anyhow::Result<()> {
    log::info!("Generating configuration files for EDGELESS in-a-box with {} nodes", number_of_nodes);

    // Closure that returns a url with a new port on each call
    let mut port = 7000;
    let mut next_url = || {
        port += 1;
        format!("http://127.0.0.1:{}", port)
    };

    let controller_url = next_url();
    let domain_register_url = next_url();

    // Balancer
    let bal_conf = edgeless_bal::EdgelessBalSettings {
        balancer_id: Uuid::new_v4(),
        invocation_url: next_url(),
    };

    // Orchestrator
    let orc_conf = edgeless_orc::EdgelessOrcSettings {
        general: edgeless_orc::EdgelessOrcGeneralSettings {
            domain_register_url: domain_register_url.clone(),
            subscription_refresh_interval_sec: 1,
            domain_id: "domain-1".to_string(),
            orchestrator_url: next_url(),
            orchestrator_url_announced: "".to_string(),
            node_register_url: next_url(),
            node_register_coap_url: None,
        },
        baseline: edgeless_orc::EdgelessOrcBaselineSettings {
            orchestration_strategy: edgeless_orc::OrchestrationStrategy::Random,
        },
        proxy: match metrics_collector {
            true => edgeless_orc::EdgelessOrcProxySettings {
                proxy_type: "Redis".to_string(),
                redis_url: Some(String::from("redis://127.0.0.1:6379")),
                dataset_settings: None,
            },
            false => edgeless_orc::EdgelessOrcProxySettings {
                proxy_type: "None".to_string(),
                redis_url: None,
                dataset_settings: None,
            },
        },
    };

    // Controller
    let con_conf = edgeless_con::EdgelessConSettings {
        controller_url,
        domain_register_url,
    };

    // Nodes
    // Only the first node gets resources
    let mut node_confs: Vec<EdgelessNodeSettings> = vec![];
    for counter in 0..number_of_nodes {
        node_confs.push(EdgelessNodeSettings {
            general: EdgelessNodeGeneralSettings {
                node_id: uuid::Uuid::new_v4(),
                agent_url: next_url(),
                agent_url_announced: "".to_string(),
                invocation_url: next_url(),
                invocation_url_announced: "".to_string(),
                invocation_url_coap: None,
                invocation_url_announced_coap: None,
                node_register_url: orc_conf.general.node_register_url.clone(),
                subscription_refresh_interval_sec: 10,
            },
            telemetry: EdgelessNodeTelemetrySettings {
                metrics_url: next_url(),
                log_level: None,
                performance_samples: false,
            },
            wasm_runtime: Some(edgeless_node::EdgelessNodeWasmRuntimeSettings { enabled: true }),
            container_runtime: None,
            resources: Some(EdgelessNodeResourceSettings {
                http_ingress_url: match counter == 0 {
                    true => Some(next_url()),
                    false => None,
                },
                http_ingress_provider: match counter == 0 {
                    true => Some("http-ingress-1".to_string()),
                    false => None,
                },
                http_egress_provider: match counter == 0 {
                    true => Some("http-egress-1".to_string()),
                    false => None,
                },
                file_log_provider: match counter == 0 {
                    true => Some("file-log-1".to_string()),
                    false => None,
                },
                redis_provider: match counter == 0 {
                    true => Some("redis-1".to_string()),
                    false => None,
                },
                dda_provider: match counter == 0 {
                    true => Some("dda-1".to_string()),
                    false => None,
                },
                ollama_provider: None,
                kafka_egress_provider: None,
                metrics_collector_provider: None,
            }),
            user_node_capabilities: None,
        });
    }

    if metrics_collector {
        node_confs.push(EdgelessNodeSettings {
            general: EdgelessNodeGeneralSettings {
                node_id: uuid::Uuid::new_v4(),
                agent_url: next_url(),
                agent_url_announced: "".to_string(),
                invocation_url: next_url(),
                invocation_url_announced: "".to_string(),
                invocation_url_coap: None,
                invocation_url_announced_coap: None,
                node_register_url: orc_conf.general.node_register_url.clone(),
                subscription_refresh_interval_sec: 10,
            },
            telemetry: EdgelessNodeTelemetrySettings {
                metrics_url: next_url(),
                log_level: None,
                performance_samples: false,
            },
            wasm_runtime: None,
            container_runtime: None,
            resources: Some(EdgelessNodeResourceSettings {
                http_ingress_url: None,
                http_ingress_provider: None,
                http_egress_provider: None,
                file_log_provider: None,
                redis_provider: None,
                dda_provider: None,
                ollama_provider: None,
                kafka_egress_provider: None,
                metrics_collector_provider: Some(edgeless_node::MetricsCollectorProviderSettings {
                    collector_type: String::from("Redis"),
                    redis_url: Some(String::from("redis://127.0.0.1:6379")),
                    provider: String::from("metrics-collector-1"),
                }),
            }),
            user_node_capabilities: None,
        });
    }

    // Try to create the directory if it does not exist.
    if fs::metadata(&config_path).is_err() {
        if let Err(_err) = fs::create_dir(&config_path) {
            anyhow::bail!("Failed with creating directory: {}", &config_path);
        }
    }

    // Write files (without overwriting).
    let orc_file = Path::new(&config_path).join("orchestrator.toml");
    if orc_file.exists() {
        log::warn!("File {:#?} exists and will not be overwritten", orc_file);
    } else {
        std::fs::write(orc_file, toml::to_string(&orc_conf).expect("Wrong"))?;
    }

    let con_file = Path::new(&config_path).join("controller.toml");
    if con_file.exists() {
        log::warn!("File {:#?} exists and will not be overwritten", con_file);
    } else {
        std::fs::write(con_file, toml::to_string(&con_conf).expect("Wrong"))?;
    }

    let bal_file = Path::new(&config_path).join("balancer.toml");
    if bal_file.exists() {
        log::warn!("File {:#?} exists and will not be overwritten", bal_file);
    } else {
        std::fs::write(bal_file, toml::to_string(&bal_conf).expect("Wrong"))?;
    }

    for (count, node_conf) in node_confs.into_iter().enumerate() {
        let node_file = Path::new(&config_path).join(format!("node{}.toml", count));
        if node_file.exists() {
            log::warn!("File {:#?} exists and will not be overwritten", node_file);
        } else {
            std::fs::write(node_file, toml::to_string(&node_conf).expect("Wrong"))?;
        }
    }

    Ok(())
}
