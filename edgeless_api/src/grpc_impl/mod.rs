pub mod function_instance;

pub mod agent;

pub mod orc;

pub mod controller;

pub mod workflow_instance;

pub mod invocation;

pub mod resource_configuration;

pub mod api {
    tonic::include_proto!("edgeless_api");
}
