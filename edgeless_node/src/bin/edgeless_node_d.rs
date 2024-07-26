// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("node.toml"))]
    config_file: String,
    #[arg(short, long, default_value_t = String::from(""))]
    template: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    // console_subscriber::init();

    let args = Args::parse();
    if !args.template.is_empty() {
        edgeless_api::util::create_template(&args.template, edgeless_node::edgeless_node_default_conf().as_str())?;
        return Ok(());
    }
    let conf: edgeless_node::EdgelessNodeSettings = toml::from_str(&std::fs::read_to_string(args.config_file)?)?;

    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let async_tasks = vec![async_runtime.spawn(edgeless_node::edgeless_node_main(conf.clone()))];

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}
