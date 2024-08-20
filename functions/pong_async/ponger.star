Ponger = edgeless_actor_class(
    id = "ponger",
    version = "0.1",
    outputs = [cast_output("pong", "com.edgeless.Pong")],
    inputs = [cast_input("ping", "com.edgeless.Ping")],
    inner_structure = [link("ping", ["pong"])],
    code = file("ponger_async.tar.gz"),
    code_type = "RUST"
)

el_main = Ponger