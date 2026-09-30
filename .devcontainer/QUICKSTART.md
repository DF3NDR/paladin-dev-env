# DevContainer Quick Reference

## 🚀 Opening the DevContainer

1. **First Time Setup**:
   ```
   1. Open VS Code
   2. Install "Remote - Containers" extension
   3. Open this project folder
   4. Click "Reopen in Container" when prompted
   5. Wait ~10-15 minutes for initial build
   ```

2. **Subsequent Opens**:
   - Just reopen the project - container starts in ~30 seconds

## 🎯 Common Tasks

### Build & Run
```bash
pd-build      # Build project
pd-run        # Run main binary
pd-watch      # Auto-rebuild on file changes
cargo run --bin paladin-cli  # Run CLI tool
```

### Testing
```bash
pd-test       # Run all unit tests
pd-test-all   # Run all tests (unit + integration)
cargo test test_name         # Run specific test
cargo test -- --nocapture    # See println! output
cargo nextest run            # Use nextest (faster)
```

### Code Quality
```bash
pd-fmt        # Format code
pd-clippy     # Lint with clippy
pd-clean-code # Format + Lint + Check (all at once)
cargo audit   # Security audit
```

### Services
```bash
pd-dev        # Start all services (Redis, RustFS, MySQL)
pd-services   # Start services without app

# Or use docker-compose directly
docker-compose -f .devcontainer/docker-compose.yml up -d
docker-compose -f .devcontainer/docker-compose.yml down
```

### Documentation
```bash
pd-doc        # Generate and open docs
cargo doc --no-deps --open  # Docs without dependencies
```

## 🔧 Debugging

### From VS Code
1. Set breakpoint in code
2. Press `F5` or click "Run and Debug"
3. Select appropriate debug configuration:
   - "Debug Paladin" - Main binary
   - "Debug CLI" - CLI tool
   - "Debug Unit Tests" - Test debugging
   - "Debug Current Test" - Debug selected test

### Command Line
```bash
rust-lldb target/debug/paladin
rust-gdb target/debug/paladin
```

## 🌐 Service URLs

When services are running:
- **Paladin API**: http://localhost:8080
- **Metrics**: http://localhost:9090
- **RustFS Console**: http://localhost:9001/rustfs/console/index.html (paladin-dev/paladin-dev-secret)
- **Redis**: localhost:6379

## 📦 Useful Cargo Commands

```bash
# Dependencies
cargo tree              # Show dependency tree
cargo outdated          # Check for outdated deps
cargo update            # Update dependencies

# Analysis
cargo bloat --release   # Binary size analysis
cargo expand            # Expand macros
cargo geiger            # Unsafe code detection

# Performance
cargo bench             # Run benchmarks
cargo flamegraph        # Generate flamegraph

# Cleaning
cargo clean             # Clean all build artifacts
cargo clean -p paladin  # Clean specific package
```

## 🐛 Troubleshooting

### "rust-analyzer not working"
```bash
# Restart rust-analyzer
Ctrl+Shift+P -> "Rust Analyzer: Restart Server"

# Or rebuild
cargo clean && cargo check
```

### "Port already in use"
```bash
# Check what's using the port
lsof -i :8080

# Kill the process
kill -9 <PID>
```

### "Cannot connect to services"
```bash
# Check services are running
docker-compose -f .devcontainer/docker-compose.yml ps

# Restart services
docker-compose -f .devcontainer/docker-compose.yml restart
```

### "Build is slow"
```bash
# Use cargo check (faster than build)
pd-check

# Use nextest (parallel testing)
cargo nextest run

# Clean and rebuild
cargo clean && cargo build
```

### "Out of disk space"
```bash
# Clean Docker volumes
docker system prune -a --volumes

# Clean cargo cache
cargo clean
rm -rf target/
```

## 🔐 Environment Variables

Create/edit `.env` file:
```env
# LLM Providers
OPENAI_API_KEY=sk-...
DEEPSEEK_API_KEY=...
ANTHROPIC_API_KEY=...

# Services
REDIS_URL=redis://redis:6379
MINIO_ENDPOINT=rustfs:9000
DATABASE_URL=mysql://paladin:paladinpass@mysql:3306/paladin

# Logging
RUST_LOG=debug
RUST_BACKTRACE=1
```

## 📝 Git Workflow

```bash
# Pre-commit hooks run automatically
git commit -m "message"

# If hooks fail:
pd-clean-code   # Fix formatting and lints
git add .
git commit -m "message"

# Bypass hooks (not recommended)
git commit --no-verify -m "message"
```

## 🎨 VS Code Shortcuts

- `Ctrl+Shift+B` - Build task
- `Ctrl+Shift+T` - Run tests
- `F5` - Start debugging
- `Ctrl+Shift+P` - Command palette
- `Ctrl+`` - Toggle terminal
- `Ctrl+K Ctrl+0` - Fold all
- `Ctrl+K Ctrl+J` - Unfold all

## 🛠️ Customization

### Add VS Code Extension
Edit `.devcontainer/devcontainer.json`:
```json
"customizations": {
  "vscode": {
    "extensions": [
      "your.extension.id"
    ]
  }
}
```
Then rebuild container: `Ctrl+Shift+P` -> "Rebuild Container"

### Add Cargo Tool
Edit `.devcontainer/Dockerfile.dev`:
```dockerfile
RUN cargo install your-tool
```
Then rebuild container.

### Add System Package
Edit `.devcontainer/Dockerfile.dev`:
```dockerfile
RUN apt-get update && apt-get install -y your-package
```
Then rebuild container.

## 📊 Performance Tips

1. **Use cargo-watch for development**:
   ```bash
   pd-watch  # Auto-rebuild on changes
   ```

2. **Use nextest for faster testing**:
   ```bash
   cargo nextest run
   ```

3. **Use cargo check instead of build**:
   ```bash
   cargo check  # Faster than full build
   ```

4. **Enable parallel compilation**:
   Already configured in the container with all CPU cores.

5. **Use release builds for benchmarks**:
   ```bash
   cargo build --release
   cargo bench
   ```

## 🔗 Resources

- [Paladin Documentation](../docs/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [VS Code Rust Setup](https://code.visualstudio.com/docs/languages/rust)

## 💡 Tips & Tricks

1. **Use bat instead of cat**:
   ```bash
   cat file.rs    # Plain output
   bat file.rs    # Syntax highlighted
   ```

2. **Use exa instead of ls**:
   ```bash
   ll            # Alias for 'exa -la'
   ```

3. **Use ripgrep for fast searching**:
   ```bash
   rg "pattern"  # Much faster than grep
   ```

4. **Use tokei for code statistics**:
   ```bash
   tokei         # Lines of code statistics
   ```

5. **Use cargo-expand for macro debugging**:
   ```bash
   cargo expand  # See macro expansions
   ```

## 🎯 Next Steps

1. Read the [project README](../README.md)
2. Check [Quickstart Guide](../docs/QUICKSTART.md)
3. Browse [examples](../examples/)
4. Run your first test: `pd-test`
5. Build the project: `pd-build`
6. Start services: `pd-dev`
7. Run the application: `pd-run`

Need help? Check the [full DevContainer README](.devcontainer/README.md)
