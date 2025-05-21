use std::time::Duration;
use tokio::time::sleep;
use log;

use cloud_offloading::CloudNodeInputData;
use cloud_offloading::create_cloud_node;
use cloud_offloading::delete_cloud_node;

#[tokio::main]
// This is used for testing purposes only
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let cloud_input_data = CloudNodeInputData {
        aws_region: "eu-west-1".to_string(),
        aws_ami_id: "ami-035085b5449b0383a".to_string(),
        aws_instance_type: "t2.medium".to_string(),
        aws_security_group_id: "sg-09dcfc636643d2868".to_string(),
        orchestrator_url: "3.253.97.217".to_string()
    };

    // Create a new EDGELESS Node in the cloud
    let cloud_node = create_cloud_node(cloud_input_data).await?;

    // Pause to check the status
    log::info!("Pausing for 600 seconds...");
    sleep(Duration::from_secs(600)).await;

    // Delete the cloud node
    delete_cloud_node(cloud_node).await?;

    Ok(())
}