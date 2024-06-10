// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use std::{collections::HashMap, path::Path};

use anyhow::anyhow;
use clap::Parser;
use edgeless_con::EdgelessConOrcConfig;
use edgeless_inabox::InABoxConfig;
use edgeless_node::{EdgelessNodeGeneralSettings, EdgelessNodeResourceSettings, EdgelessNodeSettings};
use std::fs;
use uuid::Uuid;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    /// Legacy option to generate configs for minimal cluster with only one
    /// worker node. Configs are generated at the root of the repository.
    #[arg(long, short)]
    templates: bool,
    /// Runs the edgeless-in-a-box with the specified number of worker nodes and
    /// saves the config files to configs/ directory for reference. Currently,
    /// it only supports generating new configs on each use, not reusing old files.
    #[arg(long, default_value_t = 1)]
    num_of_nodes: i32,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    if args.templates && args.num_of_nodes == 1 {
        log::info!("Generating default templates for one node");
        edgeless_api::util::create_template("node.toml", edgeless_node::edgeless_node_default_conf().as_str())?;
        edgeless_api::util::create_template("orchestrator.toml", edgeless_orc::edgeless_orc_default_conf().as_str())?;
        edgeless_api::util::create_template("balancer.toml", edgeless_bal::edgeless_bal_default_conf().as_str())?;
        edgeless_api::util::create_template("controller.toml", edgeless_con::edgeless_con_default_conf().as_str())?;
        return Ok(());
    }

    let config = match args.num_of_nodes {
        1 => InABoxConfig {
            node_conf_files: vec!["node.toml".to_string()],
            orc_conf_file: "orchestrator.toml".to_string(),
            bal_conf_file: "balancer.toml".to_string(),
            con_conf_file: "controller.toml".to_string(),
        },
        _ if args.num_of_nodes >= 2 => match generate_configs(args.num_of_nodes) {
            Ok(config) => config,
            Err(error) => return Err(anyhow!(error)),
        },
        _ => {
            return Err(anyhow!("Invalid number of worker nodes specified: {}", args.num_of_nodes));
        }
    };

    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let mut async_tasks = vec![];

    edgeless_inabox::edgeless_inabox_main(&async_runtime, &mut async_tasks, config)?;

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}

/// Generates configs for a minimal in-a-box edgeless cluster with
/// number_of_nodes nodes in the directory. If directory is non-empty, it
/// fails.
fn generate_configs(number_of_nodes: i32) -> Result<InABoxConfig, String> {
    log::info!(
        "Generating configuration files for Edgeless in a box with {} worker nodes",
        number_of_nodes
    );

    // Closure that returns a url with a new port on each call
    let mut port = 7000;
    let mut next_url = || {
        port += 1;
        format!("http://127.0.0.1:{}", port)
    };
    let controller_url = next_url();

    // At first generate endpoints for invocation_urls and orc_agent_urls
    let mut node_invocation_urls: HashMap<Uuid, String> = HashMap::new();
    let mut node_orc_agent_urls: HashMap<Uuid, String> = HashMap::new();

    for _ in 0..number_of_nodes {
        let node_id = Uuid::new_v4();
        node_invocation_urls.insert(node_id, next_url());
        node_orc_agent_urls.insert(node_id, next_url());
    }

    // Balancer
    let bal_conf = edgeless_bal::EdgelessBalSettings {
        balancer_id: Uuid::new_v4(),
        invocation_url: next_url(),
    };

    // Orchestrator
    let orc_conf = edgeless_orc::EdgelessOrcSettings {
        general: edgeless_orc::EdgelessOrcGeneralSettings {
            domain_id: "domain-1".to_string(),
            orchestrator_url: next_url(),
            agent_url: next_url(),
            invocation_url: next_url(),
        },
        baseline: edgeless_orc::EdgelessOrcBaselineSettings {
            orchestration_strategy: edgeless_orc::OrchestrationStrategy::Random,
            keep_alive_interval_secs: 2,
        },
        proxy: edgeless_orc::EdgelessOrcProxySettings {
            proxy_type: "None".to_string(),
            redis_url: None,
        },
    };

    // Controller
    let con_conf = edgeless_con::EdgelessConSettings {
        controller_url,
        // for now only one orchestrator
        orchestrators: vec![EdgelessConOrcConfig {
            domain_id: orc_conf.general.domain_id.clone(),
            orchestrator_url: orc_conf.general.orchestrator_url.clone(),
        }],
    };

    // Nodes
    // Only the first node gets resources
    let mut node_confs: Vec<EdgelessNodeSettings> = vec![];
    let mut first_node = true;
    for node_id in node_invocation_urls.keys() {
        node_confs.push(EdgelessNodeSettings {
            general: EdgelessNodeGeneralSettings {
                node_id: *node_id,
                agent_url: node_orc_agent_urls.get(node_id).expect("").clone(), // we are sure that it is there
                agent_url_announced: "".to_string(),
                invocation_url: node_invocation_urls.get(node_id).expect("").clone(), // we are sure that it is there
                invocation_url_announced: "".to_string(),
                metrics_url: next_url(),
                orchestrator_url: orc_conf.general.orchestrator_url.clone(),
            },
            wasm_runtime: Some(edgeless_node::EdgelessNodeWasmRuntimeSettings { enabled: true }),
            container_runtime: None,
            resources: Some(EdgelessNodeResourceSettings {
                http_ingress_url: match first_node {
                    true => Some(next_url()),
                    false => None,
                },
                http_ingress_provider: match first_node {
                    true => Some("http-ingress-1".to_string()),
                    false => None,
                },
                http_egress_provider: match first_node {
                    true => Some("http-egress-1".to_string()),
                    false => None,
                },
                file_log_provider: match first_node {
                    true => Some("file-log-1".to_string()),
                    false => None,
                },
                redis_provider: match first_node {
                    true => Some("redis-1".to_string()),
                    false => None,
                },
                dda_url: match first_node {
                    true => Some(next_url()),
                    false => None,
                },
                dda_provider: match first_node {
                    true => Some("dda-1".to_string()),
                    false => None,
                },
            }),
            user_node_capabilities: None,
        });
        first_node = false;
    }

    // Save the config files to a hard-coded directory if its empty, to give
    // users reference on how the cluster is configured
    let path = "config/";
    if fs::metadata(path).is_ok() {
        let is_empty = fs::read_dir(path).map(|entries| entries.count() == 0).unwrap_or(true);
        if !is_empty {
            return Err(format!(
                "Configuration directory '{}' not empty: remove old configuration files first",
                &path
            ));
        }
    } else if let Err(_err) = fs::create_dir(path) {
        return Err(format!("Failed with creating directory: {}", &path));
    }

    // now we are sure that there exists a directory which is empty (this is
    // still not completely safe, might panic)
    std::fs::write(Path::new(&path).join("orchestrator.toml"), toml::to_string(&orc_conf).expect("Wrong")).ok();
    std::fs::write(Path::new(&path).join("controller.toml"), toml::to_string(&con_conf).expect("Wrong")).ok();
    std::fs::write(Path::new(&path).join("balancer.toml"), toml::to_string(&bal_conf).expect("Wrong")).ok();
    let mut node_files = vec![];
    for (count, node_conf) in node_confs.into_iter().enumerate() {
        std::fs::write(
            Path::new(&path).join(format!("node{}.toml", count)),
            toml::to_string(&node_conf).expect("Wrong"),
        )
        .ok();
        node_files.push(format!("{}/node{}.toml", &path, count));
    }

    Ok(InABoxConfig {
        node_conf_files: node_files,
        orc_conf_file: format!("{}/orchestrator.toml", &path),
        bal_conf_file: format!("{}/balancer.toml", &path),
        con_conf_file: format!("{}/controller.toml", &path),
    })
}
