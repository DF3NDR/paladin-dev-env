# DevContainer Files Reference

Complete list of all DevContainer configuration files created.

## Configuration Files

### `.devcontainer/devcontainer.json` (5.9K)
Main VS Code DevContainer configuration file.

**Contains:**
- Container build settings
- Port forwarding configuration
- VS Code extensions to install
- Editor settings and preferences
- Environment variables
- Lifecycle scripts
- Volume mounts
- Features (Docker-in-Docker, Git, GitHub CLI)

**Key Features:**
- 25+ pre-configured VS Code extensions
- Optimized Rust settings
- Auto-format on save
- Port forwarding for services
- Cargo and target directory caching

### `.devcontainer/Dockerfile.dev` (3.4K)
Development Docker image with complete toolchain.

**Includes:**
- Rust 1.93 with all components
- 20+ cargo tools (watch, nextest, audit, etc.)
- Modern CLI tools (bat, exa, ripgrep, fd)
- Debugging tools (lldb, gdb, valgrind)
- Database clients (sqlite3, mysql, redis)
- Build tools and dependencies
- Non-root user configuration

**Optimizations:**
- Multi-layer caching
- Cargo home configured
- All CPU cores available
- Incremental compilation enabled

### `.devcontainer/docker-compose.yml` (2.1K)
Service orchestration configuration.

**Services:**
- **paladin-dev**: Main development container
- **redis**: Redis 7 for queue management
- **rustfs**: S3-compatible object storage  
- **mysql**: MySQL 8.0 database

**Features:**
- Health checks on all services
- Shared network for communication
- Persistent volumes
- Environment variables configured
- Development optimizations
- Host bind mounts: read-only `~/.config/paladin` LLM credentials, and read-write
  `~/.claude-paladin` Claude Code session state (with `CLAUDE_CONFIG_DIR` set to
  match)

### `.devcontainer/Dockerfile` (2.0K)
Production runtime image (maintained from original).

**Purpose:**
- Minimal production deployment
- Multi-stage build
- Security-focused
- Small image size

## Automation Scripts

### `.devcontainer/post-create.sh` (3.8K) 🔧
Runs once after container creation.

**Actions:**
- Fetches Rust dependencies
- Builds project initially
- Sets up git pre-commit hooks
- Configures shell aliases
- Creates .env if missing
- Displays helpful information

**Aliases Added:**
- `pd-*` commands for common tasks
- Quick navigation aliases
- Make command shortcuts

### `.devcontainer/post-start.sh` (1.9K) 🚀
Runs every time container starts.

**Actions:**
- Shows git status
- Displays system information
- Quick health check
- Claude Code state mount check (active / absent / non-writable, with host-side fix)
- Helpful reminders

### `.devcontainer/setup-network.sh` (535 bytes) 🌐
Creates Docker network if needed.

**Purpose:**
- Ensures `paladin-dev-network` exists
- Enables service communication
- Idempotent (safe to run multiple times)

### `.devcontainer/validate.sh` (4.0K) ✅
Comprehensive environment validation.

**Checks:**
1. Rust toolchain (rustc, cargo, rustfmt, clippy)
2. Cargo tools (20+ tools)
3. System tools (git, docker, jq, etc.)
4. Database clients
5. Project structure
6. Build capability
7. Service connectivity
8. Aliases configuration

**Output:**
- Color-coded pass/fail
- Test summary
- Helpful error messages

## Documentation

### `.devcontainer/README.md` (7.4K) 📖
Complete DevContainer documentation.

**Sections:**
- Features overview
- Quick start guide
- Usage instructions
- Configuration details
- Service endpoints
- Testing procedures
- Debugging guide
- Troubleshooting
- Customization
- Advanced usage

### `.devcontainer/QUICKSTART.md` (6.4K) ⚡
Quick reference for daily use.

**Contents:**
- Common commands
- Useful aliases
- Debugging tips
- Service URLs
- Cargo command reference
- Troubleshooting quick fixes
- VS Code shortcuts
- Performance tips
- Next steps

### `.devcontainer/CI-CD.md` (13K) 🔄
CI/CD integration guide.

**Covers:**
- GitHub Actions (3 options)
- GitLab CI configuration
- Jenkins pipelines
- CircleCI setup
- Optimization strategies
- Security scanning
- Deployment pipelines
- Troubleshooting

### `.devcontainer/SETUP_COMPLETE.md` (7.4K) ✅
Setup completion summary.

**Contents:**
- What was created
- How to use
- Service access
- Validation checklist
- Next steps
- Resources

## VS Code Configuration

### `.vscode/launch.json` (7.0K) 🐛
Enhanced debugging configurations.

**Configurations:**
- Debug Paladin (main binary)
- Debug CLI tools
- Debug unit tests
- Debug integration tests
- Debug current test
- Debug repository tests
- Debug functional tests
- Debug with different options

**Features:**
- Full LLDB support
- Breakpoint debugging
- Environment variables set
- Source maps configured
- Filter configurations

### `.vscode/tasks.json` (5.8K) ⚙️
Build and run tasks.

**Tasks:**
- cargo build (default)
- cargo build --release
- cargo test (default)
- cargo test --nocapture
- cargo check
- cargo clippy
- cargo fmt
- cargo clean
- cargo run
- cargo watch (background)
- Start/stop services
- Make targets

**Features:**
- Keyboard shortcuts
- Problem matchers
- Background tasks
- Panel management

### `.vscode/settings.json` (341 bytes) ⚙️
Editor settings (existing, maintained).

**Settings:**
- Editor/formatting preferences
- Auto-approval for common commands

## Additional Files

### `Makefile` (Enhanced)
Added DevContainer targets:

**New Targets:**
- `make devcontainer-build` - Build image
- `make devcontainer-validate` - Validate setup
- `make devcontainer-network` - Create network
- `make devcontainer-services` - Start services
- `make devcontainer-services-down` - Stop services
- `make devcontainer-push` - Push to registry

### `DEVCONTAINER_READY.md` (Root) 🎉
Comprehensive setup completion guide.

**Purpose:**
- Quick start guide
- Features summary
- Command reference
- Validation checklist
- Benefits overview
- Next actions
- Support information

## File Organization

```
.devcontainer/
├── Configuration
│   ├── devcontainer.json      # Main config
│   ├── Dockerfile.dev         # Dev image
│   ├── Dockerfile             # Prod image
│   └── docker-compose.yml     # Services
│
├── Automation
│   ├── post-create.sh         # Initial setup
│   ├── post-start.sh          # Startup checks
│   ├── setup-network.sh       # Network setup
│   └── validate.sh            # Validation
│
└── Documentation
    ├── README.md              # Complete docs
    ├── QUICKSTART.md          # Quick reference
    ├── CI-CD.md              # CI/CD guide
    ├── SETUP_COMPLETE.md     # Setup summary
    └── FILES.md              # This file

.vscode/
├── launch.json               # Debugging
├── tasks.json                # Build tasks
└── settings.json             # Editor settings

Root/
└── DEVCONTAINER_READY.md     # Getting started
```

## Total Size

- DevContainer files: ~53KB
- VS Code configs: ~13KB
- Documentation: ~40KB
- **Total: ~106KB**

All text files, no binaries, fully version controllable.

## Quality Metrics

- ✅ All scripts executable
- ✅ JSON syntax valid (JSONC)
- ✅ Shell scripts lint-clean
- ✅ Documentation complete
- ✅ Examples provided
- ✅ Error handling included
- ✅ Color output for readability
- ✅ Progress indicators
- ✅ Help text comprehensive

## Usage Patterns

### First Time
1. Open project in VS Code
2. "Reopen in Container"
3. Wait for build (~10-15 min)
4. Run validation
5. Start coding

### Daily Use
1. Open project (~30 sec)
2. Run pd-build
3. Run pd-test
4. Start services if needed
5. Code with full tooling

### CI/CD
1. Use same Dockerfile.dev
2. Or pull pre-built image
3. Run tests in container
4. Deploy if successful

## Maintenance

### Updating Tools
Edit `Dockerfile.dev` to add/remove tools, then rebuild.

### Updating Extensions
Edit `devcontainer.json` extensions list, then rebuild.

### Updating Services
Edit `docker-compose.yml`, then restart services.

### Updating Documentation
Edit markdown files directly, no rebuild needed.

## Support

- **Full Docs**: `.devcontainer/README.md`
- **Quick Help**: `.devcontainer/QUICKSTART.md`
- **CI/CD**: `.devcontainer/CI-CD.md`
- **Validation**: `.devcontainer/validate.sh`

## Version

- **Version**: 1.0.0
- **Date**: January 27, 2026
- **Status**: Production Ready ✅

---

*All files are production-ready and tested.*
