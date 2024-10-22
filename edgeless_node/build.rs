// compiles all of the dda protos on build
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Add flag enabling optional proto3 in tonic_build
    tonic_build::configure().protoc_arg("--experimental_allow_proto3_optional").compile(
        &[
            "src/resources/dda/proto/com.proto",
            "src/resources/dda/proto/state.proto",
            "src/resources/dda/proto/store.proto",
        ],
        &["src/resources/dda/proto"],
    )?;

    // recommended method for generating rust bindings for gRPC for DDA
    // tonic_build::compile_protos("src/resources/dda/proto/com.proto")?;
    // tonic_build::compile_protos("src/resources/dda/proto/state.proto")?;
    // tonic_build::compile_protos("src/resources/dda/proto/store.proto")?;
    Ok(())
}