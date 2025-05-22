#!/bin/bash
# SPDX-FileCopyrightText: © 2024 Siemens AG
# SPDX-License-Identifier: MIT
docker run -it -p 1883:1883 eclipse-mosquitto mosquitto -c /mosquitto-no-auth.conf