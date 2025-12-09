#!/bin/bash
ROOT_DIR=../..

# Parse -d flag
DELETE_MODE=false
while getopts ":d" opt; do
  case $opt in
    d)
      DELETE_MODE=true
      ;;
    \?)
      echo "Invalid option: -$OPTARG" >&2
      exit 1
      ;;
  esac
done

file1="$ROOT_DIR/functions/work_splitter/work_splitter.wasm"
file2="$ROOT_DIR/functions/calculator/calculator.wasm"
# file3="$ROOT_DIR/functions/http_read_parameters/http_read_parameters.wasm"

if $DELETE_MODE; then
  echo "Deleting .wasm files..."
  rm -f "$file1" "$file2" "$file3"
  echo "Files deleted (if they existed)."
fi

$ROOT_DIR/target/debug/edgeless_cli function build $ROOT_DIR/functions/work_splitter/function.json
$ROOT_DIR/target/debug/edgeless_cli function build $ROOT_DIR/functions/calculator/function.json
# $ROOT_DIR/target/debug/edgeless_cli function build $ROOT_DIR/functions/http_read_parameters/function.json

if [[ -f "$ROOT_DIR/functions/work_splitter/work_splitter.wasm" && -f "$ROOT_DIR/functions/calculator/calculator.wasm" ]]; then
    echo "All wasm files compiled."
else
    echo "One or more files are missing."
fi