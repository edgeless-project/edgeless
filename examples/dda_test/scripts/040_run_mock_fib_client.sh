#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

export MQTT_DDA="mqtt://localhost:1883"
echo "start a mock client to the fib workflow..."
cd ../mock-services/mock-fib-client/
go run main.go