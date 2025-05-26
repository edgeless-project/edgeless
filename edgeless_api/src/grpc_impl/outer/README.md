# what is this

- it implements the traits defined in outer in edgeless_api for the gRPC binding
- these implementations are the ones directly used by edgeless components - all
  the internal detail should stay private -> encapsulation
- should be renamed to grpc_traits_impl or something

# TODO:
- invocation.rs implements both the server and client in the same file?