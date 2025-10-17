use aws_config::Region;
use aws_sdk_ec2::types::{InstanceType, Tag};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::time::Instant;
use uuid::Uuid;

pub mod rebalancer;
#[derive(Debug, Clone)]
pub struct CloudNodeInputData {
    pub aws_region: String,
    pub aws_ami_id: String,
    pub aws_instance_type: String,
    pub aws_security_group_id: String,
    pub orchestrator_url: String,
}

#[derive(Debug, Clone)]
pub struct CloudNodeData {
    pub aws_region: String,
    pub instance_id: String,
    instance_name: String,
    pub node_id: String,
    pub active: bool,
    pub creation_time: Instant,
}

fn generate_instance_name() -> String {
    let random_string: String = thread_rng().sample_iter(&Alphanumeric).take(8).map(char::from).collect();
    format!("EDGELESS-Node-{random_string}")
}

pub async fn create_cloud_node(input_data: CloudNodeInputData) -> Result<CloudNodeData, Box<dyn std::error::Error>> {
    // Config for AWS SDK
    let config = aws_config::from_env().region(Region::new(input_data.aws_region.clone())).load().await;
    let client = aws_sdk_ec2::Client::new(&config);

    // Define a node id
    let node_id = Uuid::new_v4().to_string();

    // Get the user data script from the file and convert it to to Base64
    const SCRIPT_CONTENT: &str = include_str!("ec2-user-data.sh");
    let script_content_modified = SCRIPT_CONTENT
        .replace("__ORCHESTRATOR_URL_PLACEHOLDER__", &input_data.orchestrator_url)
        .replace("__NODE_ID_PLACEHOLDER__", &node_id);
    let script_content_as_string = script_content_modified.to_string();
    let encoded_user_data = STANDARD.encode(script_content_as_string);

    // Create an EC2 instance
    let run_instances = client
        .run_instances()
        .image_id(input_data.aws_ami_id)
        .instance_type(InstanceType::from(input_data.aws_instance_type.as_str()))
        .security_group_ids(input_data.aws_security_group_id)
        .user_data(encoded_user_data)
        .min_count(1)
        .max_count(1)
        .send()
        .await?;
    if run_instances.instances().is_empty() {
        log::error!("Failed to create instance");
        return Err(("Failed to create instance").into());
    }

    let instance_id = run_instances.instances()[0].instance_id().unwrap();

    // Define a name for the instance
    let instance_name = generate_instance_name();
    let _response = client
        .create_tags()
        .resources(instance_id)
        .tags(Tag::builder().key("Name").value(&instance_name).build())
        .send()
        .await?;

    log::info!("EDGELESS Node deployed on AWS Instance: {instance_id}, instance name: {instance_name}, with node_id: {node_id} has been created.");

    // Build the CloudNode struct
    let cloud_node = CloudNodeData {
        aws_region: input_data.aws_region,
        instance_id: instance_id.to_string(),
        instance_name,
        node_id,
        active: false,
        creation_time: Instant::now(),
    };

    Ok(cloud_node)
}

pub async fn delete_cloud_node(cloud_node: CloudNodeData) -> Result<String, Box<dyn std::error::Error>> {
    let instance_id: &str = &cloud_node.instance_id;
    let instance_name: &str = &cloud_node.instance_name;

    // Config for AWS SDK
    let config = aws_config::from_env().region(Region::new(cloud_node.aws_region)).load().await;
    let client = aws_sdk_ec2::Client::new(&config);

    // Terminate the EC2 instance
    client.terminate_instances().instance_ids(instance_id).send().await?;

    log::info!("EDGELESS Node deployed on AWS Instance: {instance_id}, with name: {instance_name} has been deleted.");

    Ok(instance_id.to_string())
}
