// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
mod workflow_spec;

use clap::Parser;
use edgeless_api::{controller::ControllerAPI, workflow_instance::SpawnWorkflowResponse};

#[derive(Debug, clap::Subcommand)]
enum WorkflowCommands {
    Start { spec_file: String },
    Stop { id: String },
    List {},
}

#[derive(Debug, clap::Subcommand)]
enum FunctionCommands {
    Build {
        spec_file: String,
    },
    Invoke {
        event_type: String,
        invocation_url: String,
        node_id: String,
        function_id: String,
        payload: String,
    },
}

#[derive(Debug, clap::Subcommand)]
enum Commands {
    Workflow {
        #[command(subcommand)]
        workflow_command: WorkflowCommands,
    },
    Function {
        #[command(subcommand)]
        function_command: FunctionCommands,
    },
}

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long, default_value_t = String::from("cli.toml"))]
    config_file: String,
    #[arg(short, long, default_value_t = String::from(""))]
    template: String,
}

#[derive(serde::Deserialize)]
struct CLiConfig {
    controller_url: String,
}

pub fn edgeless_cli_default_conf() -> String {
    String::from("controller_url = \"http://127.0.0.1:7001\"")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    if !args.template.is_empty() {
        edgeless_api::util::create_template(&args.template, edgeless_cli_default_conf().as_str())?;
        return Ok(());
    }

    match args.command {
        None => log::debug!("Bye"),
        Some(x) => match x {
            Commands::Workflow { workflow_command } => {
                if std::fs::metadata(&args.config_file).is_err() {
                    return Err(anyhow::anyhow!(
                        "configuration file does not exist or cannot be accessed: {}",
                        &args.config_file
                    ));
                }
                log::debug!("Got Config");
                let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(args.config_file).unwrap()).unwrap();
                let mut con_client = edgeless_api::grpc_impl::controller::ControllerAPIClient::new(&conf.controller_url).await;
                let mut con_wf_client = con_client.workflow_instance_api();
                match workflow_command {
                    WorkflowCommands::Start { spec_file } => {
                        log::debug!("Start Workflow");
                        let workflow: workflow_spec::WorkflowSpec =
                            serde_json::from_str(&std::fs::read_to_string(spec_file.clone()).unwrap()).unwrap();
                        let res = con_wf_client
                            .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                                workflow_functions: workflow
                                    .functions
                                    .into_iter()
                                    .map(|func_spec| {
                                        let p = std::path::Path::new(&spec_file)
                                            .parent()
                                            .unwrap()
                                            .join(func_spec.class_specification.include_code_file.unwrap());
                                        edgeless_api::workflow_instance::WorkflowFunction {
                                            name: func_spec.name,
                                            function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                                                function_class_id: func_spec.class_specification.id,
                                                function_class_type: func_spec.class_specification.function_type,
                                                function_class_version: func_spec.class_specification.version,
                                                function_class_code: std::fs::read(p).unwrap(),
                                                outputs: func_spec.class_specification.outputs,
                                            },
                                            output_mapping: func_spec.output_mapping,
                                            annotations: func_spec.annotations,
                                        }
                                    })
                                    .collect(),
                                workflow_resources: workflow
                                    .resources
                                    .into_iter()
                                    .map(|res_spec| edgeless_api::workflow_instance::WorkflowResource {
                                        name: res_spec.name,
                                        class_type: res_spec.class_type,
                                        output_mapping: res_spec.output_mapping,
                                        configurations: res_spec.configurations,
                                    })
                                    .collect(),
                                annotations: workflow.annotations.clone(),
                            })
                            .await;
                        match res {
                            Ok(response) => {
                                match &response {
                                    SpawnWorkflowResponse::ResponseError(err) => {
                                        println!("{:?}", err);
                                    }
                                    SpawnWorkflowResponse::WorkflowInstance(val) => {
                                        println!("{}", val.workflow_id.workflow_id.to_string());
                                    }
                                }
                                log::info!("{:?}", response)
                            }
                            Err(err) => println!("{}", err),
                        }
                    }
                    WorkflowCommands::Stop { id } => {
                        let parsed_id = uuid::Uuid::parse_str(&id)?;
                        match con_wf_client
                            .stop(edgeless_api::workflow_instance::WorkflowId { workflow_id: parsed_id })
                            .await
                        {
                            Ok(_) => println!("Workflow Stopped"),
                            Err(err) => println!("{}", err),
                        }
                    }
                    WorkflowCommands::List {} => match con_wf_client.list(edgeless_api::workflow_instance::WorkflowId::none()).await {
                        Ok(instances) => {
                            for instance in instances.iter() {
                                println!("workflow: {}", instance.workflow_id.to_string());
                                for function in instance.domain_mapping.iter() {
                                    println!("\t{:?}", function);
                                }
                            }
                        }
                        Err(err) => println!("{}", err),
                    },
                }
            }
            Commands::Function { function_command } => match function_command {
                FunctionCommands::Build { spec_file } => {
                    let spec_file_path = std::fs::canonicalize(std::path::PathBuf::from(spec_file.clone()))?;
                    let cargo_project_path = spec_file_path.parent().unwrap().to_path_buf();
                    let cargo_manifest = cargo_project_path.join("Cargo.toml");

                    let function_spec: workflow_spec::WorkflowSpecFunctionClass = serde_json::from_str(&std::fs::read_to_string(spec_file.clone())?)?;
                    let build_dir = std::env::temp_dir().join(format!("edgeless-{}-{}", function_spec.id, uuid::Uuid::new_v4()));

                    let config = &cargo::util::config::Config::default()?;
                    let mut ws = cargo::core::Workspace::new(&cargo_manifest, config)?;
                    ws.set_target_dir(cargo::util::Filesystem::new(build_dir.clone()));

                    let pack = ws.current()?;

                    let lib_name = match pack.library() {
                        Some(val) => val.name(),
                        None => {
                            return Err(anyhow::anyhow!("Cargo package does not contain library."));
                        }
                    };

                    let mut build_config = cargo::core::compiler::BuildConfig::new(
                        config,
                        None,
                        false,
                        &vec!["wasm32-unknown-unknown".to_string()],
                        cargo::core::compiler::CompileMode::Build,
                    )?;
                    build_config.requested_profile = cargo::util::interning::InternedString::new("release");

                    let compile_options = cargo::ops::CompileOptions {
                        build_config: build_config,
                        cli_features: cargo::core::resolver::CliFeatures::new_all(false),
                        spec: cargo::ops::Packages::Packages(Vec::new()),
                        filter: cargo::ops::CompileFilter::Default {
                            required_features_filterable: false,
                        },
                        target_rustdoc_args: None,
                        target_rustc_args: None,
                        target_rustc_crate_types: None,
                        rustdoc_document_private_items: false,
                        honor_rust_version: true,
                    };

                    cargo::ops::compile(&ws, &compile_options)?;

                    let raw_result = build_dir
                        .join(format!("wasm32-unknown-unknown/release/{}.wasm", lib_name))
                        .to_str()
                        .unwrap()
                        .to_string();
                    let out_file = cargo_project_path
                        .join(format!("{}.wasm", function_spec.id))
                        .to_str()
                        .unwrap()
                        .to_string();

                    println!(
                        "{:?}",
                        std::process::Command::new("wasm-opt")
                            .args(["-Oz", &raw_result, "-o", &out_file])
                            .status()?
                    );
                }
                FunctionCommands::Invoke {
                    event_type,
                    invocation_url,
                    node_id,
                    function_id,
                    payload,
                } => {
                    log::info!("invoking function: {} {} {} {}", event_type, node_id, function_id, payload);
                    let mut client = edgeless_api::grpc_impl::invocation::InvocationAPIClient::new(&invocation_url).await;
                    let event = edgeless_api::invocation::Event {
                        target: edgeless_api::function_instance::InstanceId {
                            node_id: uuid::Uuid::parse_str(&node_id)?,
                            function_id: uuid::Uuid::parse_str(&function_id)?,
                        },
                        source: edgeless_api::function_instance::InstanceId::none(),
                        stream_id: 0,
                        data: match event_type.as_str() {
                            "cast" => edgeless_api::invocation::EventData::Cast(payload),
                            _ => return Err(anyhow::anyhow!("invalid event type: {}", event_type)),
                        },
                    };
                    match edgeless_api::invocation::InvocationAPI::handle(&mut client, event).await {
                        Ok(_) => println!("event casted"),
                        Err(err) => return Err(anyhow::anyhow!("error casting the event: {}", err)),
                    }
                }
            },
        },
    }
    Ok(())
}
