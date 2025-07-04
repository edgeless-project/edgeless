#!/usr/bin/env python3


filename = os.environ.get("DATASET", "")
if filename == "":
    raise RuntimeError("missing environment variable DATASET")

df = pd.read_csv(filename)
