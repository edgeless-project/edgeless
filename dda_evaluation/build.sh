#!/bin/bash
cd ext
go build -C sensor
go build -C tester
go build -C actor
cd ..

RUST_LOG=INFO ../target/debug/edgeless_cli function build ../dda_evaluation/dda_call/function.json