// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
use clap::Parser;
use edgeless_node::resources::dda::DdaResourceSpec;
use edgeless_node::resources::file_log::FileLogResourceSpec;
use edgeless_node::resources::file_pusher::FilePusherResourceSpec;
use edgeless_node::resources::http_egress::HttpEgressResourceSpec;
use edgeless_node::resources::http_ingress::HttpIngressResourceSpec;
use edgeless_node::resources::http_poster::HttpPosterResourceSpec;
#[cfg(feature = "rdkafka")]
use edgeless_node::resources::kafka_egress::KafkaEgressResourceSpec;
use edgeless_node::resources::ollama::OllamaResourceSpec;
use edgeless_node::resources::redis::RedisResourceSpec;
use edgeless_node::resources::resource_provider_specs::ResourceProviderSpecOutput;
use edgeless_node::resources::resource_provider_specs::ResourceProviderSpecs;
use edgeless_node::resources::serverless::ServerlessResourceProviderSpec;
use edgeless_node::resources::sqlx::SqlxResourceSpec;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("node.toml"))]
    config_file: String,
    #[arg(short, long, default_value_t = String::from(""))]
    template: String,
    #[arg(long, default_value_t = false)]
    available_resources: bool,
    #[arg(long, default_value_t = false)]
    output_json: bool,
    /// Print the version number and quit.
    #[arg(long, default_value_t = false)]
    version: bool,
}

fn read_conf_from_file(filename: &str) -> anyhow::Result<edgeless_node::EdgelessNodeSettings> {
    Ok(toml::from_str::<edgeless_node::EdgelessNodeSettings>(&std::fs::read_to_string(
        filename,
    )?)?)
}

fn main() -> anyhow::Result<()> {
    // NOTE: to debug the edgeless_node using the tokio-console, exchange the
    // env_logger::init() with the next line. These two options are exclusive.
    env_logger::init();
    // console_subscriber::init();
    edgeless_api::grpc_impl::init_crypto();
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
    // Create a template node configuration and exit.
    if !args.template.is_empty() {
        edgeless_api::util::create_template(&args.template, edgeless_node::edgeless_node_default_conf().as_str())?;
        return Ok(());
    }

    // Read the configuration file.
    let conf = read_conf_from_file(&args.config_file);

    // Print the available resources and exit.
    if args.available_resources {
        #[allow(unused_mut)]
        let mut specs: Vec<Box<dyn ResourceProviderSpecs>> = vec![
            Box::new(DdaResourceSpec {}),
            Box::new(FileLogResourceSpec {}),
            Box::new(HttpEgressResourceSpec {}),
            Box::new(HttpIngressResourceSpec {}),
            Box::new(HttpPosterResourceSpec {}),
            Box::new(FilePusherResourceSpec {}),
            Box::new(OllamaResourceSpec {}),
            Box::new(RedisResourceSpec {}),
            Box::new(SqlxResourceSpec {}),
        ];
        #[cfg(feature = "rdkafka")]
        specs.push(Box::new(KafkaEgressResourceSpec {}));
        if let Ok(conf) = &conf
            && let Some(resources) = &conf.resources
            && let Some(serverless_providers) = &resources.serverless_provider
        {
            for settings in serverless_providers {
                specs.push(Box::new(ServerlessResourceProviderSpec::new(&settings.class_type, &settings.version)))
            }
        }

        if args.output_json {
            println!(
                "{}",
                serde_json::to_string(&specs.iter().map(|x| x.to_output()).collect::<Vec<ResourceProviderSpecOutput>>())
                    .expect("could not serialize available resources to JSON")
            );
        } else {
            for spec in specs {
                println!("----------");
                println!("class_type: {}", spec.class_type());
                println!("version: {}", spec.version());
                println!("outputs: [{}]", spec.outputs().join(","));
                if !spec.configurations().is_empty() {
                    println!("configurations:");
                    println!(
                        "{}",
                        spec.configurations()
                            .iter()
                            .map(|(field, desc)| format!("  - {}: {}", field, desc))
                            .collect::<Vec<String>>()
                            .join("\n")
                    )
                }
                println!("description:\n{}", spec.description());
                println!();
            }
        }
        return Ok(());
    }

    let async_runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let async_tasks = vec![async_runtime.spawn(edgeless_node::edgeless_node_main(conf?))];

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}
