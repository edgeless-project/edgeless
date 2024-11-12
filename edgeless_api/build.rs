// SPDX-FileCopyrightText: © 2023 Technical University of Munich, Chair of Connected Mobility
// SPDX-FileCopyrightText: © 2023 Claudio Cicconetti <c.cicconetti@iit.cnr.it>
// SPDX-FileCopyrightText: © 2023 Siemens AG
// SPDX-License-Identifier: MIT
fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc_impl")]
    {
        tonic_build::compile_protos("proto/services.proto")?;
    }
    Ok(())
}
