// SPDX-FileCopyrightText: © 2024 Siemens AG
// SPDX-License-Identifier: MIT
// compiles all of the dda protos on build
fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new().compile_protos(
        &[
            "protos/state.proto",
            "protos/com.proto",
            "protos/store.proto",
        ],
        &["protos/"],
    )?;
    Ok(())
}
