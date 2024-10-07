Ponger = edgeless_actor_class(
    id = "pong_async",
    version = "0.1",
    outputs = [cast_output("pong", "edgeless.example.Pong")],
    inputs = [cast_input("ping", "edgeless.example.Ping")],
    inner_structure = [link("ping", ["pong"])],
    code = file("pong_async.tar.gz"),
    code_type = "RUST"
)

el_main = Ponger