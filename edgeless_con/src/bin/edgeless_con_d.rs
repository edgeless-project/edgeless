// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    /// Path of the configuration file.
    #[arg(short, long, default_value_t = String::from("controller.toml"))]
    config_file: String,
    /// Create a template configuration file and quit immediately.
    #[arg(short, long, default_value_t = String::from(""))]
    template: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    if !args.template.is_empty() {
        edgeless_api::util::create_template(&args.template, edgeless_con::edgeless_con_default_conf().as_str())?;
        return Ok(());
    }
    let conf: edgeless_con::EdgelessConSettings = toml::from_str(&std::fs::read_to_string(args.config_file)?)?;
    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let async_tasks = vec![async_runtime.spawn(edgeless_con::edgeless_con_main(conf.clone()))];

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });

    Ok(())
}
