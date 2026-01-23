#!/bin/bash

for f in *.json; do
    echo "$f"
    cat "$f" | jq '.node_trees[].data | select(.name | test("\\.[0-9]{3}$")) | .name'
done