Pinger = edgeless_actor_class(
    id = "ping_async",
    version = "0.1",
    outputs = [cast_output("ping", "edgeless.example.Ping")],
    inputs = [cast_input("pong", "edgeless.example.Pong")],
    inner_structure = [source("ping"), sink("pong")],
    code = file("ping_async.tar.gz"),
    code_type = "RUST"
)

el_main = Pinger