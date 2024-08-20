Pinger = edgeless_actor_class(
    id = "pinger",
    version = "0.1",
    outputs = [cast_output("ping", "com.edgeless.Ping")],
    inputs = [cast_input("pong", "com.edgeless.Pong")],
    inner_structure = [source("ping"), sink("pong")],
    code = file("pinger_async.tar.gz"),
    code_type = "RUST"
)

el_main = Pinger