use aws_sdk_ec2::types::Tag;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::time::Duration;
use tokio::time::sleep;

fn generate_instance_name() -> String {
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();
    format!("EDGELESS-Node-{}", random_string)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Get Region from config.toml file

    let region_provider = "eu-west-1";
    let config = aws_config::from_env().region(region_provider).load().await;

    let client = aws_sdk_ec2::Client::new(&config);

    // TODO: Create a Security Group for the instance

    // Create an EC2 instance
    let run_instances = client
        .run_instances()
        .image_id("ami-05edf2d87fdbd91c1") // TODO: Replace with a custom image for EDGELESS Node
        .instance_type(aws_sdk_ec2::types::InstanceType::T2Micro) // TODO instance type should be an input
        .min_count(1)
        .max_count(1)
        .send()
        .await?;

    if run_instances.instances().is_empty() {
        println!("Failed to create instance");
    }

    let instance_id = run_instances.instances()[0].instance_id().unwrap();

    // Define a name for the instance
    let instance_name = generate_instance_name();
    let response = client
        .create_tags()
        .resources(instance_id)
        .tags(Tag::builder().key("Name").value(&instance_name).build())
        .send()
        .await?;

    println!("Instance: {instance_id}, with name: {instance_name} has been created.");

    // Pause to check the status
    println!("Pausing for 120 seconds...");
    sleep(Duration::from_secs(120)).await;

    // Terminate the EC2 instance
    client.terminate_instances().instance_ids(instance_id).send().await?;

    println!("Instance: {instance_id}, with name: {instance_name} has been terminated.");

    Ok(())
}
