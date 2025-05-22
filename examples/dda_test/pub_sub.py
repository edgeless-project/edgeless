import json

#
# Use this simple Python script to create configurations for dda for
# workflow.json file
#
# struct DDAComSubscription {
#     topic: String,   // also known as type in DDA
#     pattern: String, // action / event / query / state / membership -> types of dda events that can be subscribed to
#     method: String,  // cast or call as means of passing the dda event
#     target: String,  // which function should be invoked (or alias of another resource)
# }

# struct DDAComPublication {
#     topic: String,   // also known as type in DDA
#     pattern: String, // action / event/ query / input
#     id: String,      // used to identify a publication mapping
# }


def sub(topic, pattern, method, target):
    self = dict()
    self["topic"] = topic
    self["pattern"] = pattern
    self["method"] = method
    self["target"] = target
    return self


def pub(topic, pattern, alias):
    self = dict()
    self["topic"] = topic
    self["pattern"] = pattern
    self["alias"] = alias
    return self


def output(s, p):
    print()
    subs_str = json.dumps(s, ensure_ascii=True).replace('"', '\\"')
    pubs_str = json.dumps(p, ensure_ascii=True).replace('"', '\\"')
    print(f'"dda_com_subscription_mapping": "{subs_str}",')
    print(f'"dda_com_publication_mapping": "{pubs_str}"')


#
#
# dda_test - testing all the functionality
#
#
# dda_com_test
# we call the same function with different incoming events and test the whole
# functionality
subs = []
pubs = []
subs.append(sub("com.sub.event", "event", "cast", "dda_com_test"))
subs.append(sub("com.sub.action", "action", "cast", "dda_com_test"))
subs.append(sub("com.sub.query", "query", "cast", "dda_com_test"))

pubs.append(pub("com.pub.event", "event", "event_alias"))
pubs.append(pub("com.pub.action", "action", "action_alias"))
pubs.append(pub("com.pub.query", "query", "query_alias"))

# dda_state_test
subs.append(sub("com.dda.input", "input", "cast", "dda_state_test"))
subs.append(sub("com.dda.membership", "membership", "cast", "dda_state_test"))

# dda_store_test
# NOTE: store APIs are used to modify the inmemory store of the DDA -> they do
# not have subscribe methods
output(subs, pubs)

#
#
# dda_demo - demo with the robotic arm
#
#
subs2 = []
pubs2 = []

subs2.append(sub("com.edgeless.temperature", "event", "cast", "check_temperature"))
pubs2.append(pub("com.edgeless.moveRobotArm", "action", "move_arm"))
output(subs2, pubs2)
