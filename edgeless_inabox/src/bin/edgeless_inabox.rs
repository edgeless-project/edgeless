use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("node.toml"))]
    node_config_file: String,
    #[arg(short, long, default_value_t = String::from("orchestrator.toml"))]
    orc_config_file: String,
    #[arg(short, long, default_value_t = String::from("balancer.toml"))]
    bal_config_file: String,
    #[arg(short, long, default_value_t = String::from("controller.toml"))]
    con_config_file: String,
    #[arg(short, long)]
    templates: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    if args.templates {
        edgeless_api::util::create_template("node.toml", edgeless_node::edgeless_node_default_conf().as_str())?;
        edgeless_api::util::create_template("orchestrator.toml", edgeless_orc::edgeless_orc_default_conf().as_str())?;
        edgeless_api::util::create_template("balancer.toml", edgeless_bal::edgeless_bal_default_conf().as_str())?;
        edgeless_api::util::create_template("controller.toml", edgeless_con::edgeless_con_default_conf().as_str())?;
        return Ok(());
    }

    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let mut async_tasks = vec![];

    edgeless_inabox::edgeless_inabox_main(
        &async_runtime,
        &mut async_tasks,
        &args.node_config_file,
        &args.orc_config_file,
        &args.bal_config_file,
        &args.con_config_file,
    )?;

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}
