use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("./edgeless_conf/balancer.toml"))]
    config_file: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    let conf: edgeless_bal::EdgelessBalSettings = toml::from_str(&std::fs::read_to_string(args.config_file)?)?;

    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let mut async_tasks = vec![];

    async_tasks.push(async_runtime.spawn(edgeless_bal::edgeless_bal_main(conf.clone())));

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}
