#!/bin/bash
cd ..
RUST_LOG=info target/debug/edgeless_benchmark \
    --redis-url redis://127.0.0.1:6379 \
    -w "json-spec;dda_evaluation/workflows/dda_workflow_single.json" \
    --dataset-path "dda_evaluation/data/dda-exp-" \
    --lifetime 5 \
    --duration 30 \
    --interarrival 1 \
    --append

# RUST_LOG=info target/debug/edgeless_benchmark \
#     --redis-url redis://127.0.0.1:6379 \
#     -w "json-spec;dda_evaluation/workflows/dda_workflow_single_2.json" \
#     --dataset-path "dda_evaluation/data/dda-exp-" \
#     --lifetime 5 \
#     --duration 30 \
#     --interarrival 1 \
#     --append

# RUST_LOG=info target/debug/edgeless_benchmark \
#     --redis-url redis://127.0.0.1:6379 \
#     -w "json-spec;dda_evaluation/workflows/dda_workflow_single_3.json" \
#     --dataset-path "dda_evaluation/data/dda-exp-" \
#     --lifetime 5 \
#     --duration 30 \
#     --interarrival 1 \
#     --append
