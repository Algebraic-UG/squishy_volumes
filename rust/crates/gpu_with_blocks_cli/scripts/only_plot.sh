#!/bin/bash

SCRIPT_DIR="$(dirname "$0")"

MODE="$1"
TASK="$2"

DATA_FILE="test_data/$MODE-$TASK.txt"

gnuplot -e "data_file='$DATA_FILE'" "$SCRIPT_DIR/runtime_and_throughput.gplot"
