use clap::Parser;

#[derive(Debug, clap::Parser)]
#[command(long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = String::from("./edgeless_conf/node.toml"))]
    config_file: String,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args = Args::parse();
    let conf: edgeless_node::EdgelessNodeSettings = toml::from_str(&std::fs::read_to_string(args.config_file)?)?;

    let async_runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(8).enable_all().build()?;
    let mut async_tasks = vec![];

    async_tasks.push(async_runtime.spawn(edgeless_node::edgeless_node_main(conf.clone())));

    log::info!("Edgeless In A Box Mode");
    let orc_api_addr = "http://127.0.0.1:7011".to_string();
    let con_api_addr = "http://127.0.0.1:7021".to_string();

    let bal_invocation_url = "http://127.0.0.1:7032".to_string();
    let bal_rc_url = "http://127.0.0.1:7033".to_string();
    let bal_http_url = "http://127.0.0.1:7035".to_string();

    let bal_settings = edgeless_bal::EdgelessBalSettings {
        balancer_id: uuid::Uuid::parse_str("2bb0867f-e9ee-4a3a-8872-dbaa5228ee23").unwrap(),
        invocation_url: bal_invocation_url,
        resource_configuration_url: bal_rc_url,
        http_ingress_url: bal_http_url,
        nodes: vec![edgeless_dataplane::EdgelessDataplaneSettingsPeer {
            id: conf.node_id.clone(),
            invocation_url: conf.invocation_url.clone(),
        }],
    };

    async_tasks.push(async_runtime.spawn(edgeless_bal::edgeless_bal_main(bal_settings)));
    let orc_config = edgeless_orc::EdgelessOrcSettings {
        domain_id: "domain-1".to_string(),
        orchestrator_url: orc_api_addr.clone(),
        nodes: vec![edgeless_orc::EdgelessOrcNodeConfig {
            node_id: conf.node_id.clone(),
            agent_url: conf.agent_url.clone(),
        }],
    };
    async_tasks.push(async_runtime.spawn(edgeless_orc::edgeless_orc_main(orc_config)));
    let con_config = edgeless_con::EdgelessConSettings {
        controller_url: con_api_addr.clone(),
        orchestrators: vec![edgeless_con::EdgelessConOrcConfig {
            domain_id: "domain-1".to_string(),
            orchestrator_url: orc_api_addr.clone(),
        }],
        resources: vec![edgeless_con::EdgelessConResourceConfig {
            resource_provider_id: "http-ingress-1".to_string(),
            resource_class_type: "http-ingress".to_string(),
            output_callback_declarations: vec!["new_request".to_string()],
            resource_configuration_url: "http://127.0.0.1:7033".to_string(),
        }],
    };
    async_tasks.push(async_runtime.spawn(edgeless_con::edgeless_con_main(con_config)));

    async_runtime.block_on(async { futures::future::join_all(async_tasks).await });
    Ok(())
}
