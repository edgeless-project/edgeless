// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT

use crate::function_instance::FunctionClassSpecification;
use std::str::FromStr;

include!("workflow_instance_structs.rs");

const WORKFLOW_ID_NONE: uuid::Uuid = uuid::uuid!("00000000-0000-0000-0000-ffff00000000");

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, PartialOrd, Ord)]
pub struct WorkflowId {
    pub workflow_id: uuid::Uuid,
}

impl WorkflowId {
    pub fn new(s: &str) -> anyhow::Result<Self> {
        Ok(Self {
            workflow_id: uuid::Uuid::from_str(s)?,
        })
    }
    pub fn from_string(s: &str) -> Self {
        Self {
            workflow_id: uuid::Uuid::from_str(s).unwrap(),
        }
    }
    pub fn none() -> Self {
        Self {
            workflow_id: WORKFLOW_ID_NONE,
        }
    }
    pub fn is_valid(&self) -> Option<&WorkflowId> {
        if self.workflow_id == WORKFLOW_ID_NONE { None } else { Some(self) }
    }
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.workflow_id)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct WorkflowFunctionMapping {
    pub name: String,
    pub function_id: crate::function_instance::ComponentId,
    pub domain_id: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct WorkflowInstance {
    pub workflow_id: WorkflowId,
    pub domain_mapping: Vec<WorkflowFunctionMapping>,
}

impl WorkflowInstance {
    /// Return the domain associated with a given function/resource, if any.
    pub fn domain(&self, component: &str) -> Option<String> {
        for domain_mapping in &self.domain_mapping {
            if domain_mapping.name == component {
                return Some(domain_mapping.domain_id.clone());
            }
        }
        None
    }
}

impl WorkflowResource {
    pub fn is_valid(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.name.is_empty(), "empty name in resource");
        anyhow::ensure!(!self.class_type.is_empty(), "empty class type in resource");
        anyhow::ensure!(
            !self
                .output_mapping
                .iter()
                .any(|(channel, component)| channel.is_empty() || component.is_empty()),
            "empty channel or component in output_mapping of a resource"
        );
        Ok(())
    }
}

impl WorkflowFunction {
    pub fn is_valid(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.name.is_empty(), "empty name in function");
        anyhow::ensure!(
            !self
                .output_mapping
                .iter()
                .any(|(channel, component)| channel.is_empty() || component.is_empty()),
            "empty channel or component in output_mapping of a function"
        );
        Ok(())
    }
}

impl SpawnWorkflowRequest {
    /// Return the union of all the names of the components mentioned
    /// by the workflow, as component to be either started or mapped to.
    pub fn all_component_names(&self) -> std::collections::HashSet<String> {
        let source_components = self.source_components();
        let mut mapped_components = self.mapped_components();
        mapped_components.extend(source_components);
        mapped_components
    }

    /// Return the names of the components that should be started.
    pub fn source_components(&self) -> std::collections::HashSet<String> {
        let mut ret: std::collections::HashSet<String> = self.functions.iter().map(|x| x.name.clone()).collect();
        ret.extend(self.resources.iter().map(|x| x.name.clone()));
        ret
    }

    /// Return the names of the components to which others map.
    pub fn mapped_components(&self) -> std::collections::HashSet<String> {
        let mut ret: std::collections::HashSet<String> = self.functions.iter().flat_map(|x| x.output_mapping.values()).cloned().collect();
        ret.extend(self.resources.iter().flat_map(|x| x.output_mapping.values()).cloned());
        ret
    }

    /// Retrieve the function with given component name, if any.
    pub fn get_function(&self, name: &str) -> Option<&WorkflowFunction> {
        self.functions.iter().find(|x| x.name == name)
    }

    /// Retrieve the resource with given component name, if any.
    pub fn get_resource(&self, name: &str) -> Option<&WorkflowResource> {
        self.resources.iter().find(|x| x.name == name)
    }

    /// Change the target for a given channel of a function/resource.
    ///
    /// Ignore if the function/resource, or channel mapping, does not exist.
    pub fn update_mapping(&mut self, name: &str, channel: &str, new_target: String) {
        if let Some(function) = self.functions.iter_mut().find(|x| x.name == *name) {
            if let Some(mapping) = function.output_mapping.get_mut(channel) {
                *mapping = new_target;
            }
        } else if let Some(resource) = self.resources.iter_mut().find(|x| x.name == *name) {
            if let Some(mapping) = resource.output_mapping.get_mut(channel) {
                *mapping = new_target;
            }
        }
    }

    /// Return the output mappings of all the components, both functions and
    /// resources.
    pub fn output_mappings(&self) -> std::collections::HashMap<String, std::collections::HashMap<String, String>> {
        let function_mappings: std::collections::HashMap<String, std::collections::HashMap<String, String>> = self
            .functions
            .iter()
            .map(|function| (function.name.clone(), function.output_mapping.clone()))
            .collect();
        let mut resource_mappings: std::collections::HashMap<String, std::collections::HashMap<String, String>> = self
            .resources
            .iter()
            .map(|resource| (resource.name.clone(), resource.output_mapping.clone()))
            .collect();
        resource_mappings.extend(function_mappings);
        resource_mappings
    }

    /// Check if the workflow is valid.
    pub fn is_valid(&self) -> anyhow::Result<()> {
        for function in &self.functions {
            function.is_valid()?;
        }
        for resource in &self.resources {
            resource.is_valid()?;
        }
        anyhow::ensure!(
            self.mapped_components()
                .difference(&self.source_components())
                .collect::<Vec<&String>>()
                .is_empty()
        );

        // self.workflow_functions.
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct WorkflowInfo {
    pub request: SpawnWorkflowRequest,
    pub status: WorkflowInstance,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub enum SpawnWorkflowResponse {
    ResponseError(crate::common::ResponseError),
    WorkflowInstance(WorkflowInstance),
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct MigrateWorkflowRequest {
    pub workflow_id: WorkflowId,
    pub domain_id: String,
    pub component: String,
}

#[async_trait::async_trait]
pub trait WorkflowInstanceAPI: WorkflowInstanceAPIClone + Send + Sync {
    async fn start(&mut self, request: SpawnWorkflowRequest) -> anyhow::Result<SpawnWorkflowResponse>;
    async fn stop(&mut self, id: WorkflowId) -> anyhow::Result<()>;
    async fn list(&mut self) -> anyhow::Result<Vec<WorkflowId>>;
    async fn inspect(&mut self, id: WorkflowId) -> anyhow::Result<WorkflowInfo>;
    async fn domains(
        &mut self,
        domain_id: String,
    ) -> anyhow::Result<std::collections::HashMap<String, crate::domain_registration::DomainCapabilities>>;
    async fn migrate(&mut self, request: MigrateWorkflowRequest) -> anyhow::Result<SpawnWorkflowResponse>;
}

// https://stackoverflow.com/a/30353928
pub trait WorkflowInstanceAPIClone {
    fn clone_box(&self) -> Box<dyn WorkflowInstanceAPI>;
}
impl<T> WorkflowInstanceAPIClone for T
where
    T: 'static + WorkflowInstanceAPI + Clone,
{
    fn clone_box(&self) -> Box<dyn WorkflowInstanceAPI> {
        Box::new(self.clone())
    }
}
impl Clone for Box<dyn WorkflowInstanceAPI> {
    fn clone(&self) -> Box<dyn WorkflowInstanceAPI> {
        self.clone_box()
    }
}

#[cfg(test)]
mod test {
    use crate::function_instance::FunctionClassSpecification;

    use super::*;

    #[test]
    fn test_spawn_workflow_request_empty() {
        let spec = SpawnWorkflowRequest {
            functions: vec![],
            resources: vec![],
            annotations: std::collections::HashMap::new(),
        };

        assert!(spec.all_component_names().is_empty());
        assert!(spec.mapped_components().is_empty());
        assert!(spec.source_components().is_empty());
        assert!(spec.is_valid().is_ok());
        assert!(spec.output_mappings().is_empty());
    }

    #[test]
    fn test_spawn_workflow_request_accessors() {
        let spec = SpawnWorkflowRequest {
            functions: vec![
                WorkflowFunction {
                    name: String::from("f1"),
                    class_specification: FunctionClassSpecification {
                        id: String::from("function-class-id"),
                        function_type: String::from("function-class-type"),
                        version: String::from("function-class-version"),
                        binary: Some("byte-code".to_string().as_bytes().to_vec()),
                        code: Some("code-location".to_string()),
                        outputs: vec![],
                    },
                    output_mapping: std::collections::HashMap::from([
                        (String::from("out1"), String::from("r1")),
                        (String::from("out2"), String::from("f2")),
                    ]),
                    annotations: std::collections::HashMap::new(),
                },
                WorkflowFunction {
                    name: String::from("f2"),
                    class_specification: FunctionClassSpecification {
                        id: String::from("function-class-id"),
                        function_type: String::from("function-class-type"),
                        version: String::from("function-class-version"),
                        binary: Some("byte-code".to_string().as_bytes().to_vec()),
                        code: Some("code-location".to_string()),
                        outputs: vec![],
                    },
                    output_mapping: std::collections::HashMap::from([
                        (String::from("out1"), String::from("f1")),
                        (String::from("out2"), String::from("r1")),
                    ]),
                    annotations: std::collections::HashMap::new(),
                },
                WorkflowFunction {
                    name: String::from("f3"),
                    class_specification: FunctionClassSpecification {
                        id: String::from("function-class-id"),
                        function_type: String::from("function-class-type"),
                        version: String::from("function-class-version"),
                        binary: Some("byte-code".to_string().as_bytes().to_vec()),
                        code: Some("code-location".to_string()),
                        outputs: vec![],
                    },
                    output_mapping: std::collections::HashMap::new(),
                    annotations: std::collections::HashMap::new(),
                },
            ],
            resources: vec![WorkflowResource {
                name: String::from("r1"),
                class_type: String::from("resource-class"),
                output_mapping: std::collections::HashMap::from([
                    (String::from("out1"), String::from("f2")),
                    (String::from("out2"), String::from("f1")),
                ]),
                configurations: std::collections::HashMap::new(),
            }],
            annotations: std::collections::HashMap::new(),
        };

        assert!(spec.is_valid().is_ok());

        assert_eq!(
            std::collections::HashSet::from([String::from("f1"), String::from("r1"), String::from("f2"), String::from("f3")]),
            spec.all_component_names()
        );
        assert_eq!(
            std::collections::HashSet::from([String::from("f1"), String::from("r1"), String::from("f2")]),
            spec.mapped_components()
        );
        assert_eq!(
            std::collections::HashSet::from([String::from("f1"), String::from("r1"), String::from("f2"), String::from("f3")]),
            spec.source_components()
        );
        assert_eq!(
            std::collections::HashMap::from([
                (
                    String::from("f1"),
                    std::collections::HashMap::from([(String::from("out1"), String::from("r1")), (String::from("out2"), String::from("f2"))])
                ),
                (
                    String::from("r1"),
                    std::collections::HashMap::from([(String::from("out1"), String::from("f2")), (String::from("out2"), String::from("f1"))])
                ),
                (
                    String::from("f2"),
                    std::collections::HashMap::from([(String::from("out1"), String::from("f1")), (String::from("out2"), String::from("r1"))])
                ),
                (String::from("f3"), std::collections::HashMap::new()),
            ]),
            spec.output_mappings()
        );
    }
}
