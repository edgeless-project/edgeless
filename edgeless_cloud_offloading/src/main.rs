use log;
use std::time::Duration;
use tokio::time::sleep;

use cloud_offloading::create_cloud_node;
use cloud_offloading::delete_cloud_node;
use cloud_offloading::CloudNodeInputData;
use cloud_offloading::CloudNodeData;
use edgeless_orc::proxy::Proxy;

#[tokio::main]
// This is used for testing purposes only
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Cloud data
    // TODO: Load this data from a configuration file
    let cloud_input_data = CloudNodeInputData {
        aws_region: "eu-west-1".to_string(),
        aws_ami_id: "ami-035085b5449b0383a".to_string(),
        aws_instance_type: "t2.medium".to_string(),
        aws_security_group_id: "sg-09dcfc636643d2868".to_string(),
        orchestrator_url: "3.253.97.217".to_string(),
    };

    // CNR Rebalancer
    let redis_url = "redis://localhost:6379";
    //let mut rebalancer = cloud_offloading::rebalancer::Rebalancer::new(&redis_url)?;

    // Vec with all the cloud nodes
    let mut cloud_nodes: Vec<CloudNodeData> = Vec::new();

    // Create a new EDGELESS Node in the cloud
    //let cloud_node = create_cloud_node(cloud_input_data).await?;

    // Pause to check the status
    //log::info!("Pausing for 600 seconds...");
    //sleep(Duration::from_secs(600)).await;

    // Delete the cloud node
    //delete_cloud_node(cloud_node).await?;

    loop {

        // TODO: Check if it's necessary to create a new cloud node
        // Only create a new cloud node if all existing cloud nodes are active or there are no nodes
        if cloud_nodes.iter().all(|node| node.active) {
            let cloud_node = create_cloud_node(cloud_input_data.clone()).await?;
            cloud_nodes.push(cloud_node);
        }

        // Check the status of each cloud node that is not active and check if it's registered in the orchestrator
        for cloud_node in cloud_nodes.iter_mut() {
            if !cloud_node.active {
                log::info!("Checking status of cloud node: {}", cloud_node.node_id);

                // TODO: check in the orchestrator
                
                // TODO: If the node is registered in the orchestrator, set it to active
                // cloud_node.active = true;

                // TODO: If the node is registered in the orchestrator, launch the rebalancer
            }
        }

        // TODO: Check if it's necessary to remove cloud node
        // TODO: Remove cloud nodes that are not active and not registered in the orchestrator
        // TODO: If we're going to remove a cloud node that is active, move it's functions first to other nodes

        log::info!("Cloud nodes:");
        log::info!("{:#?}", cloud_nodes);
        log::info!("------------------------------------");

        sleep(Duration::from_secs(5)).await;

        
    }

    Ok(())
}
