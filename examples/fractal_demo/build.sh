#!/bin/bash
ROOT_DIR=../..

$ROOT_DIR/target/debug/edgeless_cli function build $ROOT_DIR/functions/work_splitter/function.json
$ROOT_DIR/target/debug/edgeless_cli function build $ROOT_DIR/functions/calculator/function.json
$ROOT_DIR/target/debug/edgeless_cli function build $ROOT_DIR/functions/http_read_parameters/function.json