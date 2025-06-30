#!/bin/bash

echo "Running System Log Adapter Integration Tests..."
echo "=============================================="

# Set up environment variables for testing
export RUST_LOG=debug
export SYSTEM_LOG_LEVEL=debug
export SYSTEM_LOG_FORMAT=text
export SYSTEM_LOG_ENABLE_CONSOLE=false
export SYSTEM_LOG_ENABLE_FILE=true
export RUST_BACKTRACE=0

# # Run the integration tests
# cargo test --test system_log_integration_test -- --nocapture

# echo ""
# echo "Integration tests completed."
# echo "Check the output above for any file system operations and log contents."

set -e

# echo "Starting integration tests..."

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "Error: Docker is not running. Please start Docker first."
    exit 1
fi

# Pull required images first
echo "Pulling required Docker images..."
docker pull minio/minio:latest
docker pull redis:7-alpine

# Run integration tests with proper isolation
echo "Running file storage integration tests..."
cargo test file_storage_integration_tests --test integration -- --test-threads=1 --nocapture

echo "Integration tests completed successfully!"