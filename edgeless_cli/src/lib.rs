// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT

use workflow_spec::WorkflowSpec;

pub mod workflow_spec;

pub fn workflow_spec_to_request(
    workflow_spec: WorkflowSpec,
    parent_path: &std::path::Path,
) -> anyhow::Result<edgeless_api::workflow_instance::SpawnWorkflowRequest> {
    // Check that all the RUST_WASM binaries are available.
    for function in &workflow_spec.functions {
        if function.class_specification.function_type == "RUST_WASM" {
            if let Some(code_path_str) = &function.class_specification.code {
                let code_path = parent_path.join(code_path_str);
                anyhow::ensure!(
                    code_path.exists(),
                    "code file does not exist for '{}': {}",
                    function.name,
                    code_path_str
                );
                anyhow::ensure!(
                    code_path.is_file(),
                    "code file for '{}' is not regular: {}",
                    function.name,
                    code_path_str
                );
            } else {
                anyhow::bail!(
                    "RUST_WASM function code not specified for '{}'",
                    function.name
                );
            }
        }
    }

    // Create the workflow specification.
    Ok(edgeless_api::workflow_instance::SpawnWorkflowRequest {
        workflow_functions: workflow_spec
            .functions
            .into_iter()
            .map(|func_spec| {
                let function_class_code = match func_spec.class_specification.function_type.as_str()
                {
                    "RUST_WASM" => {
                        std::fs::read(parent_path.join(func_spec.class_specification.code.unwrap()))
                            .unwrap()
                    }
                    "CONTAINER" => func_spec
                        .class_specification
                        .code
                        .unwrap()
                        .as_bytes()
                        .to_vec(),
                    _ => panic!(
                        "unknown function class type: {}",
                        func_spec.class_specification.function_type
                    ),
                };

                edgeless_api::workflow_instance::WorkflowFunction {
                    name: func_spec.name,
                    function_class_specification:
                        edgeless_api::function_instance::FunctionClassSpecification {
                            function_class_id: func_spec.class_specification.id,
                            function_class_type: func_spec.class_specification.function_type,
                            function_class_version: func_spec.class_specification.version,
                            function_class_code,
                            function_class_outputs: func_spec.class_specification.outputs,
                        },
                    output_mapping: func_spec.output_mapping,
                    annotations: func_spec.annotations,
                }
            })
            .collect(),
        workflow_resources: workflow_spec
            .resources
            .into_iter()
            .map(
                |res_spec| edgeless_api::workflow_instance::WorkflowResource {
                    name: res_spec.name,
                    class_type: res_spec.class_type,
                    output_mapping: res_spec.output_mapping,
                    configurations: res_spec.configurations,
                },
            )
            .collect(),
        annotations: workflow_spec.annotations.clone(),
    })
}
