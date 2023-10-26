#!/bin/bash
# Based on https://github.com/embassy-rs/embassy/tree/main/examples/std

sudo ip tuntap add name tap0 mode tap user edgeless
sudo ip addr add 192.168.101.2/24 dev tap0
sudo ip link set tap0 up