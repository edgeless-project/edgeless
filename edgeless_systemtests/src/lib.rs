// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[cfg(test)]
mod tests {
    // use super::*;

    fn string_to_instance_id(val: &str) -> edgeless_api::function_instance::InstanceId {
        let tokens: Vec<&str> = val.split(' ').collect();
        if tokens.len() == 4 {
            let node_id = match uuid::Uuid::from_str(&tokens[1][0..tokens[1].len() - 1]) {
                Ok(val) => val,
                Err(_) => uuid::Uuid::nil(),
            };
            let function_id = match uuid::Uuid::from_str(&tokens[3][0..tokens[3].len() - 1]) {
                Ok(val) => val,
                Err(_) => uuid::Uuid::nil(),
            };
            edgeless_api::function_instance::InstanceId { node_id, function_id }
        } else {
            edgeless_api::function_instance::InstanceId::none()
        }
    }

    // Data structure clone of ActiveInstance, which can be deserialized.
    #[derive(Clone, serde::Deserialize)]
    pub enum ActiveInstanceClone {
        // 0: request
        // 1: [ (node_id, int_fid) ]
        Function(edgeless_api::function_instance::SpawnFunctionRequest, Vec<String>),

        // 0: request
        // 1: (node_id, int_fid)
        Resource(edgeless_api::resource_configuration::ResourceInstanceSpecification, String),
    }

    impl ActiveInstanceClone {
        fn node_id(&self) -> edgeless_api::function_instance::NodeId {
            string_to_instance_id(match self {
                ActiveInstanceClone::Function(_, instances) => match instances.first() {
                    Some(val) => val,
                    None => return uuid::Uuid::nil(),
                },
                ActiveInstanceClone::Resource(_, instance) => instance,
            })
            .node_id
        }
    }

    use std::str::FromStr;

    use edgeless_api::controller::ControllerAPI;
    use edgeless_api::workflow_instance::WorkflowInstanceAPI;
    use redis::Commands;

    fn redis_node_ids(connection: &mut redis::Connection) -> Vec<edgeless_api::function_instance::NodeId> {
        let mut uuids = vec![];
        for node_key in connection.keys::<&str, Vec<String>>("node:capabilities:*").unwrap_or(vec![]) {
            let tokens: Vec<&str> = node_key.split(':').collect();
            assert_eq!(tokens.len(), 3);
            if let Ok(uuid) = edgeless_api::function_instance::NodeId::parse_str(tokens[2]) {
                uuids.push(uuid);
            }
        }
        uuids
    }

    fn redis_instances(
        connection: &mut redis::Connection,
    ) -> std::collections::HashMap<edgeless_api::function_instance::ComponentId, ActiveInstanceClone> {
        let mut instance_ids = vec![];
        for instance_key in connection.keys::<&str, Vec<String>>("instance:*").unwrap_or(vec![]) {
            let tokens: Vec<&str> = instance_key.split(':').collect();
            assert_eq!(tokens.len(), 2);
            if let Ok(uuid) = edgeless_api::function_instance::NodeId::parse_str(tokens[1]) {
                instance_ids.push(uuid);
            }
        }
        let mut instances = std::collections::HashMap::new();
        for instance_id in instance_ids {
            if let Ok(val) = connection.get::<String, String>(format!("instance:{}", instance_id.to_string())) {
                if let Ok(val) = serde_json::from_str(&val) {
                    instances.insert(instance_id, val);
                }
                if let Err(err) = serde_json::from_str::<ActiveInstanceClone>(&val) {
                    println!("{}: {}", val, err);
                }
            }
        }
        instances
    }

    async fn setup(
        num_domains: u32,
        num_nodes_per_domain: u32,
        redis_url: Option<&str>,
    ) -> (Vec<futures::future::AbortHandle>, Box<(dyn WorkflowInstanceAPI)>) {
        assert!(num_domains > 0);
        assert!(num_nodes_per_domain > 0);

        let mut handles = vec![];

        let address = "127.0.0.1";
        let mut port = 7001;
        let controller_url = format!("http://{}:{}", address, port);

        // Closure automatically incrementing the port number
        let mut next_port = || {
            port += 1;
            port
        };

        let mut orchestrators = vec![];
        for domain_i in 0..num_domains {
            let orchestrator_url = format!("http://{}:{}", address, next_port());
            let domain_id = format!("domain-{}", domain_i);
            orchestrators.push(edgeless_con::EdgelessConOrcConfig {
                domain_id: domain_id.clone(),
                orchestrator_url: orchestrator_url.clone(),
            });

            let (task, handle) = futures::future::abortable(edgeless_orc::edgeless_orc_main(edgeless_orc::EdgelessOrcSettings {
                general: edgeless_orc::EdgelessOrcGeneralSettings {
                    domain_id: domain_id.to_string(),
                    orchestrator_url: orchestrator_url.to_string(),
                    orchestrator_url_announced: "".to_string(),
                    agent_url: format!("http://{}:{}", address, next_port()),
                    agent_url_announced: "".to_string(),
                    invocation_url: format!("http://{}:{}", address, next_port()),
                    invocation_url_announced: "".to_string(),
                },
                baseline: edgeless_orc::EdgelessOrcBaselineSettings {
                    orchestration_strategy: edgeless_orc::OrchestrationStrategy::RoundRobin,
                    keep_alive_interval_secs: 1,
                },
                proxy: match redis_url {
                    None => edgeless_orc::EdgelessOrcProxySettings {
                        proxy_type: "None".to_string(),
                        redis_url: None,
                    },
                    Some(url) => edgeless_orc::EdgelessOrcProxySettings {
                        proxy_type: "Redis".to_string(),
                        redis_url: Some(url.to_string()),
                    },
                },
                collector: match redis_url {
                    None => edgeless_orc::EdgelessOrcCollectorSettings {
                        collector_type: "None".to_string(),
                        redis_url: None,
                    },
                    Some(url) => edgeless_orc::EdgelessOrcCollectorSettings {
                        collector_type: "Redis".to_string(),
                        redis_url: Some(url.to_string()),
                    },
                },
            }));
            tokio::spawn(task);
            handles.push(handle);

            // The first node in each domain is also assigned a file-log resource.
            for node_i in 0..num_nodes_per_domain {
                let (task, handle) = futures::future::abortable(edgeless_node::edgeless_node_main(match node_i {
                    0 => edgeless_node::EdgelessNodeSettings {
                        general: edgeless_node::EdgelessNodeGeneralSettings {
                            node_id: uuid::Uuid::new_v4(),
                            agent_url: format!("http://{}:{}", address, next_port()),
                            agent_url_announced: "".to_string(),
                            invocation_url: format!("http://{}:{}", address, next_port()),
                            invocation_url_announced: "".to_string(),
                            metrics_url: format!("http://{}:{}", address, next_port()),
                            orchestrator_url: orchestrator_url.to_string(),
                        },
                        wasm_runtime: Some(edgeless_node::EdgelessNodeWasmRuntimeSettings { enabled: true }),
                        container_runtime: None,
                        resources: Some(edgeless_node::EdgelessNodeResourceSettings {
                            http_ingress_url: None,
                            http_ingress_provider: None,
                            http_egress_provider: None,
                            file_log_provider: Some("file-log-1".to_string()),
                            redis_provider: None,
                            dda_url: None,
                            dda_provider: None,
                        }),
                        user_node_capabilities: None,
                    },
                    _ => {
                        edgeless_node::EdgelessNodeSettings::new_without_resources(&orchestrator_url, address, next_port(), next_port(), next_port())
                    }
                }));
                tokio::spawn(task);
                handles.push(handle);
            }
        }

        let (task, handle) = futures::future::abortable(edgeless_con::edgeless_con_main(edgeless_con::EdgelessConSettings {
            controller_url: controller_url.clone(),
            orchestrators,
        }));
        tokio::spawn(task);
        handles.push(handle);

        let mut con_client = edgeless_api::grpc_impl::controller::ControllerAPIClient::new(controller_url.as_str()).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        (handles, con_client.workflow_instance_api())
    }

    async fn wf_list(client: &mut Box<(dyn WorkflowInstanceAPI)>) -> Vec<edgeless_api::workflow_instance::WorkflowInstance> {
        match client.list(edgeless_api::workflow_instance::WorkflowId::none()).await {
            Ok(instances) => instances,
            Err(_) => vec![],
        }
    }

    fn fixture_spec() -> edgeless_api::function_instance::FunctionClassSpecification {
        edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "system_test".to_string(),
            function_class_type: "RUST_WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_code: include_bytes!("fixtures/system_test.wasm").to_vec(),
            function_class_outputs: vec!["out1".to_string(), "out2".to_string(), "err".to_string(), "log".to_string()],
        }
    }

    fn terminate(handles: Vec<futures::future::AbortHandle>) -> anyhow::Result<()> {
        for handle in handles {
            handle.abort();
        }
        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn system_test_single_domain_single_node() -> anyhow::Result<()> {
        let _ = env_logger::try_init();

        // Create the EDGELESS system.
        let (handles, mut client) = setup(1, 1, None).await;

        assert!(wf_list(&mut client).await.is_empty());

        // Create 10 workflows
        let mut workflow_ids = vec![];
        for _ in 0..10 {
            let res = client
                .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                    workflow_functions: vec![edgeless_api::workflow_instance::WorkflowFunction {
                        name: "f1".to_string(),
                        function_class_specification: fixture_spec(),
                        output_mapping: std::collections::HashMap::new(),
                        annotations: std::collections::HashMap::new(),
                    }],
                    workflow_resources: vec![],
                    annotations: std::collections::HashMap::new(),
                })
                .await;
            workflow_ids.push(match res {
                Ok(response) => match &response {
                    edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(err) => {
                        panic!("workflow rejected: {}", err)
                    }
                    edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(val) => {
                        assert_eq!(1, val.domain_mapping.len());
                        assert_eq!("f1", val.domain_mapping[0].name);
                        assert_eq!("domain-0", val.domain_mapping[0].domain_id);
                        val.workflow_id.clone()
                    }
                },
                Err(err) => panic!("could not start the workflow: {}", err),
            });
        }

        assert_eq!(10, wf_list(&mut client).await.len());

        // Stop the workflows
        for workflow_id in workflow_ids {
            match client.stop(workflow_id).await {
                Ok(_) => {}
                Err(err) => panic!("could not stop the workflow: {}", err),
            }
        }

        // Stop a non-existing workflow
        match client
            .stop(edgeless_api::workflow_instance::WorkflowId {
                workflow_id: uuid::Uuid::new_v4(),
            })
            .await
        {
            Ok(_) => {}
            Err(err) => panic!("could not stop the workflow: {}", err),
        }
        assert!(wf_list(&mut client).await.is_empty());

        terminate(handles)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial_test::serial]
    async fn system_test_single_domain_three_nodes() -> anyhow::Result<()> {
        let _ = env_logger::try_init();

        // Create the EDGELESS system.
        let (handles, mut client) = setup(1, 3, None).await;

        assert!(wf_list(&mut client).await.is_empty());

        let num_workflows = 3;

        let removeme_filename = |workflow_i| format!("removeme-{}.log", workflow_i);

        let cleanup = || {
            for workflow_i in 0..num_workflows {
                let _ = std::fs::remove_file(removeme_filename(workflow_i));
            }
        };

        // Create workflows
        let mut workflow_ids = vec![];
        cleanup();
        for workflow_i in 0..num_workflows {
            let res = client
                .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                    workflow_functions: vec![
                        edgeless_api::workflow_instance::WorkflowFunction {
                            name: "f1".to_string(),
                            function_class_specification: fixture_spec(),
                            output_mapping: std::collections::HashMap::from([
                                ("out1".to_string(), "f2".to_string()),
                                ("out2".to_string(), "f3".to_string()),
                                ("log".to_string(), "log".to_string()),
                            ]),
                            annotations: std::collections::HashMap::from([("init-payload".to_string(), "8".to_string())]),
                        },
                        edgeless_api::workflow_instance::WorkflowFunction {
                            name: "f2".to_string(),
                            function_class_specification: fixture_spec(),
                            output_mapping: std::collections::HashMap::from([("log".to_string(), "log".to_string())]),
                            annotations: std::collections::HashMap::new(),
                        },
                        edgeless_api::workflow_instance::WorkflowFunction {
                            name: "f3".to_string(),
                            function_class_specification: fixture_spec(),
                            output_mapping: std::collections::HashMap::from([("log".to_string(), "log".to_string())]),
                            annotations: std::collections::HashMap::new(),
                        },
                    ],
                    workflow_resources: vec![edgeless_api::workflow_instance::WorkflowResource {
                        name: "log".to_string(),
                        class_type: "file-log".to_string(),
                        output_mapping: std::collections::HashMap::new(),
                        configurations: std::collections::HashMap::from([("filename".to_string(), removeme_filename(workflow_i))]),
                    }],
                    annotations: std::collections::HashMap::new(),
                })
                .await;
            workflow_ids.push(match res {
                Ok(response) => match &response {
                    edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(err) => {
                        panic!("workflow rejected: {}", err)
                    }
                    edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(val) => {
                        assert_eq!(4, val.domain_mapping.len());
                        for i in 0..4 {
                            match i {
                                3 => assert_eq!("log", val.domain_mapping[i].name),
                                _ => assert_eq!(format!("f{}", i + 1), val.domain_mapping[i].name),
                            };
                            assert_eq!("domain-0", val.domain_mapping[i].domain_id);
                        }
                        val.workflow_id.clone()
                    }
                },
                Err(err) => panic!("could not start the workflow: {}", err),
            });
        }

        assert_eq!(num_workflows, wf_list(&mut client).await.len());

        // Wait until the log files have been filled.
        let mut not_done_yet: std::collections::HashSet<usize> = std::collections::HashSet::from_iter(0..num_workflows);
        let values_expected = std::collections::HashSet::<i32>::from([4, 7, 8]);
        for _ in 0..100 {
            for workflow_i in 0..num_workflows {
                let values_from_file: std::collections::HashSet<i32> = std::fs::read_to_string(removeme_filename(workflow_i))
                    .expect("could not read file")
                    .split('\n')
                    .filter_map(|x| x.parse::<i32>().ok())
                    .collect();
                if values_from_file == values_expected {
                    not_done_yet.remove(&workflow_i);
                }
            }
            if not_done_yet.is_empty() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }
        assert!(not_done_yet.is_empty(), "not all logs have been filled properly");

        // Stop the workflows
        for workflow_id in workflow_ids {
            match client.stop(workflow_id).await {
                Ok(_) => {}
                Err(err) => panic!("could not stop the workflow: {}", err),
            }
        }
        assert!(wf_list(&mut client).await.is_empty());

        cleanup();
        terminate(handles)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    #[serial_test::serial]
    async fn system_test_orchestration_intent_migration_redis() -> anyhow::Result<()> {
        let _ = env_logger::try_init();

        // Skip the test if there is no local Redis listening on default port.
        let mut redis_connection = match redis::Client::open("redis://localhost:6379") {
            Ok(client) => match client.get_connection() {
                Ok(connection) => connection,
                Err(_) => {
                    println!("the test cannot be run because there is no Redis reachable on localhost at port 6379");
                    return Ok(());
                }
            },
            Err(_) => {
                println!("the test cannot be run because there is no Redis reachable on localhost at port 6379");
                return Ok(());
            }
        };

        // Create an EDGELESS system with a single domain and two nodes.
        let (handles, mut client) = setup(1, 2, Some("redis://127.0.0.1:6379")).await;

        // Check that in the Redis there are two regular nodes, in addition to
        // the one for metrics collection in the orchestrator.
        let node_uuids = redis_node_ids(&mut redis_connection);
        assert_eq!(node_uuids.len(), 1 + 2);

        // Check that there is no workflow through the client.
        assert!(wf_list(&mut client).await.is_empty());

        // Clean-up closures.
        let removeme_filename = || format!("removeme.log");
        let cleanup = || {
            let _ = std::fs::remove_file(removeme_filename());
        };

        // Create one workflow
        let workflow_id;
        cleanup();
        let res = client
            .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
                workflow_functions: vec![
                    edgeless_api::workflow_instance::WorkflowFunction {
                        name: "f1".to_string(),
                        function_class_specification: fixture_spec(),
                        output_mapping: std::collections::HashMap::from([
                            ("out1".to_string(), "f2".to_string()),
                            ("out2".to_string(), "f3".to_string()),
                        ]),
                        annotations: std::collections::HashMap::from([("init-payload".to_string(), "8".to_string())]),
                    },
                    edgeless_api::workflow_instance::WorkflowFunction {
                        name: "f2".to_string(),
                        function_class_specification: fixture_spec(),
                        output_mapping: std::collections::HashMap::from([("log".to_string(), "log".to_string())]),
                        annotations: std::collections::HashMap::new(),
                    },
                    edgeless_api::workflow_instance::WorkflowFunction {
                        name: "f3".to_string(),
                        function_class_specification: fixture_spec(),
                        output_mapping: std::collections::HashMap::from([("log".to_string(), "log".to_string())]),
                        annotations: std::collections::HashMap::new(),
                    },
                ],
                workflow_resources: vec![],
                annotations: std::collections::HashMap::new(),
            })
            .await;
        workflow_id = Some(match res {
            Ok(response) => match &response {
                edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(err) => {
                    panic!("workflow rejected: {}", err)
                }
                edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(val) => {
                    assert_eq!(3, val.domain_mapping.len());
                    for i in 0..3 {
                        assert_eq!(format!("f{}", i + 1), val.domain_mapping[i].name);
                        assert_eq!("domain-0", val.domain_mapping[i].domain_id);
                    }
                    val.workflow_id.clone()
                }
            },
            Err(err) => panic!("could not start the workflow: {}", err),
        });

        // Check that the client now shows one workflow.
        assert_eq!(1, wf_list(&mut client).await.len());

        // Find the logical identifiers of the functions and how the instances
        // were mapped to nodes.
        let mut instances = std::collections::HashMap::new();
        for _ in 0..100 {
            instances = redis_instances(&mut redis_connection);
            if instances.len() == 3 {
                break;
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        let mut instances_to_nodes = std::collections::HashMap::new();
        for (logical_fid, instance) in instances {
            println!("before function {} -> node {}", logical_fid, instance.node_id());
            instances_to_nodes.insert(logical_fid, instance.node_id());
        }

        // Find the nodes with assigned functions.
        let mut nodes_with_functions = std::collections::HashSet::new();
        for (_logical_fid, node_id) in &instances_to_nodes {
            nodes_with_functions.insert(node_id.clone());
        }
        assert_eq!(nodes_with_functions.len(), 2);

        // Add intents to migrate the function instances.
        let other = |x: &edgeless_api::function_instance::NodeId| {
            if let Some(val) = nodes_with_functions.iter().find(|y| x != *y) {
                *val
            } else {
                uuid::Uuid::nil()
            }
        };
        for (logical_fid, node_id) in &instances_to_nodes {
            let key = format!("intent:migrate:{}", logical_fid);
            let _ = redis_connection.set::<&str, &str, usize>(&key, &other(node_id).to_string());
            let _ = redis_connection.lpush::<&str, &str, String>("intents", &key);
        }

        // Wait until the policy is implemented.
        for _ in 0..100 {
            let new_instances = redis_instances(&mut redis_connection);
            assert_eq!(new_instances.len(), instances_to_nodes.len());

            let mut not_done = false;
            for (logical_fid, node_id) in &instances_to_nodes {
                if let Some(new_instance) = new_instances.get(logical_fid) {
                    let new_node = new_instance.node_id();
                    if new_node != other(node_id) {
                        not_done = true;
                        break;
                    }
                }
            }

            if !not_done {
                break;
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        // Print new mapping
        instances = redis_instances(&mut redis_connection);
        for (logical_fid, instance) in instances {
            println!("after function {} -> node {}", logical_fid, instance.node_id());
        }

        // Check that the intents have been cleared.
        assert!(redis_connection.keys::<&str, Vec<String>>("intents").unwrap().is_empty());
        assert!(redis_connection.keys::<&str, Vec<String>>("intent:*").unwrap().is_empty());

        // Stop the workflows
        match client.stop(workflow_id.unwrap()).await {
            Ok(_) => {}
            Err(err) => panic!("could not stop the workflow: {}", err),
        }
        assert!(wf_list(&mut client).await.is_empty());

        cleanup();
        terminate(handles)
    }
}
