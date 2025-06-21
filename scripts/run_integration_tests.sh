#!/bin/bash

echo "Running System Log Adapter Integration Tests..."
echo "=============================================="

# Set up environment variables for testing
export RUST_LOG=debug
export SYSTEM_LOG_LEVEL=debug
export SYSTEM_LOG_FORMAT=text
export SYSTEM_LOG_ENABLE_CONSOLE=false
export SYSTEM_LOG_ENABLE_FILE=true

# Run the integration tests
cargo test --test system_log_integration_test -- --nocapture

echo ""
echo "Integration tests completed."
echo "Check the output above for any file system operations and log contents."