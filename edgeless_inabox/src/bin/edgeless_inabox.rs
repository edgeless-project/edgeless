use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("./edgeless_conf/node.toml"))]
    node_config_file: String,
    #[arg(short, long, default_value_t = String::from("./edgeless_conf/orchestrator.toml"))]
    orc_config_file: String,
    #[arg(short, long, default_value_t = String::from("./edgeless_conf/balancer.toml"))]
    bal_config_file: String,
    #[arg(short, long, default_value_t = String::from("./edgeless_conf/controller.toml"))]
    con_config_file: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    let node_conf: edgeless_node::EdgelessNodeSettings = toml::from_str(&std::fs::read_to_string(args.node_config_file)?)?;
    let orc_conf: edgeless_orc::EdgelessOrcSettings = toml::from_str(&std::fs::read_to_string(args.orc_config_file)?)?;
    let bal_conf: edgeless_bal::EdgelessBalSettings = toml::from_str(&std::fs::read_to_string(args.bal_config_file)?)?;
    let con_conf: edgeless_con::EdgelessConSettings = toml::from_str(&std::fs::read_to_string(args.con_config_file)?)?;

    log::info!("Edgeless In A Box");

    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let mut async_tasks = vec![];

    async_tasks.push(async_runtime.spawn(edgeless_node::edgeless_node_main(node_conf.clone())));
    async_tasks.push(async_runtime.spawn(edgeless_bal::edgeless_bal_main(bal_conf.clone())));
    async_tasks.push(async_runtime.spawn(edgeless_orc::edgeless_orc_main(orc_conf.clone())));
    async_tasks.push(async_runtime.spawn(edgeless_con::edgeless_con_main(con_conf.clone())));

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}
