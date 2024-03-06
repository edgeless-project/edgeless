#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

export MQTT_DDA="mqtt://localhost:1883"
echo "start a mock robot arm as actor ..."
cd ../mock-services/mock-robot-arm/
go run main.go