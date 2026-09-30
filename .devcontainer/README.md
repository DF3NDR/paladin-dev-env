# Paladin DevContainer

Professional development container configuration for the Paladin multi-agent orchestration framework.

## Features

### 🛠️ Development Tools

- **Rust Toolchain**: Latest stable Rust with all components
- **Cargo Extensions**: watch, edit, audit, nextest, llvm-cov, and more
- **Debugging Tools**: gdb, lldb, valgrind, strace
- **Database Clients**: sqlite3, mysql-client, redis-tools
- **Modern CLI Tools**: bat, exa, ripgrep, fd, tokei

### 🔌 VS Code Extensions

Pre-configured with essential extensions:
- Rust Analyzer (LSP)
- Even Better TOML
- GitLens
- Docker
- Error Lens
- GitHub Copilot (if available)

### 🐳 Integrated Services

The DevContainer includes Docker Compose configuration for:
- **Redis**: Queue management and caching
- **RustFS**: S3-compatible object storage
- **MySQL**: Persistent database

### ⚙️ Configured Features

- **Formatting**: Auto-format on save with rustfmt
- **Linting**: Clippy enabled with strict warnings
- **Testing**: Integrated test explorer
- **Debugging**: Full LLDB debugging support
- **Git Hooks**: Pre-commit checks for format and lint

## LLM API credentials

Live-vendor tests need real provider keys. They are supplied from the **host**, never
baked into the image or committed.

### One-time host setup

`/home/vscode` is part of the container filesystem and is destroyed on every rebuild,
so keys cannot live there. `docker-compose.yml` bind-mounts the host's
`~/.config/paladin` read-only at `/home/vscode/.config/paladin`, which survives
rebuilds and is shared with any other project using the same convention.

**One file per secret; the filename is the variable name, lowercased.**

```bash
mkdir -p ~/.config/paladin
chmod 700 ~/.config/paladin

printf '%s' '<key>' > ~/.config/paladin/gemini_api_key       # -> GEMINI_API_KEY
printf '%s' '<key>' > ~/.config/paladin/xai_api_key          # -> XAI_API_KEY        (Grok)
printf '%s' '<key>' > ~/.config/paladin/moonshot_api_key     # -> MOONSHOT_API_KEY   (Kimi)
printf '%s' '<key>' > ~/.config/paladin/dashscope_api_key    # -> DASHSCOPE_API_KEY  (Qwen)
chmod 600 ~/.config/paladin/*
```

Create the directory *before* starting the container, or the Docker daemon creates it
root-owned. Then reopen the container and check with `make keys`.

The mapping is generic — no hardcoded provider list — so a new provider needs a new
file, not a code change. Recognised today:

| file | variable | provider |
|---|---|---|
| `openai_api_key` | `OPENAI_API_KEY` | OpenAI |
| `anthropic_api_key` | `ANTHROPIC_API_KEY` | Anthropic |
| `deepseek_api_key` | `DEEPSEEK_API_KEY` | DeepSeek |
| `gemini_api_key` | `GEMINI_API_KEY` | Google Gemini |
| `xai_api_key` | `XAI_API_KEY` | Grok (xAI) |
| `moonshot_api_key` | `MOONSHOT_API_KEY` | Kimi (Moonshot) |
| `dashscope_api_key` | `DASHSCOPE_API_KEY` | Qwen (DashScope) |
| `openai_compatible_api_key` | `OPENAI_COMPATIBLE_API_KEY` | generic provider |

### Precedence

`.devcontainer/paladin-env.sh` applies a file when the variable is **unset or empty**:

1. An exported non-empty value wins (host shell passthrough, or a one-off override).
2. Otherwise the key file is used.

The empty-check matters: the repo `.env` is auto-sourced into every shell and declares
these names with **empty** values. Without it, those blanks would mask real keys.

Values are trimmed of trailing newlines — a key written with `echo` would otherwise
carry `\n` into the auth header. Empty and placeholder files (`<...>`, `REPLACE_ME`,
`your...`) are ignored silently.

### Checking

```bash
make keys      # or `paladin-keys` in any interactive shell
```

Neither ever prints a key — only the variable name, provider, and character count.

## Claude Code session persistence

`/home/vscode` is part of the container filesystem and is destroyed on every rebuild,
and Claude Code keeps all of its user state there — session transcripts under
`.claude/projects/<escaped-cwd>/`, todos, shell snapshots, user settings, and the
`.credentials.json` auth token.

### One-time host setup

```bash
mkdir -p ~/.claude-paladin
chmod 700 ~/.claude-paladin
```

Run this on the **HOST** before the container is (re)built. Creating it first is
required, because the Docker daemon otherwise creates it root-owned and the
container user cannot write to it; `post-start.sh` reports both failure modes
(absent, and root-owned) with the fix to run.

### Mechanism

`docker-compose.yml` bind-mounts the host's `~/.claude-paladin` **read-write** at
`/home/vscode/.claude` (read-write, unlike the read-only credentials mount above,
because Claude Code writes here), and sets `CLAUDE_CONFIG_DIR=/home/vscode/.claude`
so that `.claude.json` — which by default sits at `$HOME/.claude.json`, outside
`.claude/`, and is rewritten by atomic rename — is kept inside the mounted directory
too. A single-file mount or a symlink would not work here: the rename replaces it
with a plain file and persistence breaks silently. Verified against Claude Code
2.1.239.

A dedicated directory is used rather than the host's real `~/.claude` so that a
Claude Code session running on the host cannot race or conflict with the
container's.

To change the host path, edit the mount line in `.devcontainer/docker-compose.yml`.
There is deliberately no environment-variable override, matching the credentials
mount.

### First run

You must authenticate Claude Code once after the mount is in place; the login then
persists across rebuilds because `.credentials.json` lives inside the mount.

### Session continuity

Transcripts are filed under `projects/-workspace/`, a key derived from the
`workspaceFolder` `/workspace` pinned in `devcontainer.json`. That path is stable
across rebuilds, so `claude --continue` and `claude --resume` still find prior
sessions afterwards.

### Scope

This persists Claude Code session transcripts, history, todos, shell snapshots,
user settings and auth — it does **not** persist the container filesystem
generally, so anything else written under `/home/vscode` is still lost on rebuild.
`/workspace/.claude/` is a different thing entirely — the project-local GSD
install — already persisted by the workspace bind mount and untouched by this
change.

## Quick Start

### Prerequisites

- [Visual Studio Code](https://code.visualstudio.com/)
- [Docker Desktop](https://www.docker.com/products/docker-desktop)
- [Remote - Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) VS Code extension

### Opening the DevContainer

1. Open the Paladin project in VS Code
2. When prompted, click "Reopen in Container"
   - Or use Command Palette: `Remote-Containers: Reopen in Container`
3. Wait for the container to build (first time takes ~10-15 minutes)
4. The post-create script will automatically:
   - Fetch dependencies
   - Build the project
   - Set up git hooks
   - Configure aliases

### First Build

The initial build will:
1. Install all Rust components and tools
2. Fetch and compile all project dependencies
3. Set up the development environment
4. Configure shell aliases and helpers

**Note**: The first build can take 10-15 minutes depending on your machine and network speed. Subsequent starts are much faster (< 30 seconds).

## Usage

### Quick Commands (Aliases)

The DevContainer includes convenient aliases:

```bash
# Building and Testing
pd-build      # cargo build
pd-test       # cargo test
pd-run        # cargo run
pd-check      # cargo check
pd-watch      # cargo watch (auto-rebuild on changes)

# Code Quality
pd-fmt        # cargo fmt
pd-clippy     # cargo clippy with strict warnings
pd-clean-code # Format, lint, and check all at once

# Documentation and Analysis
pd-doc        # Generate and open documentation
pd-bench      # Run benchmarks
pd-audit      # Security audit
pd-outdated   # Check for outdated dependencies
pd-bloat      # Analyze binary size

# Services (via Makefile)
pd-dev        # Start all services
pd-services   # Start supporting services only
pd-test-all   # Run all tests including integration

# Navigation
pd-src        # cd to src/
pd-tests      # cd to tests/
pd-docs       # cd to docs/
pd-examples   # cd to examples/
```

### Starting Services

The DevContainer can run alongside Redis, RustFS, and MySQL:

```bash
# Start all services
pd-dev

# Or using docker-compose directly
docker-compose -f .devcontainer/docker-compose.yml up -d
```

### Service Endpoints

When services are running:
- **Paladin API**: http://localhost:8080
- **Metrics**: http://localhost:9090
- **Redis**: localhost:6379
- **RustFS API**: http://localhost:9000
- **RustFS Console**: http://localhost:9001/rustfs/console/index.html (paladin-dev/paladin-dev-secret)
- **MySQL**: localhost:3306 (paladin/paladinpass)

### Running Tests

```bash
# Unit tests only
cargo test

# All tests
pd-test-all

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Integration tests
cargo test --test integration_test_name
```

### Debugging

1. Set breakpoints in your Rust code
2. Press `F5` or go to Run and Debug panel
3. Select "Debug Rust" configuration
4. The debugger will attach with full symbol support

### Code Quality Checks

Git pre-commit hooks automatically run:
- `cargo fmt --check` - Ensures code is formatted
- `cargo clippy -- -D warnings` - Ensures no clippy warnings

You can also run these manually:

```bash
pd-clean-code  # Runs fmt, clippy, and check
```

## Configuration

### Environment Variables

Create a `.env` file in the project root:

```bash
cp .env.example .env
```

Edit `.env` with your configuration:
```env
# LLM Providers
OPENAI_API_KEY=your_key_here
DEEPSEEK_API_KEY=your_key_here
ANTHROPIC_API_KEY=your_key_here

# Services
REDIS_URL=redis://redis:6379
MINIO_ENDPOINT=rustfs:9000
MINIO_ACCESS_KEY=paladin-dev
MINIO_SECRET_KEY=paladin-dev-secret

# Database
DATABASE_URL=mysql://paladin:paladinpass@mysql:3306/paladin
```

### Customizing the DevContainer

Edit `.devcontainer/devcontainer.json` to:
- Add VS Code extensions
- Change port forwarding
- Modify environment variables
- Add additional features

After changes, rebuild the container:
- Command Palette: `Remote-Containers: Rebuild Container`

## Troubleshooting

### Container Won't Start

1. Check Docker is running: `docker ps`
2. Check Docker logs: `docker logs <container_id>`
3. Rebuild container: `Remote-Containers: Rebuild Container`

### Slow Performance

1. Ensure Docker has sufficient resources (4+ GB RAM, 4+ CPU cores)
2. Check volume mounts are using `:cached` consistency
3. Target directory is mounted as a volume (not bind mount) for better performance

### Build Failures

1. Clean and rebuild:
   ```bash
   cargo clean
   cargo build
   ```

2. Check for dependency issues:
   ```bash
   cargo update
   cargo build
   ```

3. Verify network connectivity for crate downloads

### Port Conflicts

If ports are already in use:
1. Stop conflicting services
2. Or modify port mappings in `devcontainer.json`

### Git Issues

If git complains about safe directories:
```bash
git config --global --add safe.directory /workspace
```

## Advanced Usage

### Adding Custom Tools

Edit `.devcontainer/Dockerfile.dev` to add tools:

```dockerfile
RUN cargo install your-custom-tool
```

Rebuild the container after changes.

### Modifying Services

Edit `.devcontainer/docker-compose.yml` to:
- Add new services
- Change service configurations
- Modify resource limits

### Custom Shell Configuration

Add to `.devcontainer/post-create.sh` or `.devcontainer/post-start.sh`:
- Custom aliases
- Environment setup
- Auto-start commands

## CI/CD Integration

The same Dockerfile can be used for CI/CD pipelines:

```yaml
# .github/workflows/ci.yml
jobs:
  test:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/your-org/paladin-dev:latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo test --all-features
```

Build and push the dev image:
```bash
docker build -f .devcontainer/Dockerfile.dev -t paladin-dev .
docker tag paladin-dev ghcr.io/your-org/paladin-dev:latest
docker push ghcr.io/your-org/paladin-dev:latest
```

## Resources

- [VS Code DevContainers Documentation](https://code.visualstudio.com/docs/remote/containers)
- [Rust in DevContainers](https://github.com/microsoft/vscode-dev-containers/tree/main/containers/rust)
- [Docker Compose Documentation](https://docs.docker.com/compose/)

## Support

For issues or questions:
1. Check this README
2. Review [project documentation](../docs/)
3. Open an issue on GitHub
4. Check existing issues for solutions

## License

This DevContainer configuration is part of the Paladin project and follows the same MIT license.
