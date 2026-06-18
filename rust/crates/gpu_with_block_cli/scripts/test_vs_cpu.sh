#!/bin/bash

TASK="$1"

cargo run --release --bin squishy_volumes_gpu_cli -- cpu "$TASK" --generate 1000000
cargo run --release --bin squishy_volumes_gpu_cli -- gpu "$TASK" 

diff "test_data/cpu-$TASK-out.bin" "test_data/gpu-$TASK-out.bin"
