#!/usr/bin/env python3

import pandas as pd
import os

filename = os.environ.get("DATASET", "")
if filename == "":
    raise RuntimeError("missing environment variable DATASET")

df = pd.read_csv(filename)
df = df[df["entity"] == "w"]
for wf_name in df["name"].unique():
    mean = df[df["name"] == wf_name]["value"].mean()
    print("the average latency of {} was {:.2f} ms".format(wf_name, mean))
