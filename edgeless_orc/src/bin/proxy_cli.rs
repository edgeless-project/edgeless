// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use itertools::Itertools;
use std::{io::Write, str::FromStr};

use clap::Parser;
use edgeless_orc::proxy::Proxy;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
    /// Orchestrator proxy type. One of: Redis.
    #[arg(short, long, default_value_t = String::from("Redis"))]
    proxy_type: String,
    /// URL of the Redis server used as orchestrator's proxy.
    #[arg(short, long, default_value_t = String::from("redis://localhost:6379"))]
    redis_url: String,
    /// How to print the node identifiers. One of: uuid, labels
    #[arg(long, default_value_t = String::from("hostname"))]
    node_print_format: String,
    /// Print the version number and quit.
    #[arg(long, default_value_t = false)]
    version: bool,
}

enum NodePrintFormat {
    Uuid,
    Labels,
    Hostname,
}

impl NodePrintFormat {
    fn from(str: &str) -> anyhow::Result<Self> {
        if str == "uuid" {
            Ok(Self::Uuid)
        } else if str == "labels" {
            Ok(Self::Labels)
        } else if str == "hostname" {
            Ok(Self::Hostname)
        } else {
            anyhow::bail!("invalid node-print-format value: {}", str)
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum DumpCommands {
    Performance {},
    Instances {},
}

#[derive(Debug, clap::Subcommand)]
enum NodeCommands {
    Capabilities {},
    ResourceProviders {},
    Health {},
    Instances {},
}

#[derive(Debug, clap::Subcommand)]
enum ShowCommands {
    Functions {},
    Resources {},
    LogicalToPhysical {},
    LogicalToWorkflow {},
    Node {
        #[command(subcommand)]
        node_command: NodeCommands,
    },
}

#[derive(Debug, clap::Subcommand)]
enum IntentCommands {
    Migrate { instance: String, node_id: String },
    Cordon { node_id: String },
    Uncordon { node_id: String },
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    Show {
        #[command(subcommand)]
        show_command: ShowCommands,
    },
    Intent {
        #[command(subcommand)]
        intent_command: IntentCommands,
    },
    Dump {
        #[command(subcommand)]
        dump_command: DumpCommands,
    },
}

fn open_file(filename: &str) -> anyhow::Result<std::fs::File> {
    println!("saving to {}", filename);
    Ok(std::fs::OpenOptions::new()
        .write(true)
        .append(false)
        .create(true)
        .truncate(true)
        .open(filename)?)
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
    anyhow::ensure!(args.proxy_type.to_lowercase() == "redis", "unknown proxy type: {}", args.proxy_type);

    let mut proxy = match edgeless_orc::proxy_redis::ProxyRedis::new_client(&args.redis_url) {
        Ok(proxy) => proxy,
        Err(err) => anyhow::bail!("could not connect to a Redis at {}: {}", args.redis_url, err),
    };

    let node_print_format = NodePrintFormat::from(&args.node_print_format)?;
    let mut node_to_names = std::collections::HashMap::new();
    match node_print_format {
        NodePrintFormat::Labels => {
            for (node_id, caps) in proxy.fetch_node_capabilities() {
                let labels = caps.labels.join(",");
                if !labels.is_empty() {
                    node_to_names.insert(node_id, labels);
                }
            }
        }
        NodePrintFormat::Hostname => {
            for (node_id, caps) in proxy.fetch_node_capabilities() {
                if let Some(hostname) = caps.labels.iter().find(|x| x.starts_with("hostname=")) {
                    node_to_names.insert(node_id, hostname.strip_prefix("hostname=").unwrap().to_string());
                }
            }
        }
        NodePrintFormat::Uuid => {}
    }
    let map_node = |node_id: &uuid::Uuid| node_to_names.get(node_id).unwrap_or(&node_id.to_string()).to_string();

    match args.command {
        Commands::Show { show_command } => match show_command {
            ShowCommands::Functions {} => {
                for (function, nodes) in proxy.fetch_function_instances_to_nodes().iter().sorted_by_key(|x| x.0.to_string()) {
                    println!(
                        "{} -> {}",
                        function,
                        nodes.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")
                    );
                }
            }
            ShowCommands::Resources {} => {
                for (resource, node) in proxy.fetch_resource_instances_to_nodes().iter().sorted_by_key(|x| x.0.to_string()) {
                    println!("{} -> {}", resource, map_node(node));
                }
            }
            ShowCommands::LogicalToPhysical {} => {
                for (logical, physical) in proxy.fetch_instances_to_physical_ids().iter().sorted_by_key(|x| x.0.to_string()) {
                    println!(
                        "{} -> {}",
                        logical,
                        physical.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",")
                    );
                }
            }
            ShowCommands::LogicalToWorkflow {} => {
                for (logical, workflow_id) in proxy.fetch_logical_id_to_workflow_id().iter().sorted_by_key(|x| x.0.to_string()) {
                    println!("{} -> {}", logical, workflow_id);
                }
            }
            ShowCommands::Node { node_command } => match node_command {
                NodeCommands::Capabilities {} => {
                    for (node, capabilities) in proxy.fetch_node_capabilities().iter().sorted_by_key(|x| x.0.to_string()) {
                        println!("{} -> {}", map_node(node), capabilities);
                    }
                }
                NodeCommands::ResourceProviders {} => {
                    for (provider_id, resource_providers) in proxy.fetch_resource_providers().iter().sorted_by_key(|x| x.0.to_string()) {
                        println!("{} -> {}", provider_id, resource_providers);
                    }
                }
                NodeCommands::Health {} => {
                    for (node, health) in proxy.fetch_node_health().iter().sorted_by_key(|x| x.0.to_string()) {
                        println!("{} -> {}", map_node(node), health);
                    }
                }
                NodeCommands::Instances {} => {
                    for (node, instances) in proxy.fetch_nodes_to_instances().iter().sorted_by_key(|x| x.0.to_string()) {
                        println!("{}", map_node(node));
                        for instance in instances {
                            match instance {
                                edgeless_orc::proxy::Instance::Function(id) => println!("[F] {}", id),
                                edgeless_orc::proxy::Instance::Resource(id) => println!("[R] {}", id),
                            }
                        }
                    }
                }
            },
        },
        Commands::Intent { intent_command } => match intent_command {
            IntentCommands::Migrate { instance, node_id } => {
                let instance_id = match uuid::Uuid::from_str(&instance) {
                    Ok(instance_id) => instance_id,
                    Err(err) => anyhow::bail!("invalid instance id {}: {}", instance, err),
                };
                let node_id = match uuid::Uuid::from_str(&node_id) {
                    Ok(node_id) => node_id,
                    Err(err) => anyhow::bail!("invalid node id {}: {}", node_id, err),
                };
                proxy.add_deploy_intents(vec![edgeless_orc::deploy_intent::DeployIntent::Migrate(instance_id, vec![node_id])]);
            }
            IntentCommands::Cordon { node_id } => {
                let node_id = match uuid::Uuid::from_str(&node_id) {
                    Ok(node_id) => node_id,
                    Err(err) => anyhow::bail!("invalid node id {}: {}", node_id, err),
                };
                proxy.add_deploy_intents(vec![edgeless_orc::deploy_intent::DeployIntent::Cordon(node_id)]);
            }
            IntentCommands::Uncordon { node_id } => {
                let node_id = match uuid::Uuid::from_str(&node_id) {
                    Ok(node_id) => node_id,
                    Err(err) => anyhow::bail!("invalid node id {}: {}", node_id, err),
                };
                proxy.add_deploy_intents(vec![edgeless_orc::deploy_intent::DeployIntent::Uncordon(node_id)]);
            }
        },
        Commands::Dump { dump_command } => match dump_command {
            DumpCommands::Performance {} => {
                for (metric, inner_map) in proxy.fetch_performance_samples() {
                    for (id, values) in inner_map {
                        let filename = format!("{}-{}.dat", metric, id);
                        let mut outfile = open_file(&filename)?;
                        for value in values {
                            outfile
                                .write_all(format!("{},{}\n", value.0, value.1).as_bytes())
                                .unwrap_or_else(|_| panic!("could not write to file '{}'", filename));
                        }
                    }
                }
            }
            DumpCommands::Instances {} => {
                for (lid, spec) in proxy.fetch_function_instance_requests() {
                    let filename = format!("fun-{}.json", lid);
                    let outfile = open_file(&filename)?;
                    serde_json::to_writer_pretty(outfile, &spec).unwrap_or_else(|_| panic!("could not write to file '{}'", filename));
                }
                for (lid, spec) in proxy.fetch_resource_instance_configurations() {
                    let filename = format!("res-{}.json", lid);
                    let outfile = open_file(&filename)?;
                    serde_json::to_writer_pretty(outfile, &spec).unwrap_or_else(|_| panic!("could not write to file '{}'", filename));
                }
            }
        },
    }

    Ok(())
}
