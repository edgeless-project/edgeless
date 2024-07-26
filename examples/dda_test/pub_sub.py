import json

# struct DDAComSubscription {
#     topic: String,   // also known as type in DDA
#
#     pattern: String, // action / event / query / state / membership -> types of dda events that can be subscribed to
#
#     method: String,  // cast or call as means of passing the dda event
#     target: String,  // which function should be invoked (or alias of another resource)
# }

# struct DDAComPublication {
#     topic: String,   // also known as type in DDA
#
#     pattern: String, // action / event/ query / input -> types of dda events
#     that can be sent out
#
#     id: String,      // used to identify a publication mapping
# }


def sub(topic, pattern, method, target):
    self = dict()
    self["topic"] = topic
    self["pattern"] = pattern
    self["method"] = method
    self["target"] = target
    return self


def pub(topic, pattern, id):
    self = dict()
    self["topic"] = topic
    self["pattern"] = pattern
    self["id"] = id
    return self


def output(s, p):
    print()
    subs_str = json.dumps(s, ensure_ascii=True).replace('"', '\\"')
    pubs_str = json.dumps(p, ensure_ascii=True).replace('"', '\\"')
    print(f'"dda_com_subscription_mapping": "{subs_str}",')
    print(f'"dda_com_publication_mapping": "{pubs_str}"')


subs = []
pubs = []

# dda_test
subs.append(sub("com.dda.event", "event", "cast", "dda_com_test"))
subs.append(sub("com.dda.action", "action", "call", "dda_com_test"))
# TODO: input does not need topic! -> for now it gets ignored
# FIXME: move to a separate config?? when we move to json for config this won't
# matter anymore anyways
subs.append(sub("", "input", "cast", "dda_state_store_test"))
subs.append(sub("", "membership", "call", "dda_state_store_test"))
pubs.append(pub("com.pub.event", "event", "eve"))
pubs.append(pub("com.pub.action", "action", "act"))
output(subs, pubs)


# dda_demo
subs2 = []
pubs2 = []

subs2.append(sub("com.edgeless.temperature", "event", "cast", "check_temperature"))
subs2.append(sub("com.edgeless.someddatopic", "action", "cast", "some_functioncall"))
pubs2.append(pub("action", "com.edgeless.moveRobotArm", "dda_move_arm"))
output(subs2, pubs2)
