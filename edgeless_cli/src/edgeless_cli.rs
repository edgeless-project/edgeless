// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

mod workflow_spec;
use cargo::GlobalContext;
use clap::Parser;
use edgeless_api::{outer::controller::ControllerAPI, workflow_instance::SpawnWorkflowResponse};

use mailparse::{parse_content_disposition, parse_header};
use reqwest::header::ACCEPT;
use reqwest::{multipart, Body, Client};
use std::collections::HashMap;
use std::fs::{self};
use std::io::Cursor;
use std::time::SystemTime;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Debug, clap::Subcommand)]
enum WorkflowCommands {
    Start { spec_file: String },
    Stop { id: String },
    List {},
    Inspect { id: String },
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
    Get {
        function_name: String,
    },
    Download {
        code_file_id: String,
    },
    Push {
        binary_name: String,
        function_type: String,
    },
}

#[derive(Debug, clap::Subcommand)]
enum DomainCommands {
    Inspect { id: String },
    List {},
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
    Domain {
        #[command(subcommand)]
        domain_command: DomainCommands,
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

#[derive(serde::Deserialize, serde::Serialize)]
struct CLiConfig {
    controller_url: String,
    function_repository: Option<FunctionRepositoryConfig>,
}

impl Default for CLiConfig {
    fn default() -> Self {
        Self {
            controller_url: String::from("http://127.0.0.1:7001"),
            function_repository: Some(FunctionRepositoryConfig::default()),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
struct FunctionRepositoryConfig {
    pub url: String,
    pub basic_auth_user: String,
    pub basic_auth_pass: String,
}

pub fn edgeless_cli_default_conf() -> String {
    let cli_conf = CLiConfig::default();
    toml::to_string(&cli_conf).expect("Wrong")
}

async fn wf_client(config_file: &str) -> anyhow::Result<Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>> {
    anyhow::ensure!(
        std::fs::metadata(config_file).is_ok(),
        "configuration file does not exist or cannot be accessed: {}",
        config_file
    );

    let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(config_file).unwrap()).unwrap();
    let mut con_client = edgeless_api::grpc_impl::outer::controller::ControllerAPIClient::new(&conf.controller_url).await;
    Ok(con_client.workflow_instance_api())
}

async fn workflow_stop(wf_client: &mut Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>, id: &str) -> anyhow::Result<()> {
    anyhow::ensure!(workflow_info_or_none(wf_client, id).await.is_some(), "unknown or invalid workflow {}", id);
    wf_client
        .stop(edgeless_api::workflow_instance::WorkflowId {
            workflow_id: uuid::Uuid::parse_str(id)?,
        })
        .await?;
    println!("Workflow {} stopped", id);
    Ok(())
}

async fn workflow_info_or_none(
    wf_client: &mut Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>,
    id: &str,
) -> Option<edgeless_api::workflow_instance::WorkflowInfo> {
    let workflow_id = if let Ok(id) = uuid::Uuid::parse_str(id) { id } else { return None };
    match wf_client.inspect(edgeless_api::workflow_instance::WorkflowId { workflow_id }).await {
        Ok(info) => Some(info),
        Err(_) => None,
    }
}

async fn workflow_inspect(wf_client: &mut Box<dyn edgeless_api::workflow_instance::WorkflowInstanceAPI>, id: &str) -> anyhow::Result<()> {
    let info = workflow_info_or_none(wf_client, id).await;
    anyhow::ensure!(info.is_some(), "unknown or invalid workflow {}", id);
    let info = info.unwrap();
    assert_eq!(id, info.status.workflow_id.to_string());
    for fun in info.request.workflow_functions {
        println!("* function {}", fun.name);
        println!("{}", fun.function_class_specification.to_short_string());
        for (out, next) in fun.output_mapping {
            println!("OUT {} -> {}", out, next);
        }
        for (name, annotation) in fun.annotations {
            println!("F_ANN {} -> {}", name, annotation);
        }
    }
    for res in info.request.workflow_resources {
        println!("* resource {}", res.name);
        println!("{}", res.class_type);
        for (out, next) in res.output_mapping {
            println!("OUT {} -> {}", out, next);
        }
        for (name, annotation) in res.configurations {
            println!("CONF {} -> {}", name, annotation);
        }
    }
    println!("* mapping");
    for (name, annotation) in info.request.annotations {
        println!("W_ANN {} -> {}", name, annotation);
    }
    for mapping in info.status.domain_mapping {
        println!("MAP {} -> {} [logical ID {}]", mapping.name, mapping.domain_id, mapping.function_id);
    }
    Ok(())
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
                let mut wf_client = wf_client(&args.config_file).await?;
                match workflow_command {
                    WorkflowCommands::Start { spec_file } => {
                        log::debug!("Start Workflow");

                        let workflow_spec: edgeless_cli::workflow_spec::WorkflowSpec =
                            serde_json::from_str(&std::fs::read_to_string(spec_file.clone()).unwrap()).unwrap();
                        let parent_path = std::path::Path::new(&spec_file)
                            .parent()
                            .expect("cannot find the workflow spec's parent path");
                        let workflow = edgeless_cli::workflow_spec_to_request(workflow_spec, parent_path)?;
                        let res = wf_client.start(workflow).await;
                        match res {
                            Ok(response) => {
                                match &response {
                                    SpawnWorkflowResponse::ResponseError(err) => {
                                        println!("{:?}", err);
                                    }
                                    SpawnWorkflowResponse::WorkflowInstance(val) => {
                                        println!("{}", val.workflow_id.workflow_id);
                                    }
                                }
                                log::info!("{:?}", response)
                            }
                            Err(err) => println!("{}", err),
                        }
                    }
                    WorkflowCommands::Stop { id } => {
                        if id.to_lowercase() == "all" {
                            for wf_id in wf_client.list().await? {
                                workflow_stop(&mut wf_client, &wf_id.workflow_id.to_string()).await?
                            }
                        } else {
                            workflow_stop(&mut wf_client, &id).await?
                        }
                    }
                    WorkflowCommands::List {} => {
                        for wf_id in wf_client.list().await? {
                            println!("{}", wf_id);
                        }
                    }
                    WorkflowCommands::Inspect { id } => {
                        if id.to_lowercase() == "all" {
                            for wf_id in wf_client.list().await? {
                                println!("** workflow {}", wf_id);
                                workflow_inspect(&mut wf_client, &wf_id.workflow_id.to_string()).await?
                            }
                        } else {
                            workflow_inspect(&mut wf_client, &id).await?
                        }
                    }
                }
            }
            Commands::Function { function_command } => match function_command {
                FunctionCommands::Build { spec_file } => {
                    let spec_file_path = std::fs::canonicalize(std::path::PathBuf::from(spec_file.clone()))?;
                    let cargo_project_path = spec_file_path.parent().unwrap().to_path_buf();
                    let cargo_manifest = cargo_project_path.join("Cargo.toml");

                    let function_spec: workflow_spec::WorkflowSpecFunctionClass = serde_json::from_str(&std::fs::read_to_string(spec_file.clone())?)?;
                    let build_dir = std::env::temp_dir().join(format!("edgeless-{}-{}", function_spec.id, uuid::Uuid::new_v4()));

                    let context = GlobalContext::default().expect("Could not construct a global context for the workspace");
                    let mut ws = cargo::core::Workspace::new(&cargo_manifest, &context)?;
                    ws.set_target_dir(cargo::util::Filesystem::new(build_dir.clone()));

                    let out_file = cargo_project_path
                        .join(format!("{}.wasm", function_spec.id))
                        .to_str()
                        .unwrap()
                        .to_string();
                    // check if function.json, Cargo.toml, Cargo.lock or src/
                    // have been modified since the last time the function has
                    // been built. If not - skip the build.
                    let function_build_time = match fs::metadata(out_file.clone()) {
                        Ok(metadata) => metadata.modified().ok(),
                        Err(_) => None,
                    };

                    // standard files - could be extended to look for more
                    let triggering_files = ["Cargo.toml", "Cargo.lock", "function.json", "src/lib.rs"];
                    let full_paths: Vec<std::path::PathBuf> = triggering_files.iter().map(|&f| cargo_project_path.join(f)).collect();
                    let mod_timestamps: Vec<SystemTime> = full_paths
                        .iter()
                        .filter_map(|p| fs::metadata(p).ok())
                        .filter_map(|m| m.modified().ok())
                        .collect();
                    let last_modification = mod_timestamps
                        .iter()
                        .max()
                        .expect("Function to be build does not contain the required files.");

                    let should_rebuild = match function_build_time {
                        Some(t) => t < *last_modification, // we don't use the vector anywhere else, deref is fine
                        None => true,
                    };

                    if !should_rebuild {
                        log::info!("Skipping the function build, as no modifications to relevant files were detected.");
                        return Ok(());
                    } else {
                        if function_build_time.is_some() {
                            log::info!("Sources were modified - rebuilding the function.");
                        } else {
                            log::info!("Building the function for the first time.")
                        }
                        let pack = ws.current()?;

                        let lib_name = match pack.library() {
                            Some(val) => val.name(),
                            None => {
                                return Err(anyhow::anyhow!("Cargo package does not contain library."));
                            }
                        };

                        let mut build_config = cargo::core::compiler::BuildConfig::new(
                            &context,
                            None,
                            false,
                            &["wasm32-unknown-unknown".to_string()],
                            cargo::core::compiler::CompileMode::Build,
                        )?;
                        build_config.requested_profile = cargo::util::interning::InternedString::new("release");

                        let compile_options = cargo::ops::CompileOptions {
                            build_config,
                            cli_features: cargo::core::resolver::CliFeatures::new_all(false),
                            spec: cargo::ops::Packages::Packages(Vec::new()),
                            filter: cargo::ops::CompileFilter::Default {
                                required_features_filterable: false,
                            },
                            target_rustdoc_args: None,
                            target_rustc_args: None,
                            target_rustc_crate_types: None,
                            rustdoc_document_private_items: false,
                            honor_rust_version: Some(true),
                        };

                        cargo::ops::compile(&ws, &compile_options)?;

                        let raw_result = build_dir
                            .join(format!("wasm32-unknown-unknown/release/{}.wasm", lib_name))
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
                        created: edgeless_api::function_instance::EventTimestamp::default(),
                    };
                    match edgeless_api::invocation::InvocationAPI::handle(&mut client, event).await {
                        Ok(_) => println!("event casted"),
                        Err(err) => return Err(anyhow::anyhow!("error casting the event: {}", err)),
                    }
                }

                FunctionCommands::Get { function_name } => {
                    if std::fs::metadata(&args.config_file).is_err() {
                        return Err(anyhow::anyhow!(
                            "configuration file does not exist or cannot be accessed: {}",
                            &args.config_file
                        ));
                    }
                    log::debug!("Got Config");
                    let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(args.config_file).unwrap()).unwrap();
                    let function_repository_conf = match conf.function_repository {
                        Some(conf) => conf,
                        None => anyhow::bail!("function repository configuration section missing"),
                    };

                    let client = Client::new();
                    let response = client
                        .get(function_repository_conf.url.to_string() + "/api/admin/function/" + function_name.as_str())
                        .header(ACCEPT, "application/json")
                        .basic_auth(function_repository_conf.basic_auth_user, Some(function_repository_conf.basic_auth_pass))
                        .send()
                        .await
                        .expect("failed to get response")
                        .text()
                        .await
                        .expect("failed to get payload");

                    println!("Successfully get function {}", response);
                }

                FunctionCommands::Download { code_file_id } => {
                    if std::fs::metadata(&args.config_file).is_err() {
                        return Err(anyhow::anyhow!(
                            "configuration file does not exist or cannot be accessed: {}",
                            &args.config_file
                        ));
                    }
                    log::debug!("Got Config");
                    let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(args.config_file).unwrap()).unwrap();
                    let function_repository_conf = match conf.function_repository {
                        Some(conf) => conf,
                        None => anyhow::bail!("function repository configuration section missing"),
                    };

                    let client = Client::new();
                    let response = client
                        .get(function_repository_conf.url.to_string() + "/api/admin/function/download/" + code_file_id.as_str())
                        .header(ACCEPT, "*/*")
                        .basic_auth(function_repository_conf.basic_auth_user, Some(function_repository_conf.basic_auth_pass))
                        .send()
                        .await
                        .expect("failed to get header");
                    let status = response.status();
                    println!("status code {}", status);
                    let header = response.headers().get("content-disposition").unwrap();

                    let header_str = format!("{}{}", "Content-Disposition: ", header.to_str().unwrap());
                    let (parsed, _) = parse_header(header_str.as_bytes()).unwrap();
                    let dis = parse_content_disposition(&parsed.get_value());

                    let downloadfilename = dis.params.get("filename").unwrap();

                    println!("filename:\n{:?}", downloadfilename);

                    let body = response.bytes().await.expect("failed to download payload");

                    let mut file = std::fs::File::create(downloadfilename)?;
                    let mut content = Cursor::new(body);
                    std::io::copy(&mut content, &mut file)?;

                    println!("File downloaded successfully.");
                }

                FunctionCommands::Push { binary_name, function_type } => {
                    if std::fs::metadata(&args.config_file).is_err() {
                        return Err(anyhow::anyhow!(
                            "configuration file does not exist or cannot be accessed: {}",
                            &args.config_file
                        ));
                    }
                    log::debug!("Got Config");
                    let conf: CLiConfig = toml::from_str(&std::fs::read_to_string(&args.config_file).unwrap()).unwrap();
                    let function_repository_conf = match conf.function_repository {
                        Some(conf) => conf,
                        None => anyhow::bail!("function repository configuration section missing"),
                    };

                    let client = Client::new();
                    let file = File::open(&binary_name).await?;

                    // read file body stream
                    let stream = FramedRead::new(file, BytesCodec::new());
                    let file_body = Body::wrap_stream(stream);

                    //make form part of file
                    let some_file = multipart::Part::stream(file_body).file_name("binary"); // this is in curl -F "function_x86" in "file=@function_x86"

                    //create the multipart form
                    let form = multipart::Form::new().part("file", some_file); // this is in curl -F "file"

                    let response = client
                        .post(function_repository_conf.url.to_string() + "/api/admin/function/upload")
                        .header(ACCEPT, "application/json")
                        .basic_auth(
                            function_repository_conf.basic_auth_user.clone(),
                            Some(function_repository_conf.basic_auth_pass.clone()),
                        )
                        .multipart(form)
                        .send()
                        .await
                        .expect("failed to get response");

                    let json = response.json::<HashMap<String, String>>().await?;
                    println!("receive code_file_id {:?}", json);

                    let internal_id = &binary_name;
                    let r = serde_json::json!({

                        "function_type": function_type,
                        "id": internal_id,
                        "version": "0.1",
                        "code_file_id": json.get("id"), //get the id
                        "outputs": [  "success_cb",
                                      "failure_cb"
                                   ],
                    });

                    let post_response = client
                        .post(function_repository_conf.url.to_string() + "/api/admin/function")
                        .header(ACCEPT, "application/json")
                        .basic_auth(function_repository_conf.basic_auth_user, Some(function_repository_conf.basic_auth_pass))
                        .json(&r)
                        .send()
                        .await
                        .expect("failed to get response")
                        .text()
                        .await
                        .expect("failed to get body");
                    println!("post_response body: {:?}", post_response);
                }
            },
            Commands::Domain { domain_command } => {
                let mut wf_client = wf_client(&args.config_file).await?;
                match domain_command {
                    DomainCommands::List {} => {
                        for (domain_id, caps) in wf_client.domains(String::from("")).await? {
                            println!("domain {} ({} nodes)", domain_id, caps.num_nodes);
                        }
                    }
                    DomainCommands::Inspect { id } => {
                        if id.to_lowercase() == "all" {
                            let domains = wf_client.domains(Default::default()).await?;
                            for (domain, caps) in domains {
                                println!("domain {},{}", domain, caps);
                            }
                        } else {
                            let domains = wf_client.domains(id.clone()).await?;
                            match domains.get(&id) {
                                None => println!("domain {} not found", id),
                                Some(caps) => println!("domain {},{}", id, caps),
                            }
                        }
                    }
                }
            }
        },
    }
    Ok(())
}
