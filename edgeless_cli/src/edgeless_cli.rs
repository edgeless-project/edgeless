// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 University of Cambridge, System Research Group
// SPDX-FileCopyrightText: © 2024 Roman Kolcun <roman.kolcun@cl.cam.ac.uk>
// SPDX-License-Identifier: MIT
mod workflow_spec;

use clap::Parser;
use edgeless_api::{controller::ControllerAPI, workflow_instance::SpawnWorkflowResponse};
use reqwest::header::ACCEPT;
use std::fs;
use toml;

use reqwest::{multipart, Body, Client};
use std::collections::HashMap;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

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
        #[arg(short, long, default_value_t = String::from("wasm"))]
        architecture: String,
    },
    Invoke {
        event_type: String,
        invocation_url: String,
        node_id: String,
        function_id: String,
        payload: String,
    },
    Push {
        file_name: String,
    },
    Get {
        file_name: String,
        id: String,
    }
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

enum Platform {
    WASM,
    X86,
    ARM,
}

impl Platform {
    fn target(&self) -> String {
        match self {
            Self::WASM => String::from("wasm32-unknown-unknown"),
            Self::X86 => String::from("x86_64-unknown-linux-gnu"),
            Self::ARM => String::from("aarch64-unknown-linux-gnu"),
        }
    }

    fn suffix(&self) -> String {
        match self {
            Self::WASM => String::from("wasm"),
            Self::X86 => String::from("so"),
            Self::ARM => String::from("arm"),
        }
    }

    fn name(&self) -> String {
        match self {
            Self::WASM => String::from("WASM"),
            Self::X86 => String::from("x86"),
            Self::ARM => String::from("Arm"),
        }
    }
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
                                            .join(func_spec.class_specification.code.unwrap());
                                        edgeless_api::workflow_instance::WorkflowFunction {
                                            name: func_spec.name,
                                            function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
                                                function_class_id: func_spec.class_specification.id,
                                                function_class_type: func_spec.class_specification.function_type,
                                                function_class_version: func_spec.class_specification.version,
                                                function_class_code: std::fs::read(p).unwrap(),
                                                function_class_outputs: func_spec.class_specification.outputs,
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
                FunctionCommands::Build { spec_file, architecture } => {
                    let spec_file_path = std::fs::canonicalize(std::path::PathBuf::from(spec_file.clone()))?;
                    let cargo_project_path = spec_file_path.parent().unwrap().to_path_buf();
                    let cargo_manifest = cargo_project_path.join("Cargo.toml");

                    let function_spec: workflow_spec::WorkflowSpecFunctionClass = serde_json::from_str(&std::fs::read_to_string(spec_file.clone())?)?;
                    let build_dir = std::env::temp_dir().join(format!("edgeless-{}-{}", function_spec.id, uuid::Uuid::new_v4()));

                    let config = &cargo::util::config::Config::default()?;
                    let mut ws = cargo::core::Workspace::new(&cargo_manifest, config)?;
                    ws.set_target_dir(cargo::util::Filesystem::new(build_dir.clone()));

                    let pack = ws.current()?;

                    let platform = match architecture.as_str() {
                        "wasm" => Platform::WASM,
                        "x86" => Platform::X86,
                        "arm" => Platform::ARM,
                        _ => Platform::WASM,
                    };

                    println!("Building for architecture {} using {} target.", platform.name(), platform.target());

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
                        //&vec!["wasm32-unknown-unknown".to_string()],
                        &vec![platform.target()],
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
                        .join(format!("{}/release/{}.{}", platform.target(), lib_name, platform.suffix()))
                        .to_str()
                        .unwrap()
                        .to_string();
                    let out_file = cargo_project_path
                        .join(format!("{}.{}", function_spec.id, platform.suffix()))
                        .to_str()
                        .unwrap()
                        .to_string();

                    match platform {
                        Platform::WASM => println!(
                            "{:?}",
                            std::process::Command::new("wasm-opt")
                                .args(["-Oz", &raw_result, "-o", &out_file])
                                .status()?
                        ),
                        _ => fs::write(&out_file, &raw_result).expect("Unable to write file"),
                    }
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

                FunctionCommands::Push {
                    //toml file specify the function
                    file_name,
                } => {
                    let filename = file_name;
                    //read end point and credentials in a file
                    let contents = fs::read_to_string(filename).expect("Failed to read file, please make sure the file exist and in .toml");
                    // println!("contents {}", contents); //
                    let repo_endpoint: workflow_spec::RepoEndpoint = toml::from_str(&contents).expect("invalid config");

                    // Print out the values to `stdout`.
                    println!("Url {}", repo_endpoint.url.name); //
                    println!("username {}", repo_endpoint.credential.basic_auth_user); //
                    println!("passwd {}", repo_endpoint.credential.basic_auth_pass); //
                                                                                     //create a curl request as follow
                                                                                     // curl -X 'POST' \
                                                                                     //    'https://function-repository.edgeless.wlilab.eu/api/admin/function/upload' \
                                                                                     //    -H 'accept: application/json' \
                                                                                     //    -H 'Content-Type: multipart/form-data' \
                                                                                     //    -F 'file=@function_x86'
                                                                                     //use multipart
                    let client = Client::new();
                    let file = File::open(repo_endpoint.binary.name).await?;

                    // read file body stream
                    let stream = FramedRead::new(file, BytesCodec::new());
                    let file_body = Body::wrap_stream(stream);

                    //make form part of file
                    let some_file = multipart::Part::stream(file_body).file_name("function_x86"); // this is in curl -F "function_x86" in "file=@function_x86"

                    //create the multipart form
                    let form = multipart::Form::new().part("file", some_file); // this is in curl -F "file"

                    let response = client
                        .post(repo_endpoint.url.name.to_string() + "/api/admin/function/upload")
                        .header(ACCEPT, "application/json")
                        .basic_auth(repo_endpoint.credential.basic_auth_user, Some(repo_endpoint.credential.basic_auth_pass))
                        .multipart(form)
                        .send()
                        .await
                        .expect("failed to get response");

                    let json = response.json::<HashMap<String, String>>().await?;
                    println!("receive code_file_id {:?}", json);

                    //post to /api/admin/function
                    //example
                    // curl -X 'POST' \
                    //   'https://function-repository.edgeless.wlilab.eu/api/admin/function' \
                    //   -H 'accept: application/json' \
                    //   -H 'Content-Type: application/json' \
                    //   -d '{
                    //   "function_type": "RUST_WASM",
                    //   "id": "http_requestor",
                    //   "version": "0.1",
                    //   "code_file_id": "652faf54465c2e7ec15facce",
                    //   "outputs": [
                    //     "success_cb",
                    //     "failure_cb"
                    //   ]
                    // }'

                    let r = serde_json::json!({

                        "function_type": "RUST_WASM",
                        "id": repo_endpoint.binary.id,
                        "version": "0.1",
                        "code_file_id": json.get("id"), //get the id
                        "outputs": [  "success_cb",
                                      "failure_cb"
                                   ],
                    });

                    let repo_endpoint_new: workflow_spec::RepoEndpoint = toml::from_str(&contents).expect("invalid config");
                    let post_response = client
                        .post(repo_endpoint_new.url.name.to_string() + "/api/admin/function")
                        .header(ACCEPT, "application/json")
                        .basic_auth(
                            repo_endpoint_new.credential.basic_auth_user,
                            Some(repo_endpoint_new.credential.basic_auth_pass),
                        )
                        .json(&r)
                        .send()
                        .await
                        .expect("failed to get response")
                        .text()
                        .await
                        .expect("failed to get body");
                    println!("post_response body: {:?}", post_response);
                    println!("Post function successfully!");
                }

                FunctionCommands::Get {
                    file_name, //config file name
                    id,  // wordline id, actually function name
                } => {
                    let filename = file_name;
                    //read end point and credentials in a file
                    let contents = fs::read_to_string(filename).expect("Failed to read file, please make sure the file exist and in .toml");
                    // println!("contents {}", contents); //
                    let repo_endpoint: workflow_spec::RepoEndpoint = toml::from_str(&contents).expect("invalid config");

                    // Print out the values to `stdout`.
                    println!("Url {}", repo_endpoint.url.name); //
                    println!("username {}", repo_endpoint.credential.basic_auth_user); //
                    println!("passwd {}", repo_endpoint.credential.basic_auth_pass); //
                                                                                     //create a curl request as follow
                                                                                     // curl -X 'POST' \
                                                                                     //    'https://function-repository.edgeless.wlilab.eu/api/admin/function/upload' \
                                                                                     //    -H 'accept: application/json' \
                                                                                     //    -H 'Content-Type: multipart/form-data' \
                                                                                     //    -F 'file=@function_x86'
                                                                                     //use multipart
                    let client = Client::new();
                    let response = client
                        .get(repo_endpoint.url.name.to_string() + "/api/admin/function/" + id.as_str())
                        .header(ACCEPT, "application/json")
                        .basic_auth(repo_endpoint.credential.basic_auth_user, Some(repo_endpoint.credential.basic_auth_pass))
                        .send()
                        .await
                        .expect("failed to get response")
                        .text()
                        .await
                        .expect("failed to get payload");
                        

                    println!("Successfully get function {}", response);
                   


                }
            },
        },
    }
    Ok(())
}
