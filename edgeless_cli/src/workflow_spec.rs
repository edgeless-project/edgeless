#[derive(Debug, serde::Deserialize)]
pub struct WorkflowSpecFunctionClass {
    pub id: String,
    pub function_type: String,
    pub version: String,
    pub include_code_file: Option<String>,
    pub output_callbacks: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WorflowSpecFunction {
    pub alias: String,
    pub class_specification: WorkflowSpecFunctionClass,
    pub output_callback_definitions: std::collections::HashMap<String, String>,
    pub annotations: std::collections::HashMap<String, String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct WorkflowSpec {
    pub alias: String,
    pub functions: Vec<WorflowSpecFunction>,
    pub annotations: std::collections::HashMap<String, String>,
}
