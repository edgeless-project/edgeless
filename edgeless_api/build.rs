// SPDX-FileCopyrightText: © 2025 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2025 Siemens AG
// SPDX-License-Identifier: MIT

use schemars::schema_for;
use std::fs::File;
use std::io::Write;

include!("src/function_instance_structs.rs");
include!("src/workflow_instance_structs.rs");

// https://stackoverflow.com/questions/67461445/cargo-rust-build-script-print-output-of-command
// for printing
macro_rules! p {
    ($($tokens: tt)*) => {
        println!("cargo:warning={}", format!($($tokens)*))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc_impl")]
    {
        tonic_build::compile_protos("proto/services.proto")?;
    }

    if std::fs::metadata("../schemas").is_err() {
        p!("schemas/ directory not available - skipping JSON schema generation - they will not be available in your IDE");
    } else {
        let workflow_schema = schema_for!(SpawnWorkflowRequest);
        let workflow_schema_json = serde_json::to_string_pretty(&workflow_schema).unwrap();
        let mut workflow_schema_file = File::create("../schemas/workflow_schema.json").unwrap();
        workflow_schema_file.write_all(workflow_schema_json.as_bytes()).unwrap();

        let function_schema = schema_for!(FunctionClassSpecification);
        let function_schema_json = serde_json::to_string_pretty(&function_schema).unwrap();
        let mut function_schema_file = File::create("../schemas/function_class_schema.json").unwrap();
        function_schema_file.write_all(function_schema_json.as_bytes()).unwrap()
    }

    Ok(())
}
