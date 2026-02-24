#!/bin/bash

# Local build test for React app

# Check dependencies
if ! command -v node &> /dev/null; then
    echo "Error: Node.js not found"
    exit 1
fi

if ! command -v npm &> /dev/null; then
    echo "Error: npm not found"
    exit 1
fi

# Install and build
npm install

if [ $? -ne 0 ]; then
    echo "Error: Failed to install dependencies"
    exit 1
fi

npm run build

if [ $? -ne 0 ]; then
    echo "Error: Failed to build"
    exit 1
fi

echo "Build completed. Run npm run start to start services."
