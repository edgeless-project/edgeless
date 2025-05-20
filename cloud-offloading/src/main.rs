use base64::{engine::general_purpose::STANDARD, Engine as _};
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

    // TODO: The orchestrator url should be an argument
    let orchestrator_url = "3.253.97.217";

    // Get the user data script from the file and convert it to to Base64
    const SCRIPT_CONTENT: &str = include_str!("ec2-user-data.sh");
    let script_content_modified = SCRIPT_CONTENT.replace("__ORCHESTRATOR_URL_PLACEHOLDER__", orchestrator_url);
    let script_content_as_string = script_content_modified.to_string();
    let encoded_user_data = STANDARD.encode(script_content_as_string);

    // Create an EC2 instance
    let run_instances = client
        .run_instances()
        .image_id("ami-035085b5449b0383a") // TODO: AMI ID should be defined in a config.toml file
        .instance_type(aws_sdk_ec2::types::InstanceType::T2Micro) // TODO: instance type should be an input
        .security_group_ids("sg-09dcfc636643d2868") // TODO: Security Group ID should be defined in a config.toml file
        .user_data(encoded_user_data)
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
    println!("Pausing for 600 seconds...");
    sleep(Duration::from_secs(600)).await;

    // Terminate the EC2 instance
    client.terminate_instances().instance_ids(instance_id).send().await?;

    println!("Instance: {instance_id}, with name: {instance_name} has been terminated.");

    Ok(())
}
