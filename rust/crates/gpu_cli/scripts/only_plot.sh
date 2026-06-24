#!/bin/bash

SCRIPT_DIR="$(dirname "$0")"

TASK="$1"

DATA_FILE="test_data/$TASK.txt"

gnuplot -e "data_file='$DATA_FILE'" "$SCRIPT_DIR/runtime_and_throughput.gplot"
