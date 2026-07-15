from tkinter import W
import argparse
import plotly.graph_objects as go
import csv
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("input")
parser.add_argument("output")
args = parser.parse_args()


labels = []
values = []
with open(args.input) as csvfile:
    reader = csv.reader(csvfile, delimiter=",")
    for row in reader:
        if not labels:
            labels = row
            for _ in range(len(labels)):
                values.append([])
            continue

        for i, value in enumerate(row):
            values[i].append(float(value))

assert labels[0] == "time"

fig = go.Figure()

for i in range((len(labels) - 1) // 2):
    start = i * 2 + 1
    end = i * 2 + 2
    fig.add_trace(
        go.Scatter(
            x=values[0],
            y=values[start],
            mode="lines",
            fill=None,
            name=labels[start],
            showlegend=False,
        )
    )
    fig.add_trace(
        go.Scatter(
            x=values[0], y=values[end], mode="lines", fill="tonexty", name=labels[start]
        )
    )

fig.show()
