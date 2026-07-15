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
            del labels[1::2]
            # drop the -end suffix
            labels = [labels[0]] + [l[0:-4] for l in labels[1:]]
            for _ in range(len(labels)):
                values.append([])
            continue

        values[0].append(float(row[0]))
        for i in range(len(labels) - 1):
            values[i + 1].append(float(row[i * 2 + 2]) - float(row[i * 2 + 1]))

assert labels[0] == "time"

fig = go.Figure()

for i in range(1, len(labels)):
    fig.add_trace(go.Bar(x=values[0], y=values[i], name=labels[i]))

fig.show()
