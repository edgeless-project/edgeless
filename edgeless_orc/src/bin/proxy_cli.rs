// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use core::f64;
use edgeless_api::outer::controller::ControllerAPI;
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
    /// URL of the EDGELESS controller, used only with some commands.
    #[arg(short, long, default_value_t = String::from("http://127.0.0.1:7001"))]
    controller_url: String,
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
    PerformanceCsv {},
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
enum TopCommands {
    Nodes {},
    Workflow { wf_id: String },
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
    Top {
        #[command(subcommand)]
        top_command: TopCommands,
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

fn top_nodes(
    proxy: &mut edgeless_orc::proxy_redis::ProxyRedis,
    stdout: &mut std::io::Stdout,
    map_node: &dyn Fn(&uuid::Uuid) -> String,
) -> anyhow::Result<()> {
    let mut ascii_table = ascii_table::AsciiTable::default();
    ascii_table.set_max_width(termion::terminal_size()?.0 as usize);
    ascii_table.column(0).set_header("Node").set_align(ascii_table::Align::Left);
    ascii_table.column(1).set_header("Runtimes").set_align(ascii_table::Align::Left);
    ascii_table.column(2).set_header("Load").set_align(ascii_table::Align::Left);
    ascii_table.column(3).set_header("Proc CPU").set_align(ascii_table::Align::Left);
    ascii_table.column(4).set_header("Memory").set_align(ascii_table::Align::Left);
    ascii_table.column(5).set_header("GPU util").set_align(ascii_table::Align::Left);
    ascii_table.column(6).set_header("Power").set_align(ascii_table::Align::Left);

    let all_caps = proxy.fetch_node_capabilities();
    let all_health = proxy.fetch_node_health();

    let mut nodes: Vec<(String, uuid::Uuid)> = all_caps.keys().map(|x| (map_node(x), *x)).collect();
    nodes.sort_by(|(name_a, _uuid_a), (name_b, _uuid_b)| name_a.cmp(name_b));

    let mut runtimes = vec![];
    let mut memory = vec![];
    for (_node_name, node_id) in &nodes {
        runtimes.push(all_caps.get(node_id).unwrap().runtimes.join(","));
        let health = all_health.get(node_id).unwrap();
        memory.push(health.mem_used / health.mem_available);
    }

    let mut data: Vec<Vec<&dyn std::fmt::Display>> = vec![];
    for (i, (node_name, node_id)) in nodes.iter().enumerate() {
        let health = all_health.get(node_id).unwrap();

        data.push(vec![
            node_name,
            &runtimes[i],
            &health.load_avg_1,
            &health.proc_cpu_usage,
            &memory[i],
            &health.gpu_load_perc,
            &health.active_power,
        ]);
    }

    let _ = write!(stdout, "{}", ascii_table.format(data));

    Ok(())
}

async fn top_workflow(
    proxy: &mut edgeless_orc::proxy_redis::ProxyRedis,
    edgeless_cli: &mut Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>,
    wf_id: &edgeless_api::workflow_instance::WorkflowId,
    stdout: &mut std::io::Stdout,
    map_node: &dyn Fn(&uuid::Uuid) -> String,
) -> anyhow::Result<()> {
    let now = chrono::Utc::now();
    let avg_min_max = |series: &edgeless_orc::proxy::PerformanceSeries| {
        let mut tot = 0.0_f64;
        let mut num = 0_usize;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for (timestamp, value) in series {
            if (now - timestamp) > chrono::Duration::seconds(60) {
                continue;
            }
            if let Ok(mut value) = value.parse::<f64>() {
                value *= 1000.0;
                tot += value;
                num += 1;
                min = min.min(value);
                max = max.max(value);
            }
        }
        (tot / num as f64, min, max)
    };

    let mut ascii_table = ascii_table::AsciiTable::default();
    ascii_table.set_max_width(termion::terminal_size()?.0 as usize);
    ascii_table.column(0).set_header("Name").set_align(ascii_table::Align::Left);
    ascii_table.column(1).set_header("Domain").set_align(ascii_table::Align::Left);
    ascii_table.column(2).set_header("Node").set_align(ascii_table::Align::Left);
    ascii_table.column(3).set_header("Exec (ms) Avg").set_align(ascii_table::Align::Left);
    ascii_table.column(4).set_header("Min").set_align(ascii_table::Align::Left);
    ascii_table.column(5).set_header("Max").set_align(ascii_table::Align::Left);
    ascii_table.column(6).set_header("Transfer (ms) Avg").set_align(ascii_table::Align::Left);
    ascii_table.column(7).set_header("Min").set_align(ascii_table::Align::Left);
    ascii_table.column(8).set_header("Max").set_align(ascii_table::Align::Left);

    let lid_to_pid = proxy.fetch_instances_to_physical_ids();
    let fun_lid_to_node_id: std::collections::HashMap<edgeless_api::function_instance::ComponentId, edgeless_api::function_instance::ComponentId> =
        proxy
            .fetch_function_instances_to_nodes()
            .into_iter()
            .map(|(lid, node_id_vec)| (lid, *node_id_vec.first().unwrap()))
            .collect();
    let mut lid_to_node_id = proxy.fetch_resource_instances_to_nodes();
    lid_to_node_id.extend(fun_lid_to_node_id);

    let mut data: Vec<Vec<String>> = vec![];
    let node_id_none = uuid::Uuid::nil();
    if let Ok(workflow_info) = edgeless_cli.inspect(wf_id.clone()).await {
        for edgeless_api::workflow_instance::WorkflowFunctionMapping {
            name,
            function_id: lid,
            domain_id,
        } in &workflow_info.status.domain_mapping
        {
            if let Some(pid) = lid_to_pid.get(lid) {
                let pid = pid.first().unwrap_or(&node_id_none);
                let node_id = lid_to_node_id.get(lid).unwrap_or(&node_id_none);
                let exec_times = proxy.fetch_performance_series(&pid.to_string(), "function_execution_time");
                let tran_times = proxy.fetch_performance_series(&pid.to_string(), "function_transfer_time");

                let (exec_avg, exec_min, exec_max) = avg_min_max(&exec_times);
                let (tran_avg, tran_min, tran_max) = avg_min_max(&tran_times);
                data.push(vec![
                    name.clone(),
                    domain_id.clone(),
                    map_node(node_id).to_string(),
                    format!("{:.1}", exec_avg),
                    format!("{:.1}", exec_min),
                    format!("{:.1}", exec_max),
                    format!("{:.1}", tran_avg),
                    format!("{:.1}", tran_min),
                    format!("{:.1}", tran_max),
                ]);
            } else {
                data.push(vec![
                    name.clone(),
                    domain_id.clone(),
                    String::default(),
                    String::default(),
                    String::default(),
                    String::default(),
                    String::default(),
                    String::default(),
                    String::default(),
                ]);
            }
        }
    }
    let _ = write!(stdout, "{}", ascii_table.format(data));

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        Commands::Top { top_command } => {
            let mut stdout = std::io::stdout();

            let mut wf_id = edgeless_api::workflow_instance::WorkflowId::none();
            let mut edgeless_cli = match top_command {
                TopCommands::Workflow { wf_id: wf_id_string } => {
                    wf_id = edgeless_api::workflow_instance::WorkflowId::from_string(&wf_id_string);
                    Some(
                        edgeless_api::grpc_impl::outer::controller::ControllerAPIClient::new(&args.controller_url)
                            .await
                            .workflow_instance_api(),
                    )
                }
                _ => None,
            };

            loop {
                write!(stdout, "{}", termion::clear::All).unwrap();

                if let Some(edgeless_cli) = &mut edgeless_cli {
                    top_workflow(&mut proxy, edgeless_cli, &wf_id, &mut stdout, &map_node).await?;
                } else {
                    top_nodes(&mut proxy, &mut stdout, &map_node)?;
                }

                stdout.flush()?;

                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
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
                for (pid, inner_map) in proxy.fetch_performance_samples() {
                    for (metric, values) in inner_map {
                        let filename = format!("{}-{}.dat", metric, pid);
                        let mut outfile = open_file(&filename)?;
                        for value in values {
                            outfile
                                .write_all(format!("{},{}\n", value.0, value.1).as_bytes())
                                .unwrap_or_else(|_| panic!("could not write to file '{}'", filename));
                        }
                    }
                }
            }
            DumpCommands::PerformanceCsv {} => {
                let mut client = edgeless_api::grpc_impl::outer::controller::ControllerAPIClient::new(&args.controller_url)
                    .await
                    .workflow_instance_api();
                let mut lid_to_name = std::collections::HashMap::new();
                if let Ok(wf_ids) = client.list().await {
                    for wf_id in wf_ids {
                        if let Ok(workflow_info) = client.inspect(wf_id).await {
                            for edgeless_api::workflow_instance::WorkflowFunctionMapping {
                                name,
                                function_id: lid,
                                domain_id: _,
                            } in &workflow_info.status.domain_mapping
                            {
                                lid_to_name.insert(lid.to_string(), name.clone());
                            }
                        }
                    }
                }

                println!("metric,name,lid,timestamp,value");
                let pid_to_lid: std::collections::HashMap<String, String> = proxy
                    .fetch_instances_to_physical_ids()
                    .iter()
                    .map(|(lid, pid_vec)| (pid_vec.first().unwrap_or(&uuid::Uuid::nil()).to_string(), lid.to_string()))
                    .collect();
                for (pid, inner_map) in proxy.fetch_performance_samples() {
                    for (metric, values) in inner_map {
                        for (timestamp, value) in values {
                            let lid = pid_to_lid.get(&pid).cloned().unwrap_or(String::default());
                            let name = lid_to_name.get(&lid).cloned().unwrap_or(String::default());
                            println!("{metric},{lid},{name},{timestamp},{value}");
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
