#!/bin/bash

# Usage: ./bench.sh <output_directory> [chain_length]
# Example: ./bench.sh /path/to/results 5

if [ $# -lt 1 ]; then
    echo "Usage: $0 <output_directory> [chain_length]"
    echo "Example: $0 /path/to/results 5"
    exit 1
fi

OUTPUT_DIR="$1"
CHAIN_LENGTH="${2:-0}"  # Default to 0 if not specified

mkdir -p "$OUTPUT_DIR"

cd ..

cp workflows/dda_workflow_chain.json workflows/dda_workflow_chain.json.bak

# Update chain_length in the workflow file
sed -i "s/\"chain_length\": [0-9]*/\"chain_length\": $CHAIN_LENGTH/" workflows/dda_workflow_chain.json

echo "Running benchmark with chain_length=$CHAIN_LENGTH, output to $OUTPUT_DIR"

# Run the benchmark
RUST_LOG=info binaries/edgeless_benchmark \
    --controller-url http://localhost:7001 \
    -w "dda-chain;workflows/dda_workflow_chain.json" \
    --output "./data/benchmark_output.csv" \
    --lifetime 10 \
    --duration 30 \
    --interarrival 1 \
    --seed 12345

pwd
if [ -f "data/dda_application_logs.csv" ]; then
    cp data/dda_application_logs.csv "scripts/${OUTPUT_DIR}"
    mv "scripts/${OUTPUT_DIR}/dda_application_logs.csv" "scripts/${OUTPUT_DIR}/${CHAIN_LENGTH}.csv"
    echo "Copied dda_application_logs.csv to $OUTPUT_DIR and renamed it!"
else
    echo "Warning: data/application_logs.csv not found"
fi

rm -f data/*.csv
echo "Cleared all CSV files from data directory"

mv workflows/dda_workflow_chain.json.bak workflows/dda_workflow_chain.json
echo "Restored original workflow configuration"