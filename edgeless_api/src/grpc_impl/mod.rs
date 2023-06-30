pub mod function_instance;

pub mod agent;

pub mod orc;

pub mod con;

pub mod workflow_instance;

pub mod api {
    tonic::include_proto!("agent_api");
}
