#!/bin/bash

SCRIPT_DIR="$(dirname "$0")"

MODE="$1"
TASK="$2"

DATA_FILE="test_data/$MODE-$TASK.txt"

> "$DATA_FILE"

for ((i=500000; i<=10000000; i+=500000)); do
    echo -n "$i " >> "$DATA_FILE"
    cargo run --release --bin squishy_volumes_gpu_cli -- \
        "$MODE" "$TASK" --generate $i \
        | grep XXX | sed -E 's/.*XXX: ([0-9]+)/\1/' >> "$DATA_FILE"
done

"$SCRIPT_DIR/only_plot.sh" "$MODE" "$TASK"
