// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
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
    #[arg(short, long, default_value_t = String::from("Redis"))]
    proxy_type: String,
    #[arg(short, long, default_value_t = String::from("redis://localhost:6379"))]
    redis_url: String,
}

#[derive(Debug, clap::Subcommand)]
enum DumpCommands {
    Performance {},
}

#[derive(Debug, clap::Subcommand)]
enum NodeCommands {
    Capabilities {},
    Health {},
    Instances {},
}

#[derive(Debug, clap::Subcommand)]
enum ShowCommands {
    Functions {},
    Resources {},
    Node {
        #[command(subcommand)]
        node_command: NodeCommands,
    },
}

#[derive(Debug, clap::Subcommand)]
enum IntentCommands {
    Migrate { instance: String, node: String },
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

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();

    anyhow::ensure!(args.proxy_type.to_lowercase() == "redis", "unknown proxy type: {}", args.proxy_type);

    let mut proxy = match edgeless_orc::proxy_redis::ProxyRedis::new(&args.redis_url, false) {
        Ok(proxy) => proxy,
        Err(err) => anyhow::bail!("could not connect to a Redis at {}: {}", args.redis_url, err),
    };

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
                    println!("{} -> {}", resource, node);
                }
            }
            ShowCommands::Node { node_command } => match node_command {
                NodeCommands::Capabilities {} => {
                    for (node, capabilities) in proxy.fetch_node_capabilities().iter().sorted_by_key(|x| x.0.to_string()) {
                        println!("{} -> {}", node, capabilities);
                    }
                }
                NodeCommands::Health {} => {
                    for (node, health) in proxy.fetch_node_health().iter().sorted_by_key(|x| x.0.to_string()) {
                        println!("{} -> {}", node, health);
                    }
                }
                NodeCommands::Instances {} => {
                    for (node, instances) in proxy.fetch_nodes_to_instances().iter().sorted_by_key(|x| x.0.to_string()) {
                        println!("{}", node);
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
            IntentCommands::Migrate { instance, node } => {
                let instance_id = match uuid::Uuid::from_str(&instance) {
                    Ok(instance_id) => instance_id,
                    Err(err) => anyhow::bail!("invalid instance id {}: {}", instance, err),
                };
                let node_id = match uuid::Uuid::from_str(&node) {
                    Ok(node_id) => node_id,
                    Err(err) => anyhow::bail!("invalid instance id {}: {}", node, err),
                };
                proxy.add_deploy_intents(vec![edgeless_orc::orchestrator::DeployIntent::Migrate(instance_id, vec![node_id])]);
            }
        },
        Commands::Dump { dump_command } => match dump_command {
            DumpCommands::Performance {} => {
                for (metric, inner_map) in proxy.fetch_performance_samples() {
                    for (id, values) in inner_map {
                        let filename = format!("{}-{}.dat", metric, id);
                        println!("saving to {}", filename);
                        let mut outfile = std::fs::OpenOptions::new()
                            .write(true)
                            .append(false)
                            .create(true)
                            .truncate(true)
                            .open(filename.clone())?;
                        for value in values {
                            outfile
                                .write_all(format!("{},{}\n", value.0, value.1).as_bytes())
                                .unwrap_or_else(|_| panic!("could not write to file '{}'", filename));
                        }
                    }
                }
            }
        },
    }

    Ok(())
}
