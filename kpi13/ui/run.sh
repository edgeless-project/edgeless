#!/bin/bash
# Script to run KPI13 UI with either CI-built images or local builds

set -e

MODE="${1:-local}"
REGISTRY="${CI_REGISTRY_IMAGE:-cr.siemens.com/iot2050/luke/edgeless-finalization}"
TAG="${2:-latest}"

case $MODE in
  "ci"|"registry")
    echo "Using CI-built images from registry: $REGISTRY"
    CI_REGISTRY_IMAGE="$REGISTRY" IMAGE_TAG="$TAG" docker-compose -f docker-compose.ci.yml up "$@"
    ;;
  "local"|"build")
    echo "Building and running local images"
    docker-compose up --build "$@"
    ;;
  "pull")
    echo "Pulling latest CI images..."
    CI_REGISTRY_IMAGE="$REGISTRY" IMAGE_TAG="$TAG" docker-compose -f docker-compose.ci.yml pull
    ;;
  *)
    echo "Usage: $0 [local|ci|pull] [tag]"
    echo "  local  - Build and run local images (default)"
    echo "  ci     - Use CI-built images from registry"
    echo "  pull   - Pull latest CI images"
    echo "  tag    - Image tag to use (default: latest)"
    exit 1
    ;;
esac