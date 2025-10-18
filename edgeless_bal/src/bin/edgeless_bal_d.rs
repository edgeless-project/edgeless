// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("balancer.toml"))]
    config_file: String,
    #[arg(short, long, default_value_t = String::from(""))]
    template: String,
    /// Print the version number and quit.
    #[arg(long, default_value_t = false)]
    version: bool,
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
    if !args.template.is_empty() {
        edgeless_api::util::create_template(&args.template, edgeless_bal::edgeless_bal_default_conf().as_str())?;
        return Ok(());
    }
    let conf: edgeless_bal::EdgelessBalSettings = toml::from_str(&std::fs::read_to_string(args.config_file)?)?;

    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let async_tasks = vec![async_runtime.spawn(edgeless_bal::edgeless_bal_main(conf.clone()))];

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}
