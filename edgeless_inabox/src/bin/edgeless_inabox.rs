// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use std::path::Path;

use clap::Parser;
use edgeless_node::{
    EdgelessNodeContainerRuntimeSettings, EdgelessNodeGeneralSettings, EdgelessNodeResourceSettings, EdgelessNodeSettings,
    EdgelessNodeTelemetrySettings, OllamaProviderSettings, ServerlessProviderSettings,
};
use std::fs;

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
    /// Initial TCP port to be used for services.
    #[arg(long, default_value_t = 7000)]
    initial_port: u16,
    /// Use the first non-loopback IP address to bind local sockets instead of 127.0.0.1.
    #[arg(long, short)]
    bind_to_nonloopback: bool,
    /// Print the version number and quit.
    #[arg(long, default_value_t = false)]
    version: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    if args.version {
        println!(
            "{}.{}.{}{}{}",
            env!("CARGO_PKG_VERSION_MAJOR"),
            env!("CARGO_PKG_VERSION_MINOR"),
            env!("CARGO_PKG_VERSION_PATCH"),
            if env!("CARGO_PKG_VERSION_PRE").is_empty() { "" } else { "-" },
            env!("CARGO_PKG_VERSION_PRE")
        );
        return Ok(());
    }
    if args.templates {
        let last_port = generate_configs(args.config_path, args.num_of_nodes, args.initial_port, args.bind_to_nonloopback)?;
        log::info!("Templates written, last port used: {}", last_port);
        return Ok(());
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
fn generate_configs(config_path: String, number_of_nodes: u32, initial_port: u16, bind_to_nonloopback: bool) -> anyhow::Result<u16> {
    log::info!("Generating configuration files for EDGELESS in-a-box with {} nodes", number_of_nodes);

    let reserved_controller_port = 7001;
    let reserved_domain_register_port = 7002;

    let ip = match bind_to_nonloopback {
        true => edgeless_api::util::get_my_ip()?,
        false => String::from("127.0.0.1"),
    };

    // Closure that returns a URL with a new port on each call
    let mut port = initial_port - 1;
    let mut next_url = |any: bool| {
        port += 1;
        while port == reserved_controller_port || port == reserved_domain_register_port {
            port += 1;
        }
        if any && bind_to_nonloopback {
            format!("http://0.0.0.0:{}", port)
        } else {
            format!("http://{}:{}", ip, port)
        }
    };
    let announced_url = |url| {
        if bind_to_nonloopback {
            String::default()
        } else {
            url
        }
    };

    let controller_url = format!("http://{}:{}", ip, reserved_controller_port);
    let domain_register_url = format!("http://{}:{}", ip, reserved_domain_register_port);

    // Orchestrator
    let orchestrator_url = next_url(true);
    let orc_conf = edgeless_orc::EdgelessOrcSettings {
        general: edgeless_orc::EdgelessOrcGeneralSettings {
            domain_register_url: domain_register_url.clone(),
            subscription_refresh_interval_sec: 2,
            domain_id: format!("domain-{}", initial_port),
            orchestrator_url: orchestrator_url.clone(),
            orchestrator_url_announced: announced_url(orchestrator_url),
            node_register_url: next_url(false),
            node_register_coap_url: None,
        },
        baseline: edgeless_orc::EdgelessOrcBaselineSettings {
            orchestration_strategy: edgeless_orc::OrchestrationStrategy::Random,
        },
        proxy: edgeless_orc::EdgelessOrcProxySettings {
            proxy_type: "None".to_string(),
            proxy_gc_period_seconds: 0,
            redis_url: None,
            dataset_settings: None,
        },
    };

    // Controller
    let con_conf = edgeless_con::EdgelessConSettings {
        controller_url,
        domain_register_url,
        persistence_filename: "controller.save".to_string(),
    };

    // Nodes
    // Only the first node gets resources
    let mut node_confs: Vec<EdgelessNodeSettings> = vec![];
    for counter in 0..number_of_nodes {
        let agent_url = next_url(true);
        let invocation_url = next_url(true);
        node_confs.push(EdgelessNodeSettings {
            general: EdgelessNodeGeneralSettings {
                node_id: uuid::Uuid::new_v4(),
                agent_url: agent_url.clone(),
                agent_url_announced: announced_url(agent_url),
                invocation_url: invocation_url.clone(),
                invocation_url_announced: announced_url(invocation_url),
                invocation_url_coap: None,
                invocation_url_announced_coap: None,
                node_register_url: orc_conf.general.node_register_url.clone(),
                subscription_refresh_interval_sec: 2,
            },
            telemetry: EdgelessNodeTelemetrySettings {
                metrics_url: next_url(false),
                performance_samples: false,
            },
            wasm_runtime: Some(edgeless_node::EdgelessNodeWasmRuntimeSettings { enabled: true }),
            container_runtime: Some(EdgelessNodeContainerRuntimeSettings::default()),
            resources: Some(EdgelessNodeResourceSettings {
                prepend_hostname: true,
                http_ingress_url: match counter == 0 {
                    true => Some(next_url(false)),
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
                ollama_provider: Some(OllamaProviderSettings::default()),
                serverless_provider: Some(vec![ServerlessProviderSettings::default()]),
                kafka_egress_provider: Some(String::default()),
                sqlx_provider: match counter == 0 {
                    true => Some("sqlx-1".to_string()),
                    false => None,
                },
            }),
            user_node_capabilities: Some(edgeless_node::NodeCapabilitiesUser::default()),
            power_info: None,
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

    let single_node = node_confs.len() == 1;
    for (count, node_conf) in node_confs.into_iter().enumerate() {
        let filename = if single_node {
            String::from("node.toml")
        } else {
            format!("node{}.toml", count)
        };
        let node_file = Path::new(&config_path).join(filename);
        if node_file.exists() {
            log::warn!("File {:#?} exists and will not be overwritten", node_file);
        } else {
            std::fs::write(node_file, toml::to_string(&node_conf).expect("Wrong"))?;
        }
    }

    Ok(port)
}
