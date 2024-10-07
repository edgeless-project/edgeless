load("../../functions/ping_async/pinger.star", "Pinger")
load("../../functions/pong_async/ponger.star", "Ponger")

pinger = edgeless_actor(
    id = "pinger_i",
    klass = Pinger,
    annotations = {}
)

ponger = edgeless_actor(
    id = "ponger_i",
    klass = Ponger,
    annotations = {}
)

ponger2 = edgeless_actor(
    id = "ponger_i_2",
    klass = Ponger,
    annotations = {}
)

pinger.ping >> ponger.ping
ponger.pong >> pinger.pong

wf = edgeless_workflow(
    "ping_pong_async",
    [pinger, ponger, ponger2],
    annotations = {}
)

el_main = wf




# print(dir(pinger))

# rc = edgeless_resource_class(
#     id = "baz",
#     outputs = [],
#     inputs = [],
#     inner_structure = []
# )

# a = edgeless_actor(
#     id = "foo_instance",
#     k = ac,
# )

# b = edgeless_actor(
#     id = "bar_instance",
#     k = load_actor_class("foo", "0.1")
# )

# c = edgeless_resource(
#     id = "baz_instance",
#     r = load_resource("foo")
# )

# wf = edgeless_workflow(
#     a
# )

# wf

# # a.map_output_ping

# a.foo >> b.bar

# 