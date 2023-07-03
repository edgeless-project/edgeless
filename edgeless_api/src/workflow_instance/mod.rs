use crate::function_instance::FunctionId;

#[derive(Debug, Clone)]
pub struct WorkflowId {
    pub workflow_id: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct WorkflowFunctionMapping {
    pub function_alias: String,
    pub instances: Vec<FunctionId>,
}

#[derive(Debug, Clone)]
pub struct WorkflowInstance {
    pub workflow_id: WorkflowId,
    pub functions: Vec<WorkflowFunctionMapping>,
}

#[derive(Clone)]
pub struct WorkflowFunction {
    pub function_alias: String,
    pub function_class_specification: crate::function_instance::FunctionClassSpecification,
    pub output_callback_definitions: std::collections::HashMap<String, String>,
    pub return_continuation: String,
    pub function_annotations: std::collections::HashMap<String, String>,
}

#[derive(Clone)]
pub struct SpawnWorkflowRequest {
    pub workflow_id: WorkflowId,
    pub workflow_functions: Vec<WorkflowFunction>,
    pub workflow_annotations: std::collections::HashMap<String, String>,
}

#[async_trait::async_trait]
pub trait WorkflowInstanceAPI: WorkflowInstanceAPIClone + Send + Sync {
    async fn start_workflow_instance(&mut self, request: SpawnWorkflowRequest) -> anyhow::Result<WorkflowInstance>;
    async fn stop_workflow_instance(&mut self, id: WorkflowId) -> anyhow::Result<()>;
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
