#!/bin/bash
cd ..

# single function in a chain
RUST_LOG=info target/debug/edgeless_benchmark \
    --redis-url redis://127.0.0.1:6379 \
    -w "vector-mul-chain;dda_evaluation/workflows/vector_mul_chain_1.json" \
    --dataset-path "dda_evaluation/data/myexp-" \
    --additional-fields "1" \
    --additional-header "chain_length" \
    --lifetime 5 \
    --duration 30 \
    --interarrival 1 \
    --append

# 3 functions in a chain
RUST_LOG=info target/debug/edgeless_benchmark \
    --redis-url redis://127.0.0.1:6379 \
    -w "vector-mul-chain;dda_evaluation/workflows/vector_mul_chain_3.json" \
    --dataset-path "dda_evaluation/data/myexp-" \
    --additional-fields "3" \
    --additional-header "chain_length" \
    --lifetime 5 \
    --duration 30 \
    --interarrival 1 \
    --append

# 5 functions in a chain
RUST_LOG=info target/debug/edgeless_benchmark \
    --redis-url redis://127.0.0.1:6379 \
    -w "vector-mul-chain;dda_evaluation/workflows/vector_mul_chain_5.json" \
    --dataset-path "dda_evaluation/data/myexp-" \
    --additional-fields "5" \
    --additional-header "chain_length" \
    --lifetime 5 \
    --duration 30 \
    --interarrival 1 \
    --append

# 10 functions in a chain
RUST_LOG=info target/debug/edgeless_benchmark \
    --redis-url redis://127.0.0.1:6379 \
    -w "vector-mul-chain;dda_evaluation/workflows/vector_mul_chain_10.json" \
    --dataset-path "dda_evaluation/data/myexp-" \
    --additional-fields "10" \
    --additional-header "chain_length" \
    --lifetime 5 \
    --duration 30 \
    --interarrival 1 \
    --append
