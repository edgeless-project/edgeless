mod config;

use anyhow::anyhow;
use clap::Parser;
use cloud_offloading::rebalancer::Rebalancer;
use cloud_offloading::{CloudNodeData, CloudNodeInputData, create_cloud_node, delete_cloud_node};
use config::Config;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

const DEFAULT_CONFIG_FILENAME: &str = "cloud_offloading.toml";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Generate templates instead of running the services.
    #[arg(long, short)]
    templates: bool,
    /// Directory in which to save the configuration files.
    #[arg(long, default_value_t = String::from("./"))]
    config_path: String,
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
        generate_config(&args.config_path)?;
        return Ok(());
    }

    let config = load_config(&args.config_path)?;

    let async_runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;

    async_runtime.block_on(async {
        if let Err(e) = run_cloud_offloading_delegated_orc(config).await {
            log::error!("Application runtime error: {}", e);
        }
    });

    Ok(())
}

async fn run_cloud_offloading_delegated_orc(config: Config) -> anyhow::Result<()> {
    let cloud_input_data = CloudNodeInputData {
        aws_region: config.cloud_provider.aws.region,
        aws_ami_id: config.cloud_provider.aws.ami_id,
        aws_instance_type: config.cloud_provider.aws.instance_type,
        aws_security_group_id: config.cloud_provider.aws.security_group_id,
        orchestrator_url: config.cluster.orchestrator_url,
    };

    let mut rebalancer = Rebalancer::new(&config.cluster.redis_url)?;

    // --- Cloud offloading state ---
    let mut cloud_nodes: Vec<CloudNodeData> = Vec::new();
    let mut node_being_emptied: Option<(String, std::time::Instant)> = None;

    log::info!("Cloud Offloading Component Started");

    // --- Main loop for the delegated orchestrator ---
    loop {
        log::info!("------------------------------------");
        log::info!("Running delegated orchestrator check cycle...");

        // 1. UPDATE STATE FROM REDIS DATA
        let active_orc_nodes = rebalancer.update_state();

        // Update the 'active' state of the cloud nodes
        // This is needed to ensure that the cloud nodes are marked as active because it takes almost 1 minute to configure a new node
        for node in cloud_nodes.iter_mut() {
            if !node.active && active_orc_nodes.contains(&node.node_id) {
                node.active = true;
                log::info!("Cloud node {} is now active in the orchestrator!", node.node_id);
                // After a node becomes active, we force a rebalance to ensure it receives load
                rebalancer.rebalance_cluster();
            }
        }

        // CLEANUP: Find and delete any cloud nodes that failed to be active if 5 minutes have passed since creation
        const NODE_ACTIVATION_TIMEOUT_SECS: u64 = 300;
        let mut broken_nodes_to_delete = Vec::new();

        for node in &cloud_nodes {
            if !node.active && node.creation_time.elapsed().as_secs() > NODE_ACTIVATION_TIMEOUT_SECS {
                log::warn!(
                    "Node {} ({}) failed to become active within {} seconds. Marking for deletion.",
                    node.node_id,
                    node.instance_id,
                    NODE_ACTIVATION_TIMEOUT_SECS
                );
                broken_nodes_to_delete.push(node.instance_id.clone());
            }
        }

        if !broken_nodes_to_delete.is_empty() {
            // Delete broken nodes from AWS
            cloud_nodes.retain(|node| {
                if broken_nodes_to_delete.contains(&node.instance_id) {
                    let node_to_delete = node.clone();
                    tokio::spawn(async move {
                        log::info!("Deleting broken node from AWS: {}", node_to_delete.node_id);
                        if let Err(e) = delete_cloud_node(node_to_delete).await {
                            log::error!("Failed to delete broken node {}: {}", e, e);
                        }
                    });
                    false
                } else {
                    true
                }
            });

            // After cleanup, it's better to restart the cycle to have a clean state.
            log::info!("Cleanup finished. Restarting check cycle...");
            sleep(Duration::from_secs(config.general.check_interval_seconds)).await;
            continue;
        }

        // 2. DECIDE WHETHER TO CREATE A NEW NODE
        // Only create a new node if there isn't one already being created.
        // If the total number of active nodes is less than the minimum required, we also create a new node.
        let is_creating_node = cloud_nodes.iter().any(|n| !n.active);
        if !is_creating_node
            && (rebalancer.should_create_node(
                config.scaling.thresholds.credit_overload,
                config.scaling.thresholds.cpu_high_percent,
                config.scaling.thresholds.mem_high_percent,
            ) || active_orc_nodes.len() < config.cluster.minimum_nodes)
        {
            log::warn!("Cluster is overloaded or has fewer nodes than expected. Creating a new cloud node...");
            match create_cloud_node(cloud_input_data.clone()).await {
                Ok(new_node) => {
                    log::info!("Successfully initiated creation of new node: {}", new_node.node_id);
                    cloud_nodes.push(new_node);
                }
                Err(e) => log::error!("Failed to create cloud node. Full error: {:?}", e),
            }
        }

        // 3. DECIDE WHETHER TO DELETE A NODE
        if let Some((emptying_id, start_time)) = &node_being_emptied {
            // If there's a node being emptied, check if it's empty and if the cooldown period has passed
            if rebalancer.is_node_empty(emptying_id) {
                if start_time.elapsed().as_secs() >= config.scaling.thresholds.delete_cooldown_seconds {
                    log::info!("Node {} is empty and cooldown period is over. Deleting it.", emptying_id);
                    let node_id_to_remove = emptying_id.clone();
                    if let Some(pos) = cloud_nodes.iter().position(|n| n.node_id == node_id_to_remove) {
                        let node_to_delete = cloud_nodes.remove(pos);
                        if let Err(e) = delete_cloud_node(node_to_delete).await {
                            log::error!("Failed to delete cloud node {}: {}", node_id_to_remove, e);
                        }
                    }
                    node_being_emptied = None;
                } else {
                    log::info!("Node {} is empty, waiting for cooldown period to finish...", emptying_id);
                }
            } else {
                // Empty and cordon the node
                rebalancer.cordon_node(emptying_id);
                rebalancer.empty_node(emptying_id);
            }
        } else if !is_creating_node {
            // We only delete a node if there are more nodes available
            if active_orc_nodes.len() > 1 {
                // If there's no node being created or emptied, check if we can find an underutilized node to delete
                let managed_cloud_node_ids: HashSet<String> = cloud_nodes.iter().map(|n| n.node_id.clone()).collect();
                if let Some(victim_id) = rebalancer.find_node_to_delete(
                    &managed_cloud_node_ids,
                    config.scaling.thresholds.cpu_low_percent,
                    config.scaling.thresholds.mem_low_percent,
                ) {
                    log::warn!("Found underutilized node {}. Attempting to empty and cordon it for deletion.", victim_id);
                    rebalancer.cordon_node(&victim_id);
                    rebalancer.empty_node(&victim_id);
                    node_being_emptied = Some((victim_id, std::time::Instant::now()));
                }
            }
        }

        // Log the current state of the cloud nodes
        log::info!(
            "Managed cloud nodes: {} active, {} total.",
            cloud_nodes.iter().filter(|n| n.active).count(),
            cloud_nodes.len()
        );
        log::debug!("{:#?}", cloud_nodes);

        sleep(Duration::from_secs(config.general.check_interval_seconds)).await;
    }
}

fn generate_config(config_path: &str) -> anyhow::Result<()> {
    let path = PathBuf::from(config_path).join(DEFAULT_CONFIG_FILENAME);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let default_config = Config::default();
    let toml_string = toml::to_string_pretty(&default_config)?;
    fs::write(&path, toml_string)?;

    log::info!("Default configuration template written to {:?}", path);
    Ok(())
}

fn load_config(config_path: &str) -> anyhow::Result<Config> {
    let path = PathBuf::from(config_path).join(DEFAULT_CONFIG_FILENAME);
    let config_str = fs::read_to_string(&path).map_err(|e| anyhow!("Failed to read configuration file at {:?}: {}", path, e))?;
    toml::from_str(&config_str).map_err(|e| anyhow!("Failed to parse configuration file at {:?}: {}", path, e))
}
