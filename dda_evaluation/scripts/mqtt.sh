#!/bin/bash
docker stop mqtt
docker rm mqtt
docker run --name mqtt \
  -p 1883:1883 \
  --network host \
  eclipse-mosquitto:latest \
  mosquitto -c /mosquitto-no-auth.conf