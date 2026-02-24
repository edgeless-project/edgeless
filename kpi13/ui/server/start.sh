#!/bin/bash

# Check if dependencies are installed
if [ ! -d "node_modules" ]; then
    echo "Dependencies not found. Run ./build.sh first."
    exit 1
fi

# Start the server
echo "Starting Redis WebSocket backend on port 3002..."
npx ts-node redis-ws-backend.ts