// compiles all of the dda protos on build
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // recommended method for generating rust bindings for gRPC for DDA
    tonic_build::compile_protos("src/resources/dda/proto/com.proto")?;
    tonic_build::compile_protos("src/resources/dda/proto/state.proto")?;
    tonic_build::compile_protos("src/resources/dda/proto/store.proto")?;
    Ok(())
}
