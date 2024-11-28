// SPDX-FileCopyrightText: © 2024 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2024 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT

#[cfg(test)]
mod tests {
    // use super::*;

    use edgeless_api::outer::controller::ControllerAPI;
    use edgeless_api::workflow_instance::WorkflowInstanceAPI;
    use edgeless_orc::proxy::Proxy;

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

        let domain_registration_url = format!("http://{}:{}", address, next_port());

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
                    controller_url: controller_url.clone(),
                    subscription_refresh_interval_sec: 1,
                    domain_id: domain_id.to_string(),
                    orchestrator_url: orchestrator_url.to_string(),
                    orchestrator_url_announced: "".to_string(),
                    orchestrator_coap_url: None,
                    orchestrator_coap_url_announced: None,
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
                        dataset_settings: None,
                    },
                    Some(url) => edgeless_orc::EdgelessOrcProxySettings {
                        proxy_type: "Redis".to_string(),
                        redis_url: Some(url.to_string()),
                        dataset_settings: None,
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
                            invocation_url_coap: None,
                            invocation_url_announced_coap: None,
                            orchestrator_url: orchestrator_url.to_string(),
                        },
                        telemetry: edgeless_node::EdgelessNodeTelemetrySettings {
                            metrics_url: format!("http://{}:{}", address, next_port()),
                            log_level: None,
                            performance_samples: false,
                        },
                        wasm_runtime: Some(edgeless_node::EdgelessNodeWasmRuntimeSettings { enabled: true }),
                        container_runtime: None,
                        resources: Some(edgeless_node::EdgelessNodeResourceSettings {
                            http_ingress_url: None,
                            http_ingress_provider: None,
                            http_egress_provider: None,
                            file_log_provider: Some("file-log-1".to_string()),
                            redis_provider: None,
                            dda_provider: None,
                            ollama_provider: None,
                            kafka_egress_provider: None,
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
            domain_registration_url,
            orchestrators,
        }));
        tokio::spawn(task);
        handles.push(handle);

        let mut con_client = edgeless_api::grpc_impl::outer::controller::ControllerAPIClient::new(controller_url.as_str()).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        (handles, con_client.workflow_instance_api())
    }

    async fn wf_list(client: &mut Box<(dyn WorkflowInstanceAPI)>) -> Vec<edgeless_api::workflow_instance::WorkflowInstance> {
        (client.list(edgeless_api::workflow_instance::WorkflowId::none()).await).unwrap_or_default()
    }

    fn fixture_spec() -> edgeless_api::function_instance::FunctionClassSpecification {
        edgeless_api::function_instance::FunctionClassSpecification {
            function_class_id: "system_test".to_string(),
            function_class_type: "RUST_WASM".to_string(),
            function_class_version: "0.1".to_string(),
            function_class_code: include_bytes!("../../functions/system_test/system_test.wasm").to_vec(),
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
                        let mut expected_names = std::collections::HashSet::<String>::new();
                        let mut expected_domains = std::collections::HashSet::<String>::new();
                        let mut actual_names = std::collections::HashSet::<String>::new();
                        let mut actual_domains = std::collections::HashSet::<String>::new();

                        for i in 0..4 {
                            match i {
                                3 => expected_names.insert("log".to_string()),
                                _ => expected_names.insert(format!("f{}", i + 1)),
                            };
                            expected_domains.insert("domain-0".to_string());
                            actual_names.insert(val.domain_mapping[i].name.clone());
                            actual_domains.insert(val.domain_mapping[i].domain_id.clone());
                        }
                        assert_eq!(expected_names, actual_names);
                        assert_eq!(expected_domains, actual_domains);
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
        let mut redis_proxy = match edgeless_orc::proxy_redis::ProxyRedis::new("redis://localhost:6379", true, None) {
            Ok(redis_proxy) => redis_proxy,
            Err(_) => {
                println!("the test cannot be run because there is no Redis reachable on localhost at port 6379");
                return Ok(());
            }
        };

        // Create an EDGELESS system with a single domain and two nodes.
        let (handles, mut client) = setup(1, 2, Some("redis://127.0.0.1:6379")).await;

        // Check that in the Redis there are two regular nodes, in addition to
        // the one for metrics collection in the orchestrator.
        for (uuid, capabilities) in redis_proxy.fetch_node_capabilities() {
            println!("node {}, capabilities {}", uuid, capabilities);
        }
        for (uuid, health) in redis_proxy.fetch_node_health() {
            println!("node {}, health {}", uuid, health);
        }
        let node_uuids = redis_proxy
            .fetch_node_capabilities()
            .keys()
            .cloned()
            .collect::<Vec<edgeless_api::function_instance::NodeId>>();
        assert_eq!(node_uuids.len(), 1 + 2);
        assert_eq!(redis_proxy.fetch_node_health().len(), 1 + 2);

        // Check that there is no workflow through the client.
        assert!(wf_list(&mut client).await.is_empty());

        // Clean-up closures.
        let removeme_filename = || "removeme.log".to_string();
        let cleanup = || {
            let _ = std::fs::remove_file(removeme_filename());
        };

        // Create one workflow

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
                workflow_resources: vec![edgeless_api::workflow_instance::WorkflowResource {
                    name: "log".to_string(),
                    class_type: "file-log".to_string(),
                    output_mapping: std::collections::HashMap::new(),
                    configurations: std::collections::HashMap::from([("filename".to_string(), removeme_filename())]),
                }],
                annotations: std::collections::HashMap::new(),
            })
            .await;
        let expected_instance_names = std::collections::HashSet::from(["f1", "f2", "f3", "log"]);
        let workflow_id = match res {
            Ok(response) => match &response {
                edgeless_api::workflow_instance::SpawnWorkflowResponse::ResponseError(err) => {
                    panic!("workflow rejected: {}", err)
                }
                edgeless_api::workflow_instance::SpawnWorkflowResponse::WorkflowInstance(val) => {
                    assert_eq!(4, val.domain_mapping.len());
                    for i in 0..4 {
                        assert!(expected_instance_names.contains(val.domain_mapping[i].name.as_str()));
                        assert_eq!("domain-0", val.domain_mapping[i].domain_id);
                    }
                    val.workflow_id.clone()
                }
            },
            Err(err) => panic!("could not start the workflow: {}", err),
        };

        // Check that the client now shows one workflow.
        assert_eq!(1, wf_list(&mut client).await.len());

        // Logical function identifiers -> nodes.
        let mut instances = std::collections::HashMap::new();
        for _ in 0..100 {
            instances = redis_proxy.fetch_function_instances_to_nodes();
            if instances.len() == 3 {
                break;
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        // Find the nodes with assigned functions.
        let mut nodes_with_functions = std::collections::HashSet::new();
        for (logical_fid, nodes) in &instances {
            assert!(nodes.len() == 1);
            println!("before function {} -> node {}", logical_fid, nodes.first().unwrap());
            nodes_with_functions.insert(*nodes.first().unwrap());
        }

        // Add intents to migrate the function instances.
        let other = |x: &edgeless_api::function_instance::NodeId| {
            if let Some(val) = nodes_with_functions.iter().find(|y| x != *y) {
                *val
            } else {
                uuid::Uuid::nil()
            }
        };

        let mut intents = vec![];
        for (logical_fid, nodes) in &instances {
            assert!(nodes.len() == 1);
            intents.push(edgeless_orc::orchestrator::DeployIntent::Migrate(
                *logical_fid,
                vec![other(nodes.first().unwrap())],
            ));
        }
        redis_proxy.add_deploy_intents(intents);

        // Wait until the policy is implemented.
        for _ in 0..100 {
            let new_instances = redis_proxy.fetch_function_instances_to_nodes();
            assert_eq!(new_instances.len(), instances.len());

            let mut not_done = false;
            for (logical_fid, nodes) in &instances {
                if let Some(new_nodes) = new_instances.get(logical_fid) {
                    assert!(new_nodes.len() == 1);
                    let new_node = new_nodes.first().unwrap();
                    let node = nodes.first().unwrap();
                    if new_node != &other(node) {
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
        instances = redis_proxy.fetch_function_instances_to_nodes();
        for (logical_fid, nodes) in instances {
            assert!(nodes.len() == 1);
            println!("after function {} -> node {}", logical_fid, nodes.first().unwrap());
        }

        // Check that the intents have been cleared.
        assert!(redis_proxy.retrieve_deploy_intents().is_empty());

        // Stop the workflows
        match client.stop(workflow_id).await {
            Ok(_) => {}
            Err(err) => panic!("could not stop the workflow: {}", err),
        }
        assert!(wf_list(&mut client).await.is_empty());

        cleanup();
        terminate(handles)
    }
}
