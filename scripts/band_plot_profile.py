from tkinter import W
import argparse
import plotly.graph_objects as go
import csv
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("input")
parser.add_argument("output")
args = parser.parse_args()


frames = []
labels = []
values = []
with open(args.input) as csvfile:
    reader = csv.reader(csvfile, delimiter=",")
    for row in reader:
        if not labels:
            labels = row[1:]
            for _ in range(len(labels)):
                values.append([])
            continue

        frames.append(int(row[0]))
        for i, value in enumerate(row[1:]):
            values[i].append(float(value))

fig = go.Figure()

for i in range(len(labels) // 2):
    start = i * 2
    end = i * 2 + 1
    fig.add_trace(
        go.Scatter(
            x=frames,
            y=values[start],
            mode="lines",
            fill=None,
            name=labels[start],
            showlegend=False,
        )
    )
    fig.add_trace(
        go.Scatter(
            x=frames, y=values[end], mode="lines", fill="tonexty", name=labels[start]
        )
    )

fig.show()
