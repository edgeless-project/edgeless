use std::{collections::HashMap, fmt, path::Path};

use anyhow::anyhow;
use clap::Parser;
use edgeless_con::{EdgelessConOrcConfig, EdgelessConResourceConfig};
use edgeless_dataplane::core::EdgelessDataplanePeerSettings;
use edgeless_inabox::InABoxConfig;
use edgeless_node::EdgelessNodeSettings;
use edgeless_orc::EdgelessOrcNodeConfig;
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
    let config: InABoxConfig;

    if args.templates || args.num_of_nodes == 1 {
        log::info!("Generating default templates for one node");
        edgeless_api::util::create_template("node.toml", edgeless_node::edgeless_node_default_conf().as_str())?;
        edgeless_api::util::create_template("orchestrator.toml", edgeless_orc::edgeless_orc_default_conf().as_str())?;
        edgeless_api::util::create_template("balancer.toml", edgeless_bal::edgeless_bal_default_conf().as_str())?;
        edgeless_api::util::create_template("controller.toml", edgeless_con::edgeless_con_default_conf().as_str())?;
        config = InABoxConfig {
            node_conf_files: vec!["node.toml".to_string()],
            orc_conf_file: "orchestrator.toml".to_string(),
            bal_conf_file: "balancer.toml".to_string(),
            con_conf_file: "controller.toml".to_string(),
        };
        return Ok(());
    } else if args.num_of_nodes > 0 {
        config = match generate_configs(args.num_of_nodes) {
            Ok(config) => config,
            Err(error) => return Err(anyhow!(error)),
        }
    } else {
        return Err(anyhow!("Invalid number of worker nodes specified"));
    }

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

    // At first generate endpoints for invocation_urls and orc_agent_urls
    let mut node_invocation_urls: HashMap<Uuid, String> = HashMap::new();
    let mut node_orc_agent_urls: HashMap<Uuid, String> = HashMap::new();

    for _ in (0..number_of_nodes) {
        let node_id = Uuid::new_v4();
        node_invocation_urls.insert(node_id, next_url());
        node_orc_agent_urls.insert(node_id, next_url());
    }

    // Balancer
    let bal_conf = edgeless_bal::EdgelessBalSettings {
        balancer_id: Uuid::new_v4(),
        invocation_url: next_url(),
        resource_configuration_url: next_url(),
        http_ingress_url: next_url(),
        // filled with all values from the node_invocation_urls
        nodes: node_invocation_urls
            .iter()
            .map(|(key, value)| EdgelessDataplanePeerSettings {
                id: key.clone(),
                invocation_url: value.clone(),
            })
            .collect(),
    };

    // Nodes
    // node peers: its own invocation_url, inv_url of balancer, invocation_urls
    // of other nodes
    let mut node_confs: Vec<EdgelessNodeSettings> = vec![];
    let node_peers: Vec<EdgelessDataplanePeerSettings> = node_invocation_urls
        .iter()
        .map(|(key, value)| EdgelessDataplanePeerSettings {
            id: key.clone(),
            invocation_url: value.clone(),
        })
        .collect();
    for node_id in node_invocation_urls.keys() {
        let mut peers: Vec<EdgelessDataplanePeerSettings> = vec![
            // peer for the balancer
            EdgelessDataplanePeerSettings {
                id: bal_conf.balancer_id.clone(),
                invocation_url: bal_conf.invocation_url.clone(),
            },
        ];
        peers.extend(node_peers.clone());
        node_confs.push(EdgelessNodeSettings {
            node_id: node_id.clone(),
            agent_url: node_orc_agent_urls.get(node_id).expect("").clone(), // we are sure that it is there
            invocation_url: node_invocation_urls.get(node_id).expect("").clone(), // we are sure that it is there
            metrics_url: next_url(),
            peers,
        })
    }

    // Orchestrator
    let orc_conf = edgeless_orc::EdgelessOrcSettings {
        domain_id: "domain-1".to_string(),
        orchestrator_url: next_url(),
        // filled with all values from the node_orc_agent_urls
        nodes: node_orc_agent_urls
            .iter()
            .map(|(key, value)| EdgelessOrcNodeConfig {
                node_id: key.clone(),
                agent_url: value.clone(),
            })
            .collect(),
    };

    // Controller
    let con_conf = edgeless_con::EdgelessConSettings {
        controller_url: next_url(),
        // for now only one orchestrator
        orchestrators: vec![EdgelessConOrcConfig {
            domain_id: orc_conf.domain_id.clone(),
            orchestrator_url: orc_conf.orchestrator_url.clone(),
        }],
        // resources are hardcoded mostly
        resources: vec![
            EdgelessConResourceConfig {
                resource_provider_id: "file-log-1".to_string(),
                resource_class_type: "file-log".to_string(),
                output_callback_declarations: vec![],
                resource_configuration_url: bal_conf.resource_configuration_url.clone(),
            },
            EdgelessConResourceConfig {
                resource_provider_id: "http-ingress-1".to_string(),
                resource_class_type: "http-ingress".to_string(),
                output_callback_declarations: vec!["new_request".to_string()],
                resource_configuration_url: bal_conf.resource_configuration_url.clone(),
            },
            EdgelessConResourceConfig {
                resource_provider_id: "http-egress-1".to_string(),
                resource_class_type: "http-egress".to_string(),
                output_callback_declarations: vec![],
                resource_configuration_url: bal_conf.resource_configuration_url.clone(),
            },
            EdgelessConResourceConfig {
                resource_provider_id: "redis-1".to_string(),
                resource_class_type: "redis".to_string(),
                output_callback_declarations: vec![],
                resource_configuration_url: bal_conf.resource_configuration_url.clone(),
            },
        ],
    };

    // Save the config files to the configs/ directory if its empty, to give
    // users reference on how the cluster is configured
    let path = "config/";
    if fs::metadata(&path).is_ok() {
        let is_empty = fs::read_dir(&path).map(|entries| entries.count() == 0).unwrap_or(true);
        if !is_empty {
            return Err("/config directory is not empty - remove old configuration files first".to_string());
        }
    } else if let Err(err) = fs::create_dir(&path) {
        return Err("Failed with creating a directory config/".to_string());
    }

    // now we are sure that there exists a directory which is empty (this is
    // still not completely safe, might panic)
    std::fs::write(Path::new(&path).join("orchestrator.toml"), toml::to_string(&orc_conf).expect("Wrong")).ok();
    std::fs::write(Path::new(&path).join("controller.toml"), toml::to_string(&con_conf).expect("Wrong")).ok();
    std::fs::write(Path::new(&path).join("balancer.toml"), toml::to_string(&bal_conf).expect("Wrong")).ok();
    let mut count = 0;
    let mut node_files = vec![];
    for node_conf in node_confs {
        std::fs::write(
            Path::new(&path).join(format!("node{}.toml", count)),
            toml::to_string(&node_conf).expect("Wrong"),
        )
        .ok();
        node_files.push(format!("config/node{}.toml", count));
        count += 1;
    }

    Ok(InABoxConfig {
        node_conf_files: node_files,
        orc_conf_file: "config/orchestrator.toml".to_string(),
        bal_conf_file: "config/balancer.toml".to_string(),
        con_conf_file: "config/controller.toml".to_string(),
    })
}
