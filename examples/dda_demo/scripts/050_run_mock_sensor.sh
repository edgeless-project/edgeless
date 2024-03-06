#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

export MQTT_DDA="mqtt://localhost:1883"
echo "start a mock temperature sensor ..."
cd ../mock-services/mock-temperature-sensor/
go run main.go