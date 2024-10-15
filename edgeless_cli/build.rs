use schemars::schema_for;
use std::fs::File;
use std::io::Write;

// hacky way of referencing: https://stackoverflow.com/questions/67905320/how-can-i-import-a-source-file-from-my-library-into-build-rs
mod workflow_spec {
    include!("src/workflow_spec.rs");
}

// https://stackoverflow.com/questions/67461445/cargo-rust-build-script-print-output-of-command
// for printing
macro_rules! p {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

fn main() {
    if std::fs::metadata("../schemas").is_err() {
        p!("schemas/ directory not available - skipping JSON schema generation - they will not be available in your IDE");
        return;
    }

    let workflow_schema = schema_for!(workflow_spec::WorkflowSpec);
    let workflow_schema_json = serde_json::to_string_pretty(&workflow_schema).unwrap();
    let mut workflow_schema_file = File::create("../schemas/workflow_schema.json").unwrap();
    workflow_schema_file.write_all(workflow_schema_json.as_bytes()).unwrap();

    let function_schema = schema_for!(workflow_spec::WorkflowSpecFunctionClass);
    let function_schema_json = serde_json::to_string_pretty(&function_schema).unwrap();
    let mut function_schema_file = File::create("../schemas/function_class_schema.json").unwrap();
    function_schema_file.write_all(function_schema_json.as_bytes()).unwrap()
}
