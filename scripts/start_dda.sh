#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT

# start dda and broker
echo "Install dda via go..."
go install github.com/coatyio/dda/cmd/dda@latest

echo "Start a dda (note: make sure a MQTT broker is running)..."
dda -c ./scripts/dda.yaml