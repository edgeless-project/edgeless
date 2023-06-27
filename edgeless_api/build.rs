fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc_impl")]
    {
        tonic_build::compile_protos("proto/agent_api.proto")?;
    }
    Ok(())
}
