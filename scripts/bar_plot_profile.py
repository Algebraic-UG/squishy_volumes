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
            del labels[1::2]
            for _ in range(len(labels)):
                values.append([])
            continue

        frames.append(int(row[0]))
        for i in range(len(labels)):
            values[i].append(float(row[i * 2 + 2]) - float(row[i * 2 + 1]))

fig = go.Figure()

for i in range(len(labels)):
    fig.add_trace(go.Bar(x=frames, y=values[i], name=labels[i]))

fig.show()
