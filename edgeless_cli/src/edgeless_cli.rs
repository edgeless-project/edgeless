mod workflow_spec;

use clap::Parser;
use edgeless_api::controller::ControllerAPI;

#[derive(Debug, clap::Subcommand)]
enum WorkflowCommands {
    Start { spec_file: String },
    Stop { id: String },
}

#[derive(Debug, clap::Subcommand)]
enum FunctionCommands {
    Build { spec_file: String },
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
    String::from("controller_url = \"http://127.0.0.1:7021\"")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if !args.template.is_empty() {
        edgeless_api::util::create_template(&args.template, edgeless_cli_default_conf().as_str())?;
        return Ok(());
    }

    match args.command {
        None => println!("Bye"),
        Some(x) => {
            let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(args.config_file)?)?;
            let mut con_client = edgeless_api::grpc_impl::controller::ControllerAPIClient::new(&conf.controller_url).await;
            let mut con_wf_client = con_client.workflow_instance_api();
            match x {
                Commands::Workflow { workflow_command } => match workflow_command {
                    WorkflowCommands::Start { spec_file } => {
                        let workflow: workflow_spec::WorkflowSpec = serde_json::from_str(&std::fs::read_to_string(spec_file.clone())?)?;
                        let my_wf_id = edgeless_api::workflow_instance::WorkflowId {
                            workflow_id: uuid::Uuid::new_v4(),
                        };
                        let res = con_wf_client
                            .start_workflow_instance(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                                workflow_id: my_wf_id.clone(),
                                workflow_functions: workflow
                                    .functions
                                    .into_iter()
                                    .map(|wf| {
                                        let p = std::path::Path::new(&spec_file)
                                            .parent()
                                            .unwrap()
                                            .join(wf.class_specification.include_code_file.unwrap());
                                        edgeless_api::workflow_instance::WorkflowFunction {
                                            function_alias: wf.alias,
                                            function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                                                function_class_id: wf.class_specification.id,
                                                function_class_type: wf.class_specification.function_type,
                                                function_class_version: wf.class_specification.version,
                                                function_class_inlude_code: std::fs::read(p).unwrap(),
                                                output_callback_declarations: wf.class_specification.output_callbacks,
                                            },
                                            output_callback_definitions: wf.output_callback_definitions,
                                            return_continuation: "UNUSED".to_string(),
                                            function_annotations: wf.annotations,
                                        }
                                    })
                                    .collect(),
                                workflow_resources: workflow
                                    .resources
                                    .into_iter()
                                    .map(|wr| edgeless_api::workflow_instance::WorkflowResource {
                                        alias: wr.alias,
                                        resource_class_type: wr.resource_class_type,
                                        output_callback_definitions: wr.output_callback_definitions,
                                        configurations: wr.configurations,
                                    })
                                    .collect(),
                                workflow_annotations: workflow.annotations.clone(),
                            })
                            .await;
                        if let Ok(instance) = res {
                            println!("{}", instance.workflow_id.workflow_id.to_string())
                        }
                    }
                    WorkflowCommands::Stop { id } => {
                        let parsed_id = uuid::Uuid::parse_str(&id)?;
                        let res = con_wf_client
                            .stop_workflow_instance(edgeless_api::workflow_instance::WorkflowId { workflow_id: parsed_id })
                            .await;
                        if let Ok(_) = res {
                            println!("Workflow Stopped");
                        }
                    }
                },
                Commands::Function { function_command } => match function_command {
                    FunctionCommands::Build { spec_file } => {
                        let spec_file_path = std::fs::canonicalize(std::path::PathBuf::from(spec_file.clone()))?;
                        let cargo_project_path = spec_file_path.parent().unwrap().to_path_buf();
                        let cargo_manifest = cargo_project_path.join("Cargo.toml");

                        let function_spec: workflow_spec::WorkflowSpecFunctionClass =
                            serde_json::from_str(&std::fs::read_to_string(spec_file.clone())?)?;
                        let build_dir = std::env::temp_dir().join(format!("edgeless-{}-{}", function_spec.id, uuid::Uuid::new_v4()));

                        let config = &cargo::util::config::Config::default()?;
                        let mut ws = cargo::core::Workspace::new(&cargo_manifest, config.clone())?;
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
                            std::process::Command::new("wasm-tools")
                                .args(["component", "new", &raw_result, "-o", &out_file])
                                .status()?
                        );
                    }
                },
            }
        }
    }
    Ok(())
}
