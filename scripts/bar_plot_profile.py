# SPDX-License-Identifier: AGPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

import argparse
import csv

import plotly.graph_objects as go

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
