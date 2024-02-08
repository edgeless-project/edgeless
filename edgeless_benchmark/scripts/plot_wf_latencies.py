#!/usr/bin/env python3

import pandas as pd
import matplotlib.pyplot as plt


df = pd.read_csv('out.csv')

fig, ax = plt.subplots()

for workflow in df[df["entity"] == "W"]["name"].unique():
    values = df[df["entity"] == "W"][df["name"] == workflow]["value"].tolist()
    ax.plot(range(len(values)), sorted(values), label=workflow)

ax.legend()

plt.show(block=True)
