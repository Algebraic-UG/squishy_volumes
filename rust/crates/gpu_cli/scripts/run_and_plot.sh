#!/bin/bash

set -o pipefail

SCRIPT_DIR="$(dirname "$0")"

TASK="$1"

DATA_FILE="test_data/$TASK.txt"

> "$DATA_FILE"

for ((i=500000; i<=6000000; i+=500000)); do
    t=$(cargo run --release --bin squishy_volumes_gpu_cli -- \
        "$TASK" --generate $i \
        | grep XXX | sed -E 's/.*XXX: ([0-9]+)/\1/')
    if [[ $? -ne 0 ]]
    then
        echo "Looks like something went wrong with i=$i"
        break
    fi
    echo "$i $t" >> "$DATA_FILE"
done

"$SCRIPT_DIR/only_plot.sh" "$TASK"
