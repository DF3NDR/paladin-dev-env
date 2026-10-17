#!/bin/bash
# Post-create script for DevContainer
# Runs once after the container is created

set -e

echo "🚀 Running post-create setup for Paladin DevContainer..."

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Change to workspace directory
cd /workspace

# Install project dependencies
echo -e "${BLUE}📦 Installing Rust dependencies...${NC}"
cargo fetch || echo -e "${YELLOW}⚠️  Warning: cargo fetch failed, will retry during build${NC}"

# Build the project (this caches dependencies)
echo -e "${BLUE}🔨 Building project (this may take a while on first run)...${NC}"
cargo build || echo -e "${YELLOW}⚠️  Warning: initial build failed, check dependencies${NC}"

# Run database migrations if they exist
if [ -d "migrations" ]; then
    echo -e "${BLUE}🗄️  Checking database migrations...${NC}"
    # Check if we need to run migrations
    if command -v sqlx &> /dev/null; then
        echo "SQLx CLI is available for migrations"
    fi
fi

# Install GSD (Git. Ship. Done.) workflow tooling into ./.claude
# The install lives in the bind-mounted workspace, so it normally survives a
# container rebuild on its own. This guard restores it on a fresh clone, or if
# .claude/ has been cleared, and is a no-op when GSD is already present.
echo -e "${BLUE}⚙️  Checking GSD workflow tooling...${NC}"
if [ -f ".claude/gsd-file-manifest.json" ]; then
    echo -e "${GREEN}✅ GSD already installed (run 'npx @opengsd/gsd-core@latest --claude --local' to update)${NC}"
elif command -v npx >/dev/null 2>&1; then
    echo -e "${BLUE}📦 Installing GSD for Claude Code...${NC}"
    npx --yes @opengsd/gsd-core@latest --claude --local \
        || echo -e "${YELLOW}⚠️  GSD install failed; run it manually once the network is available.${NC}"
else
    echo -e "${YELLOW}⚠️  npx not found on PATH; skipping GSD install.${NC}"
    echo -e "${YELLOW}   Node.js 22+ is provisioned by .devcontainer/Dockerfile.dev — rebuild the container.${NC}"
fi

# Set up git hooks if .git exists
if [ -d ".git" ]; then
    echo -e "${BLUE}🔗 Setting up git hooks (pre-commit framework)...${NC}"

    if command -v pre-commit >/dev/null 2>&1; then
        # Install the version-controlled hook suite (commit + pre-push stages).
        pre-commit install --install-hooks
        pre-commit install --hook-type pre-push
        echo -e "${GREEN}✅ Git hooks installed via pre-commit${NC}"
    else
        echo -e "${YELLOW}⚠️  pre-commit not found on PATH; skipping hook install.${NC}"
        echo -e "${YELLOW}   Install it with 'pipx install pre-commit' then run 'make hooks'.${NC}"
    fi
fi

# Create useful aliases
echo -e "${BLUE}⚙️  Setting up aliases...${NC}"
cat >> ~/.bashrc << 'EOF'

# Paladin development aliases
alias pd-build='cargo build'
alias pd-test='cargo test'
alias pd-run='cargo run'
alias pd-check='cargo check'
alias pd-fmt='cargo fmt'
alias pd-clippy='cargo clippy -- -D warnings'
alias pd-clean='cargo clean'
alias pd-doc='cargo doc --open --no-deps'
alias pd-watch='cargo watch -x check -x test -x run'
alias pd-bench='cargo bench'
alias pd-audit='cargo audit'
alias pd-outdated='cargo outdated'
alias pd-bloat='cargo bloat --release'

# Make commands
alias pd-dev='make dev'
alias pd-services='make services-up'
alias pd-test-all='make test-all'
alias pd-clean-code='make clean-code'

# Quick navigation
alias pd-src='cd /workspace/src'
alias pd-tests='cd /workspace/tests'
alias pd-docs='cd /workspace/docs'
alias pd-examples='cd /workspace/examples'

EOF

# shellcheck source=/dev/null  # user's rc file; not resolvable at lint time
source ~/.bashrc 2>/dev/null || true

# Create .env if it doesn't exist
if [ ! -f ".env" ] && [ -f ".env.example" ]; then
    echo -e "${BLUE}📝 Creating .env file from .env.example...${NC}"
    cp .env.example .env
    echo -e "${YELLOW}⚠️  Please update .env with your configuration${NC}"
fi

# Check for required environment variables
echo -e "${BLUE}🔍 Checking environment...${NC}"
if [ ! -f ".env" ]; then
    echo -e "${YELLOW}⚠️  No .env file found. Create one from .env.example${NC}"
fi

# Display helpful information
echo ""
echo -e "${GREEN}✅ DevContainer setup complete!${NC}"
echo ""
echo -e "${BLUE}📚 Useful commands:${NC}"
echo "  pd-build      - Build the project"
echo "  pd-test       - Run tests"
echo "  pd-run        - Run the application"
echo "  pd-watch      - Watch mode (auto-rebuild on changes)"
echo "  pd-clippy     - Run clippy lints"
echo "  pd-fmt        - Format code"
echo "  pd-clean-code - Format, lint, and check"
echo "  pd-doc        - Generate and open documentation"
echo ""
echo -e "${BLUE}🔧 Development:${NC}"
echo "  pd-dev        - Start all services (Redis, RustFS, etc.)"
echo "  pd-services   - Start supporting services only"
echo "  pd-test-all   - Run all tests including integration"
echo ""
echo -e "${BLUE}📖 Documentation:${NC}"
echo "  docs/         - Project documentation"
echo "  examples/     - Example code"
echo ""
echo -e "${GREEN}Happy coding! 🎉${NC}"
echo ""
