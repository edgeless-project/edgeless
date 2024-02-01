// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

#[cfg(test)]
mod tests {
    // use super::*;

    use edgeless_api::controller::ControllerAPI;
    use edgeless_api::workflow_instance::WorkflowInstanceAPI;

    async fn setup(
        num_domains: u32,
        num_nodes_per_domain: u32,
    ) -> (tokio::runtime::Runtime, Vec<tokio::task::JoinHandle<()>>, Box<(dyn WorkflowInstanceAPI)>) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(8)
            .enable_all()
            .build()
            .unwrap();

        let mut tasks = vec![];

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

            tasks.push(runtime.spawn(edgeless_orc::edgeless_orc_main(edgeless_orc::EdgelessOrcSettings {
                domain_id: domain_id.to_string(),
                orchestrator_url: orchestrator_url.to_string(),
                orchestration_strategy: edgeless_orc::OrchestrationStrategy::Random,
                keep_alive_interval_secs: 1,
            })));

            for _ in 0..num_nodes_per_domain {
                tasks.push(runtime.spawn(edgeless_node::edgeless_node_main(
                    edgeless_node::EdgelessNodeSettings::new_without_resources(&orchestrator_url, address, next_port(), next_port(), next_port()),
                )));
            }
        }

        tasks.push(runtime.spawn(edgeless_con::edgeless_con_main(edgeless_con::EdgelessConSettings {
            controller_url: controller_url.clone(),
            orchestrators,
        })));

        let mut con_client = edgeless_api::grpc_impl::controller::ControllerAPIClient::new(controller_url.as_str()).await;

        (runtime, tasks, con_client.workflow_instance_api())
    }

    async fn wf_list(client: &mut Box<(dyn WorkflowInstanceAPI)>) -> Vec<edgeless_api::workflow_instance::WorkflowInstance> {
        match client.list(edgeless_api::workflow_instance::WorkflowId::none()).await {
            Ok(instances) => instances,
            Err(_) => vec![],
        }
    }

    #[tokio::test]
    async fn test_single_domain_single_node() -> anyhow::Result<()> {
        env_logger::init();

        let (runtime, tasks, mut client) = setup(1, 1).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert!(wf_list(&mut client).await.is_empty());

        // let workflow: workflow_spec::WorkflowSpec = serde_json::from_str(&std::fs::read_to_string(spec_file.clone()).unwrap()).unwrap();
        // let res = con_wf_client
        //     .start(edgeless_api::workflow_instance::SpawnWorkflowRequest {
        //         workflow_functions: workflow
        //             .functions
        //             .into_iter()
        //             .map(|func_spec| {
        //                 let p = std::path::Path::new(&spec_file)
        //                     .parent()
        //                     .unwrap()
        //                     .join(func_spec.class_specification.include_code_file.unwrap());
        //                 edgeless_api::workflow_instance::WorkflowFunction {
        //                     name: func_spec.name,
        //                     function_class_specification: edgeless_api::function_instance::FunctionClassSpecification {
        //                         function_class_id: func_spec.class_specification.id,
        //                         function_class_type: func_spec.class_specification.function_type,
        //                         function_class_version: func_spec.class_specification.version,
        //                         function_class_inlude_code: std::fs::read(p).unwrap(),
        //                         outputs: func_spec.class_specification.outputs,
        //                     },
        //                     output_mapping: func_spec.output_mapping,
        //                     annotations: func_spec.annotations,
        //                 }
        //             })
        //             .collect(),
        //         workflow_resources: workflow
        //             .resources
        //             .into_iter()
        //             .map(|res_spec| edgeless_api::workflow_instance::WorkflowResource {
        //                 name: res_spec.name,
        //                 class_type: res_spec.class_type,
        //                 output_mapping: res_spec.output_mapping,
        //                 configurations: res_spec.configurations,
        //             })
        //             .collect(),
        //         annotations: workflow.annotations.clone(),
        //     })
        //     .await;
        // match res {
        //     Ok(response) => {
        //         match &response {
        //             SpawnWorkflowResponse::ResponseError(err) => {
        //                 println!("{:?}", err);
        //             }
        //             SpawnWorkflowResponse::WorkflowInstance(val) => {
        //                 println!("{}", val.workflow_id.workflow_id.to_string());
        //             }
        //         }
        //         log::info!("{:?}", response)
        //     }
        //     Err(err) => println!("{}", err),
        // };

        // runtime.block_on(async { futures::future::join_all(tasks).await });

        Ok(())
    }
}
