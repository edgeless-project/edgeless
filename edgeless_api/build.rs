// SPDX-FileCopyrightText: © 2023 TUM
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-License-Identifier: MIT
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc_impl")]
    {
        tonic_build::compile_protos("proto/services.proto")?;
    }
    Ok(())
}
