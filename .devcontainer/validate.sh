#!/bin/bash
# Validation script to test DevContainer setup
# Run this inside the DevContainer to verify everything works

set -e

echo "🧪 Paladin DevContainer Validation"
echo "=================================="
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Test function
test_command() {
    local name="$1"
    local command="$2"

    echo -n "Testing $name... "
    if eval "$command" &> /dev/null; then
        echo -e "${GREEN}✅ PASS${NC}"
        # Use $((...)) assignment, not ((TESTS_PASSED++)): the post-increment form
        # evaluates to 0 on the first call, which is a non-zero *exit status* and
        # aborts the whole script under `set -e`.
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}❌ FAIL${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi

    # Always succeed: a failed check is reported via TESTS_FAILED and the summary,
    # and must not terminate the run via `set -e`.
    return 0
}

echo -e "${BLUE}1. Rust Toolchain${NC}"
echo "-------------------"
test_command "rustc" "rustc --version"
test_command "cargo" "cargo --version"
test_command "rustfmt" "rustfmt --version"
test_command "clippy" "cargo clippy --version"
test_command "rust-analyzer" "rust-analyzer --version"
echo ""

echo -e "${BLUE}2. Cargo Tools${NC}"
echo "-------------------"
test_command "cargo-watch" "cargo watch --version"
test_command "cargo-nextest" "cargo nextest --version"
test_command "cargo-audit" "cargo audit --version"
test_command "cargo-edit" "cargo set-version --help"
test_command "cargo-expand" "cargo expand --version"
test_command "sqlx-cli" "sqlx --version"
echo ""

echo -e "${BLUE}3. System Tools${NC}"
echo "-------------------"
test_command "git" "git --version"
test_command "docker" "docker --version"
test_command "jq" "jq --version"
test_command "ripgrep" "rg --version"
test_command "fd" "fd --version"
test_command "bat" "bat --version"
test_command "exa" "exa --version"
echo ""

echo -e "${BLUE}3b. Node.js / GSD Tooling${NC}"
echo "-------------------"
test_command "node (>= 22)" "node --version && [ \"\$(node -p 'process.versions.node.split(\".\")[0]')\" -ge 22 ]"
test_command "npm" "npm --version"
test_command "npx" "npx --version"
test_command "GSD installed" "test -f /workspace/.claude/gsd-file-manifest.json"
echo ""

echo -e "${BLUE}4. Database Clients${NC}"
echo "-------------------"
test_command "sqlite3" "sqlite3 --version"
test_command "mysql" "mysql --version"
test_command "redis-cli" "redis-cli --version"
echo ""

echo -e "${BLUE}5. Project Structure${NC}"
echo "-------------------"
test_command "Cargo.toml" "test -f /workspace/Cargo.toml"
test_command "src/ directory" "test -d /workspace/src"
test_command "tests/ directory" "test -d /workspace/tests"
test_command ".env file" "test -f /workspace/.env || test -f /workspace/.env.example"
echo ""

echo -e "${BLUE}6. Cargo Commands${NC}"
echo "-------------------"
cd /workspace || exit 1
test_command "cargo check" "cargo check --quiet"
test_command "cargo fmt --check" "cargo fmt --check"
echo ""

echo -e "${BLUE}7. Service Connectivity${NC}"
echo "-------------------"
if command -v nc &> /dev/null; then
    # Only test if services are expected to be running
    if docker ps | grep -q redis; then
        test_command "Redis connection" "nc -z redis 6379"
    else
        echo -e "${YELLOW}⏭️  Redis not running (skipped)${NC}"
    fi

    if docker ps | grep -q rustfs; then
        test_command "RustFS connection" "nc -z rustfs 9000"
    else
        echo -e "${YELLOW}⏭️  RustFS not running (skipped)${NC}"
    fi

    if docker ps | grep -q mysql; then
        test_command "MySQL connection" "nc -z mysql 3306"
    else
        echo -e "${YELLOW}⏭️  MySQL not running (skipped)${NC}"
    fi
else
    echo -e "${YELLOW}⏭️  netcat not available (service checks skipped)${NC}"
fi
echo ""

echo -e "${BLUE}8. Aliases${NC}"
echo "-------------------"
test_command "pd-build alias" "type pd-build"
test_command "pd-test alias" "type pd-test"
test_command "pd-run alias" "type pd-run"
echo ""

# Summary
echo "=================================="
echo -e "${BLUE}Test Summary${NC}"
echo "=================================="
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✨ All tests passed! DevContainer is ready to use.${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Some tests failed. Check the output above.${NC}"
    echo -e "${YELLOW}   This may be normal if services aren't running.${NC}"
    exit 0  # Don't fail the script, just warn
fi
