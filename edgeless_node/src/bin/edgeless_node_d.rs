// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
use clap::Parser;
use edgeless_node::resources::dda::DdaResourceSpec;
use edgeless_node::resources::file_log::FileLogResourceSpec;
use edgeless_node::resources::http_egress::HttpEgressResourceSpec;
use edgeless_node::resources::http_ingress::HttpIngressResourceSpec;
#[cfg(feature = "rdkafka")]
use edgeless_node::resources::kafka_egress::KafkaEgressResourceSpec;
use edgeless_node::resources::ollama::OllamasResourceSpec;
use edgeless_node::resources::redis::RedisResourceSpec;
use edgeless_node::resources::resource_provider_specs::ResourceProviderSpecs;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("node.toml"))]
    config_file: String,
    #[arg(short, long, default_value_t = String::from(""))]
    template: String,
    #[arg(long, default_value_t = false)]
    available_resources: bool,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    // NOTE: to debug the edgeless_node using the tokio-console, exchange the
    // env_logger::init() with the next line. These two options are exclusive.
    // console_subscriber::init();

    let args = Args::parse();
    if args.available_resources {
        #[allow(unused_mut)]
        let mut specs: Vec<Box<dyn ResourceProviderSpecs>> = vec![
            Box::new(DdaResourceSpec {}),
            Box::new(FileLogResourceSpec {}),
            Box::new(HttpEgressResourceSpec {}),
            Box::new(HttpIngressResourceSpec {}),
            Box::new(OllamasResourceSpec {}),
            Box::new(RedisResourceSpec {}),
        ];
        #[cfg(feature = "rdkafka")]
        specs.push(Box::new(KafkaEgressResourceSpec {}));
        for spec in specs {
            println!("class_type: {}", spec.class_type());
            println!("version: {}", spec.version());
            println!("outputs: [{}]", spec.outputs().join(","));
            if !spec.configurations().is_empty() {
                println!("configurations:");
                println!(
                    "{}",
                    spec.configurations()
                        .iter()
                        .map(|(field, desc)| format!("- {}: {}", field, desc))
                        .collect::<Vec<String>>()
                        .join("\n")
                )
            }
            println!();
        }
        return Ok(());
    }
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
