#!/usr/bin/env bash
# SPDX-FileCopyrightText: © 2026 Technical University of Crete
# SPDX-License-Identifier: MIT

#
# How to use:
#   export SCONE_HUB_ACCESS_TOKEN=<your_gitlab_access_token>
#   export REPO=<repository_name>   # e.g. "cli", "services", "crosscompilers"
#   ./list-images.sh
#
# This script:
#   1) Checks required environment variables
#   2) Finds the GitLab project ID for the given repo
#   3) Lists available Docker image tags in the registry
#

set -euo pipefail

# 1) Check required environment variables
if [[ -z "${SCONE_HUB_ACCESS_TOKEN:-}" ]]; then
  echo "Error: SCONE_HUB_ACCESS_TOKEN is not set"
  exit 1
fi

if [[ -z "${REPO:-}" ]]; then
  echo "Error: REPO is not set"
  exit 1
fi

# 2) Query GitLab for the project ID
PROJECT_ID=$(
  curl -s \
    -H "PRIVATE-TOKEN: $SCONE_HUB_ACCESS_TOKEN" \
    "https://gitlab.scontain.com/api/v4/projects?search=$REPO" \
  | jq -r '.[0].id'
)

if [[ -z "$PROJECT_ID" || "$PROJECT_ID" == "null" ]]; then
  echo "Error: Could not find project ID for repo '$REPO'"
  exit 1
fi

export PROJECT_ID
echo "Found PROJECT_ID=$PROJECT_ID for repo '$REPO'"

# 3) List image tags in the repository
echo "Available image tags:"
curl -s \
  -H "PRIVATE-TOKEN: $SCONE_HUB_ACCESS_TOKEN" \
  "https://gitlab.scontain.com/api/v4/projects/$PROJECT_ID/registry/repositories?tags=1" \
| jq -r '.[].tags[].name'
