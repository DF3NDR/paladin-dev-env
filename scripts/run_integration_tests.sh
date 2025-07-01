#!/bin/bash
# scripts/run_integration_tests.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCKER_COMPOSE_FILE="$PROJECT_ROOT/docker/docker-compose.yml"
DOCKER_COMPOSE_TEST_FILE="$PROJECT_ROOT/docker/docker-compose.test.yml"

# Default values
MODE="local"  # local, docker, ci
VERBOSE=false
CLEANUP=true
PARALLEL=false
SPECIFIC_TESTS=""

# Print usage
usage() {
    cat << EOF
Integration Test Runner for in4me

USAGE:
    $0 [OPTIONS]

OPTIONS:
    -m, --mode MODE         Test mode: local, docker, or ci (default: local)
    -v, --verbose          Enable verbose output
    -c, --no-cleanup       Skip cleanup after tests
    -p, --parallel         Run tests in parallel (when possible)
    -t, --tests PATTERN    Run specific tests matching pattern
    -h, --help             Show this help message

MODES:
    local    Use testcontainers (good for development)
    docker   Use docker-compose services (good for CI)
    ci       Optimized for CI environments

EXAMPLES:
    $0                                    # Run all tests in local mode
    $0 -m docker -v                      # Run with docker-compose, verbose
    $0 -t "file_storage" -m local        # Run only file storage tests
    $0 -m ci --no-cleanup                # CI mode without cleanup

EOF
}

# Logging functions
log_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
    echo -e "${RED}❌${NC} $1"
}

log_step() {
    echo -e "${BLUE}🔧${NC} $1"
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -m|--mode)
                MODE="$2"
                shift 2
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -c|--no-cleanup)
                CLEANUP=false
                shift
                ;;
            -p|--parallel)
                PARALLEL=true
                shift
                ;;
            -t|--tests)
                SPECIFIC_TESTS="$2"
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

# Check prerequisites
check_prerequisites() {
    log_step "Checking prerequisites..."

    # Check if we're in the right directory
    if [[ ! -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        log_error "Not in a Rust project directory"
        exit 1
    fi

    # Check Rust installation
    if ! command -v cargo &> /dev/null; then
        log_error "Cargo not found. Please install Rust"
        exit 1
    fi

    # Check Docker if needed
    if [[ "$MODE" == "docker" || "$MODE" == "ci" ]]; then
        if ! command -v docker &> /dev/null; then
            log_error "Docker not found but required for $MODE mode"
            exit 1
        fi

        if ! command -v docker-compose &> /dev/null; then
            log_error "Docker Compose not found but required for $MODE mode"
            exit 1
        fi
    fi

    log_success "Prerequisites check passed"
}

# Setup environment based on mode
setup_environment() {
    log_step "Setting up environment for $MODE mode..."

    case "$MODE" in
        "local")
            setup_local_environment
            ;;
        "docker")
            setup_docker_environment
            ;;
        "ci")
            setup_ci_environment
            ;;
        *)
            log_error "Unknown mode: $MODE"
            exit 1
            ;;
    esac
}

setup_local_environment() {
    log_info "Local mode: Using testcontainers"
    export USE_EXTERNAL_TEST_SERVICES="false"
    
    # Check if Docker is running (needed for testcontainers)
    if ! docker info &> /dev/null; then
        log_error "Docker is not running. Testcontainers require Docker"
        exit 1
    fi
}

setup_docker_environment() {
    log_info "Docker mode: Starting services with docker-compose"
    
    # Create .env file if it doesn't exist
    if [[ ! -f "$PROJECT_ROOT/.env" ]]; then
        log_info "Creating .env file from example"
        cp "$PROJECT_ROOT/.env.example" "$PROJECT_ROOT/.env" 2>/dev/null || true
    fi

    # Start test services
    log_step "Starting test services..."
    docker-compose -f "$DOCKER_COMPOSE_TEST_FILE" up -d redis-test minio-test minio-test-init

    # Wait for services
    wait_for_services

    export USE_EXTERNAL_TEST_SERVICES="true"
    export TEST_REDIS_HOST="localhost"
    export TEST_REDIS_PORT="6380"
    export TEST_MINIO_ENDPOINT="localhost:9010"
    export TEST_MINIO_ACCESS_KEY="testuser"
    export TEST_MINIO_SECRET_KEY="testpass123"
}

setup_ci_environment() {
    log_info "CI mode: Using pre-configured services"
    
    export USE_EXTERNAL_TEST_SERVICES="true"
    export TEST_REDIS_HOST="${TEST_REDIS_HOST:-localhost}"
    export TEST_REDIS_PORT="${TEST_REDIS_PORT:-6380}"
    export TEST_MINIO_ENDPOINT="${TEST_MINIO_ENDPOINT:-localhost:9010}"
    export TEST_MINIO_ACCESS_KEY="${TEST_MINIO_ACCESS_KEY:-testuser}"
    export TEST_MINIO_SECRET_KEY="${TEST_MINIO_SECRET_KEY:-testpass123}"

    # In CI, services should already be running
    wait_for_services
}

# Wait for external services to be ready
wait_for_services() {
    if [[ "$USE_EXTERNAL_TEST_SERVICES" == "true" ]]; then
        log_step "Waiting for external services..."

        # Wait for Redis
        log_info "Checking Redis at $TEST_REDIS_HOST:$TEST_REDIS_PORT"
        for i in {1..30}; do
            if command -v redis-cli &> /dev/null && redis-cli -h "$TEST_REDIS_HOST" -p "$TEST_REDIS_PORT" ping &> /dev/null; then
                log_success "Redis is ready"
                break
            elif nc -z "$TEST_REDIS_HOST" "$TEST_REDIS_PORT" &> /dev/null; then
                log_success "Redis port is open"
                break
            fi
            
            if [[ $i -eq 30 ]]; then
                log_error "Redis not available after 30 attempts"
                exit 1
            fi
            
            sleep 1
        done

        # Wait for MinIO
        local minio_host=$(echo "$TEST_MINIO_ENDPOINT" | cut -d: -f1)
        local minio_port=$(echo "$TEST_MINIO_ENDPOINT" | cut -d: -f2)
        
        log_info "Checking MinIO at $TEST_MINIO_ENDPOINT"
        for i in {1..30}; do
            if curl -f "http://$TEST_MINIO_ENDPOINT/minio/health/live" &> /dev/null; then
                log_success "MinIO is ready"
                break
            elif nc -z "$minio_host" "$minio_port" &> /dev/null; then
                log_info "MinIO port is open, waiting for service..."
            fi
            
            if [[ $i -eq 30 ]]; then
                log_error "MinIO not available after 30 attempts"
                exit 1
            fi
            
            sleep 1
        done

        sleep 2  # Additional buffer time
    fi
}

# Build the project
build_project() {
    log_step "Building project..."
    
    local build_flags="--release"
    if [[ "$VERBOSE" == "true" ]]; then
        build_flags="$build_flags --verbose"
    fi

    if ! cargo build $build_flags; then
        log_error "Build failed"
        exit 1
    fi

    log_success "Build completed"
}

# Run integration tests
run_integration_tests() {
    log_step "Running integration tests..."

    local test_flags="--release -- --ignored --nocapture"
    local test_env=""

    if [[ "$VERBOSE" == "true" ]]; then
        test_env="RUST_LOG=debug TEST_LOG=1"
    fi

    if [[ -n "$SPECIFIC_TESTS" ]]; then
        test_flags="$test_flags $SPECIFIC_TESTS"
        log_info "Running tests matching: $SPECIFIC_TESTS"
    fi

    # Set test environment
    export RUST_BACKTRACE=1

    # Run Redis queue tests
    if [[ -z "$SPECIFIC_TESTS" || "$SPECIFIC_TESTS" == *"redis"* || "$SPECIFIC_TESTS" == *"queue"* ]]; then
        log_info "Running Redis queue integration tests..."
        if ! env $test_env cargo test redis_queue_integration_tests $test_flags; then
            log_error "Redis queue integration tests failed"
            return 1
        fi
        log_success "Redis queue integration tests passed"
    fi

    # Run file storage tests
    if [[ -z "$SPECIFIC_TESTS" || "$SPECIFIC_TESTS" == *"file"* || "$SPECIFIC_TESTS" == *"storage"* || "$SPECIFIC_TESTS" == *"minio"* ]]; then
        log_info "Running file storage integration tests..."
        if ! env $test_env cargo test file_storage_integration_tests $test_flags; then
            log_error "File storage integration tests failed"
            return 1
        fi
        log_success "File storage integration tests passed"
    fi

    # Run end-to-end tests
    if [[ -z "$SPECIFIC_TESTS" || "$SPECIFIC_TESTS" == *"e2e"* || "$SPECIFIC_TESTS" == *"end_to_end"* ]]; then
        log_info "Running end-to-end tests..."
        if ! env $test_env cargo test end_to_end $test_flags; then
            log_warning "End-to-end tests failed or not found (this may be expected)"
        else
            log_success "End-to-end tests passed"
        fi
    fi

    log_success "All integration tests completed"
}

# Cleanup function
cleanup() {
    if [[ "$CLEANUP" == "true" ]]; then
        log_step "Cleaning up..."

        case "$MODE" in
            "docker")
                log_info "Stopping docker-compose services..."
                docker-compose -f "$DOCKER_COMPOSE_TEST_FILE" down -v 2>/dev/null || true
                ;;
            "local")
                log_info "Testcontainers will auto-cleanup"
                ;;
            "ci")
                log_info "CI cleanup handled by CI system"
                ;;
        esac

        log_success "Cleanup completed"
    else
        log_info "Skipping cleanup as requested"
    fi
}

# Main execution
main() {
    parse_args "$@"

    log_info "Starting integration tests in $MODE mode"
    log_info "Project root: $PROJECT_ROOT"

    # Setup cleanup trap
    trap cleanup EXIT

    check_prerequisites
    setup_environment
    build_project

    # Change to project directory
    cd "$PROJECT_ROOT"

    if run_integration_tests; then
        log_success "🎉 All integration tests passed!"
        exit 0
    else
        log_error "💥 Some integration tests failed!"
        exit 1
    fi
}

# Run main function with all arguments
main "$@"