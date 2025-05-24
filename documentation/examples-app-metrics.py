#!/usr/bin/env python3

import os
from sys import float_info

MAPPING = os.environ.get("MAPPING", "")
SAMPLES = os.environ.get("SAMPLES", "")

basename = os.path.basename(os.getcwd())

pid_to_wid = {}
with open(MAPPING, "r") as infile:
    for line in infile:
        (_lid, wid, _node_id, pid) = line.rstrip().split(",")[-4:]
        pid_to_wid[pid] = wid


timestamps = {}
min_timestamp = float_info.max
with open(SAMPLES, "r") as infile:
    for line in infile:
        (pid, metric, timestamp, value) = line.rstrip().split(",")[-4:]
        if metric != "tbegin" and metric != "tend":
            continue
        timestamp = float(timestamp)
        min_timestamp = min(timestamp, min_timestamp)
        msg_id = int(value)

        assert pid in pid_to_wid, f"unknown PID: {pid}"

        wid = pid_to_wid[pid]

        if wid not in timestamps:
            timestamps[wid] = {}
        if msg_id not in timestamps[wid]:
            timestamps[wid][msg_id] = [None, None]

        if metric == "tbegin":
            if timestamps[wid][msg_id][0] is None:
                timestamps[wid][msg_id][0] = timestamp
            else:
                print(
                    "dup timestamp of tbegin at {} for msg_id {} wid {}".format(
                        timestamp, msg_id, wid
                    )
                )
        elif metric == "tend":
            if timestamps[wid][msg_id][1] is None:
                timestamps[wid][msg_id][1] = timestamp
            else:
                print(
                    "dup timestamp of tend at {} for msg_id {} wid {}".format(
                        timestamp, msg_id, wid
                    )
                )

latencies = {}
losses = []
for wid, timestamps in timestamps.items():
    delivered = 0
    lost = 0
    for _msg_id, ids in timestamps.items():
        tbegin = ids[0] - min_timestamp if ids[0] is not None else None
        tend = ids[1] - min_timestamp if ids[1] is not None else None

        if tbegin is not None and tend is not None:
            if wid not in latencies:
                latencies[wid] = []
            latencies[wid].append(tend - tbegin)
            delivered += 1
        else:
            lost += 1
    if (delivered + lost) > 0:
        losses.append([wid, float(lost) / (delivered + lost)])

print("losses:")
for wid, loss_ratio in losses:
    print("{}: {}".format(wid, loss_ratio))

print("latencies (mean, in ms):")
for wid, values in latencies.items():
    print("{}: {}".format(wid, sum(values) / len(values) * 1000))
