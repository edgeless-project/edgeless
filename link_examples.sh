#!/bin/bash

wasm-tools component new target/wasm32-unknown-unknown/debug/edgeless_sample_pong.wasm -o examples/ping_pong/pong/pong.wasm
wasm-tools component new target/wasm32-unknown-unknown/debug/edgeless_sample_ping.wasm -o examples/ping_pong/ping/ping.wasm