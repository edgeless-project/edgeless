#!/bin/bash
docker stop dda
docker rm dda
docker run --name dda \
  -v ./dda_cfg/:/dda/ \
  -p 12000:12000 \
  --network host \
  ghcr.io/coatyio/dda:latest