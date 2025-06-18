use cloud_offloading::rebalancer::Rebalancer;
use cloud_offloading::{create_cloud_node, delete_cloud_node, CloudNodeData, CloudNodeInputData};
use log;
use std::collections::HashSet;
use std::time::Duration;
use tokio::time::sleep;

const CHECK_INTERVAL_SECONDS: u64 = 15; // Interval to check the cluster state
const CREATE_NODE_OVERLOAD_THRESHOLD: f64 = 1.0; // If the total overload exceeds this threshold, a new node will be created
const NODE_COOLDOWN_PERIOD_SECONDS: u64 = 300; // Wait time before deleting a newly emptied node

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // --- Config ---
    // TODO: Load these values from a config file or environment variables
    let cloud_input_data = CloudNodeInputData {
        aws_region: "eu-west-1".to_string(),
        aws_ami_id: "ami-035085b5449b038a".to_string(), // Asegúrate de que esta AMI es correcta
        aws_instance_type: "t2.medium".to_string(),
        aws_security_group_id: "sg-09dcfc636643d2868".to_string(),
        orchestrator_url: "3.253.97.217".to_string(),
    };
    let redis_url = "redis://localhost:6379";
    let mut rebalancer = Rebalancer::new(redis_url)?;

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

        // 2. DECIDE WHETHER TO CREATE A NEW NODE
        // Only create a new node if there isn't one already being created.
        let is_creating_node = cloud_nodes.iter().any(|n| !n.active);
        if !is_creating_node && rebalancer.should_create_node(CREATE_NODE_OVERLOAD_THRESHOLD) {
            log::warn!("Cluster is overloaded. Creating a new cloud node...");
            match create_cloud_node(cloud_input_data.clone()).await {
                Ok(new_node) => {
                    log::info!("Successfully initiated creation of new node: {}", new_node.node_id);
                    cloud_nodes.push(new_node);
                }
                Err(e) => log::error!("Failed to create cloud node: {}", e),
            }
        }

        // 3. DECIDE WHETHER TO DELETE A NODE
        if let Some((emptying_id, start_time)) = &node_being_emptied {
            // If there's a node being emptied, check if it's empty and if the cooldown period has passed
            if rebalancer.is_node_empty(emptying_id) {
                 if start_time.elapsed().as_secs() >= NODE_COOLDOWN_PERIOD_SECONDS {
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
                // Keey trying to empty the node
                rebalancer.empty_node(emptying_id);
            }
        } else if !is_creating_node {
            // If there's no node being created or emptied, check if we can find an underutilized node to delete
            let managed_cloud_node_ids: HashSet<String> = cloud_nodes.iter().map(|n| n.node_id.clone()).collect();
            if let Some(victim_id) = rebalancer.find_node_to_delete(&managed_cloud_node_ids) {
                log::warn!("Found underutilized node {}. Attempting to empty it for deletion.", victim_id);
                rebalancer.empty_node(&victim_id);
                node_being_emptied = Some((victim_id, std::time::Instant::now()));
            }
        }

        // Log the current state of the cloud nodes
        log::info!("Managed cloud nodes: {} active, {} total.", cloud_nodes.iter().filter(|n| n.active).count(), cloud_nodes.len());
        log::debug!("{:#?}", cloud_nodes);

        sleep(Duration::from_secs(CHECK_INTERVAL_SECONDS)).await;
    }
}