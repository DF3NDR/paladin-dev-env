# Docker Deployment Guide

Complete guide for deploying Paladin using Docker, including multi-architecture support, versioning strategies, and production best practices.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Docker Images](#docker-images)
- [Configuration](#configuration)
- [Environment Variables](#environment-variables)
- [Volumes and Persistence](#volumes-and-persistence)
- [Networking](#networking)
- [Multi-Container Setup](#multi-container-setup)
- [Multi-Architecture Support](#multi-architecture-support)
- [Image Versioning](#image-versioning)
- [Health Checks](#health-checks)
- [Resource Limits](#resource-limits)
- [Production Deployment](#production-deployment)
- [Troubleshooting](#troubleshooting)

## Overview

Paladin provides official Docker images for easy deployment across environments. Images are:
- **Multi-architecture**: Support for AMD64 and ARM64
- **Versioned**: Semantic versioning with immutable tags
- **Optimized**: Multi-stage builds for minimal image size
- **Secure**: Non-root user, minimal attack surface

## Prerequisites

```bash
# Docker 20.10+
docker --version

# Docker Compose 2.0+ (optional)
docker-compose --version

# For building from source
make --version
cargo --version
```

## Quick Start

> **Development shortcut**: For local development use `make dev` (starts all services via `docker/docker-compose.dev.yml`) or `make services-up` (starts Redis + MinIO only). See `make help` for all targets.

### Run Prebuilt Image

```bash
# Pull and run latest Paladin image
docker run -d \
  --name paladin \
  -p 8080:8080 \
  -e OPENAI_API_KEY=your_api_key_here \
  -v paladin-data:/app/data \
  ghcr.io/your-org/paladin:latest
```

### Build and Run Locally

```bash
# Clone repository
git clone https://github.com/your-org/paladin.git
cd paladin

# Build Docker image
docker build -t paladin:local .

# Run container
docker run -d \
  --name paladin \
  -p 8080:8080 \
  -v ./config.yml:/app/config.yml \
  -v paladin-data:/app/data \
  paladin:local
```

## Docker Images

### Official Images

Paladin images are available from GitHub Container Registry:

```bash
# Latest stable release
ghcr.io/your-org/paladin:latest

# Specific version
ghcr.io/your-org/paladin:v0.8.0

# Latest commit on main branch
ghcr.io/your-org/paladin:main

# Development builds (feature branches)
ghcr.io/your-org/paladin:dev-<branch-name>
```

### Image Variants

| Tag Pattern | Description | Use Case |
|-------------|-------------|----------|
| `latest` | Most recent stable release | Production |
| `v<semver>` | Specific version (e.g., `v0.8.0`) | Production (pinned) |
| `main` | Latest commit on main branch | Staging |
| `<branch>` | Feature branch builds | Development |
| `slim` | Minimal image without examples | Production (space-constrained) |
| `debug` | Debug symbols included | Development/troubleshooting |

### Dockerfile

Paladin's multi-stage Dockerfile optimizes for size and security. There are three Dockerfiles in the repository:

- **`Dockerfile`** — Standard two-stage build (`builder` → `runtime`) for the full `paladin` CLI binary
- **`Dockerfile.chef`** — Cargo-chef optimized build for faster CI (caches Rust dependencies as a separate layer)
- **`Dockerfile.server`** — Builds `paladin-server` (the `web-server` feature only: agent HTTP API, health/readiness, OpenAPI docs, auth); built via `make docker-build-server` and used by `docker/docker-compose.server.yml`

The `paladin` binary carries `required-features = ["cli"]` (ADR-0023, `.planning/decisions/0023-cli-dependency-isolation.md`), so a build command that omits `--features cli` fails — the Dockerfile below always passes it.

```dockerfile
# Standard Dockerfile — two stages

# Stage 1: Builder (rust:1.93-slim-bookworm)
FROM rust:1.93-slim-bookworm AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config libssl-dev g++ curl \
    && rm -rf /var/lib/apt/lists/*
# curl is needed by the `utoipa-swagger-ui` build script (pulled by `paladin-web`)
# to download the Swagger UI bundle during the workspace build.

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY crates ./crates
COPY benches ./benches
# SQL migrations are embedded in the binary at compile time -- no migrations/
# directory to copy, and no manual migration step is required at container
# startup.

RUN cargo build --release --workspace --bin paladin --features cli
RUN strip target/release/paladin

# Stage 2: Runtime (debian:12-slim)
FROM debian:12-slim
WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/paladin /usr/local/bin/paladin

# Non-root user (uid/gid 65532)
RUN groupadd -g 65532 paladin && \
    useradd -u 65532 -g paladin -s /bin/false -M paladin && \
    chown -R paladin:paladin /app

USER paladin:paladin
EXPOSE 8080 9090
CMD ["/usr/local/bin/paladin"]
```

> **Note:** Port 9090 is reserved and exposed for future Prometheus metrics; no `/metrics`
> HTTP handler is wired up yet (the shipped routes are `/health` and `/ready`, see
> `crates/paladin-web/src/health.rs`).

> **Tip:** Use `Dockerfile.chef` in CI for faster builds — `cargo-chef` caches the dependency compilation layer separately from application code, so only changed crates are rebuilt.

## Configuration

### Configuration Files

Mount configuration files as volumes:

```bash
docker run -d \
  --name paladin \
  -v ./config.yml:/app/config.yml:ro \
  ghcr.io/your-org/paladin:latest
```

> **Note:** Paladin has no separate secrets file. Config loads from a single required
> `config.yml`/`config.toml` (plus an optional `config.$APP_ENV` override), and every value
> can additionally be overridden by an `APP_`-prefixed environment variable
> (`src/config/settings.rs`, `Environment::with_prefix("APP")`) — see
> [Environment Variables](#environment-variables) below.

### Example config.yml

```yaml
# config.yml
server:
  host: "0.0.0.0"
  port: 8080

# There is no top-level `paladin:` defaults section — a single Paladin's model,
# temperature, max_loops etc. are set programmatically via the `PaladinBuilder`
# Rust API. The HTTP service host (`paladin-server`) instead loads a list of
# agent definitions under `agents:` (see docs/src/user-guides/paladin-configuration.md).

garrison:
  garrison_type: "sqlite"
  path: "/app/data/garrison.db"
  max_entries: 1000
  max_tokens: 8000

arsenal:
  mcp_servers:
    - name: "web_search"
      server_type: "stdio"
      command: "uvx"
      args: ["mcp-web-search"]

llm:
  openai:
    api_key: "${OPENAI_API_KEY}"
    base_url: "https://api.openai.com/v1"
  deepseek:
    api_key: "${DEEPSEEK_API_KEY}"
    base_url: "https://api.deepseek.com/v1"
  anthropic:
    api_key: "${ANTHROPIC_API_KEY}"
    base_url: "https://api.anthropic.com/v1"

file_storage:
  minio_endpoint: "minio:9000"
  minio_access_key: "minioadmin"
  minio_secret_key: "minioadmin"
  minio_bucket: "paladin"
  minio_secure: false

queue:
  redis_host: "redis"
  redis_port: 6379
  redis_password: "changeme"
```

## Environment Variables

> **Note:** Paladin loads settings via the `config` crate with `Environment::with_prefix("APP")`
> (`src/config/settings.rs:66`) — every config-file key is overridable by an `APP_`-prefixed
> variable. The only *unprefixed* variables the binary itself reads directly are the three LLM
> provider API keys (`src/application/cli/commands/setup_check.rs`) and `RUST_LOG` (the standard
> Rust logging convention, `src/infrastructure/adapters/logs/system_log_adapter.rs`). There is no
> `SERVER_HOST`/`SERVER_PORT`/`LOG_LEVEL`/`DEFAULT_MODEL`/`DEFAULT_TEMPERATURE`/`DEFAULT_MAX_LOOPS`
> override — `server.host`/`server.port` are config-file-only, and there is no top-level Paladin
> defaults section to override (see the config.yml note above).

### Required Variables

```bash
# LLM Provider API Keys (read unprefixed, directly by the binary)
OPENAI_API_KEY=sk-...
DEEPSEEK_API_KEY=your_key_here
ANTHROPIC_API_KEY=your_key_here

# Redis (queue) — read by the Paladin binary as APP_REDIS_PASSWORD
APP_REDIS_PASSWORD=changeme

# MinIO (object storage) — read by the Paladin binary as APP_MINIO_ACCESS_KEY/APP_MINIO_SECRET_KEY.
# MINIO_ROOT_USER/MINIO_ROOT_PASSWORD below are consumed by the MinIO *container* itself
# (its own bootstrap credentials), not by the Paladin binary.
APP_MINIO_ACCESS_KEY=minioadmin
APP_MINIO_SECRET_KEY=minioadmin
MINIO_ROOT_USER=minioadmin
MINIO_ROOT_PASSWORD=minioadmin
```

### Optional Variables

```bash
# Logging
RUST_LOG=info

# Garrison configuration
APP_GARRISON_TYPE=sqlite
APP_GARRISON_PATH=/app/data/garrison.db
APP_GARRISON_MAX_ENTRIES=1000
```

### Passing Environment Variables

```bash
# From command line
docker run -d \
  -e OPENAI_API_KEY=sk-... \
  -e RUST_LOG=debug \
  ghcr.io/your-org/paladin:latest

# From .env file
docker run -d \
  --env-file .env \
  ghcr.io/your-org/paladin:latest

# In docker-compose.yml
services:
  paladin:
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - RUST_LOG=info
```

## Volumes and Persistence

### Data Volumes

Paladin requires persistent storage for:
- **Garrison database**: Conversation history
- **Citadel checkpoints**: State snapshots
- **Logs**: Application logs
- **Configuration**: Custom configs

```bash
# Named volumes
docker volume create paladin-data
docker volume create paladin-logs

docker run -d \
  --name paladin \
  -v paladin-data:/app/data \
  -v paladin-logs:/app/logs \
  ghcr.io/your-org/paladin:latest

# Bind mounts (host paths)
docker run -d \
  --name paladin \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/logs:/app/logs \
  ghcr.io/your-org/paladin:latest
```

### Volume Permissions

Paladin runs as non-root user (UID 1000). Ensure host directories have correct permissions:

```bash
# Set ownership for bind mounts
sudo chown -R 1000:1000 ./data ./logs

# Or use Docker volume (recommended)
docker volume create paladin-data
```

### Backup and Restore

```bash
# Backup volume
docker run --rm \
  -v paladin-data:/data \
  -v $(pwd)/backups:/backup \
  ubuntu tar czf /backup/paladin-data-$(date +%Y%m%d).tar.gz -C /data .

# Restore volume
docker run --rm \
  -v paladin-data:/data \
  -v $(pwd)/backups:/backup \
  ubuntu tar xzf /backup/paladin-data-20240101.tar.gz -C /data
```

## Networking

### Port Mapping

```bash
# Map container port to host
docker run -d \
  -p 8080:8080 \           # HTTP API (/health, /ready)
  -p 9090:9090 \           # Reserved for future Prometheus metrics — no /metrics route yet
  ghcr.io/your-org/paladin:latest
```

### Custom Networks

```bash
# Create network
docker network create paladin-net

# Run container on custom network
docker run -d \
  --name paladin \
  --network paladin-net \
  ghcr.io/your-org/paladin:latest

# Connect other services
docker run -d \
  --name redis \
  --network paladin-net \
  redis:7-alpine
```

## Multi-Container Setup

### Docker Compose

Complete setup with Redis, MinIO, and Paladin:

```yaml
# docker-compose.yml
version: '3.8'

services:
  redis:
    image: redis:7-alpine
    container_name: paladin-redis
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

  minio:
    image: minio/minio:latest
    container_name: paladin-minio
    ports:
      - "9000:9000"  # API
      - "9001:9001"  # Console
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    volumes:
      - minio-data:/data
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 5s
      timeout: 3s
      retries: 5

  paladin:
    image: ghcr.io/your-org/paladin:latest
    container_name: paladin
    ports:
      - "8080:8080"
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - DEEPSEEK_API_KEY=${DEEPSEEK_API_KEY}
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - RUST_LOG=info
      - APP_GARRISON_TYPE=sqlite
      - APP_GARRISON_PATH=/app/data/garrison.db
    volumes:
      - ./config.yml:/app/config.yml:ro
      - paladin-data:/app/data
      - paladin-logs:/app/logs
    depends_on:
      redis:
        condition: service_healthy
      minio:
        condition: service_healthy
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 3s
      retries: 3

volumes:
  redis-data:
  minio-data:
  paladin-data:
  paladin-logs:
```

### Running with Compose

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f paladin

# Stop services
docker-compose down

# Stop and remove volumes
docker-compose down -v
```

## Multi-Architecture Support

Paladin supports AMD64 and ARM64 architectures (Apple Silicon, ARM servers):

### Building Multi-Arch Images

```bash
# Create buildx builder (one-time setup)
docker buildx create --name multiarch --use
docker buildx inspect --bootstrap

# Build for multiple platforms
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ghcr.io/your-org/paladin:v0.8.0 \
  --push \
  .
```

### Automated Multi-Arch Builds

There is no `.github/workflows/docker-publish.yml` — the real pipeline is split across two
workflows: `.github/workflows/ci.yml`'s `docker` job builds (but does not push) a multi-arch
image on every PR/push as a build-and-size gate, and `.github/workflows/release.yml`'s
`build-docker` job builds, tags (via `docker/metadata-action`) and pushes to `ghcr.io` when a
release tag lands:

```yaml
# .github/workflows/release.yml, job: build-docker
- name: Build and push
  uses: docker/build-push-action@v5
  with:
    context: .
    platforms: linux/amd64,linux/arm64
    push: true
    tags: ${{ steps.meta.outputs.tags }}   # from docker/metadata-action, semver + latest
    labels: ${{ steps.meta.outputs.labels }}
    cache-from: type=gha
    cache-to: type=gha,mode=max
```

## Image Versioning

### Tagging Strategy

Paladin follows semantic versioning with Docker tags:

```bash
# Release v0.8.0
ghcr.io/your-org/paladin:latest       # Always points to latest release
ghcr.io/your-org/paladin:v0.8.0       # Immutable version tag
ghcr.io/your-org/paladin:v0.8         # Minor version (updates with patches)
ghcr.io/your-org/paladin:v0           # Major version

# Development
ghcr.io/your-org/paladin:main         # Latest main branch
ghcr.io/your-org/paladin:dev-feature  # Feature branch
```

### Version Pinning

**Production**: Always pin to specific versions:

```bash
# ✅ Good: Immutable version
docker run ghcr.io/your-org/paladin:v0.8.0

# ❌ Avoid: Latest can change
docker run ghcr.io/your-org/paladin:latest
```

**Development**: Use `latest` or branch tags:

```bash
docker run ghcr.io/your-org/paladin:main
```

## Health Checks

### Built-in Health Check

Paladin includes liveness and readiness endpoints (`crates/paladin-web/src/health.rs`):

```bash
# Liveness — always 200 once the process is up; no dependency checks
curl http://localhost:8080/health
# Response
{ "status": "ok" }

# Readiness — 200 once the agent registry is built and serving (shallow check,
# no network I/O against LLM/garrison/arsenal/queue)
curl http://localhost:8080/ready
# Response
{ "status": "ready", "agents": 3 }
```

### Docker Health Check

> **Note:** the shipped `Dockerfile` sets `HEALTHCHECK NONE` — health checking is left to the
> orchestrator's own probes (Kubernetes liveness/readiness, or a `healthcheck:` block in
> `docker-compose.yml` as shown in [Multi-Container Setup](#multi-container-setup)). A plain
> `docker run` of the built image has no Docker-level health check unless you add one, either
> with `--health-cmd` (see [Health Check Failing](#health-check-failing) below) or in compose.

```bash
# Check container health (only reports a status if a HEALTHCHECK was configured
# via --health-cmd or a compose healthcheck: block)
docker inspect --format='{{.State.Health.Status}}' paladin

# View health check logs
docker inspect --format='{{range .State.Health.Log}}{{.Output}}{{end}}' paladin
```

## Resource Limits

### CPU and Memory Limits

```bash
# Set resource limits
docker run -d \
  --name paladin \
  --cpus="2.0" \
  --memory="4g" \
  --memory-swap="4g" \
  ghcr.io/your-org/paladin:latest
```

### Docker Compose Limits

```yaml
services:
  paladin:
    image: ghcr.io/your-org/paladin:latest
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 4G
        reservations:
          cpus: '1.0'
          memory: 2G
```

### Recommended Limits

| Deployment | CPUs | Memory | Use Case |
|------------|------|--------|----------|
| **Minimal** | 0.5 | 512MB | Testing, low traffic |
| **Small** | 1.0 | 2GB | Development, light workloads |
| **Medium** | 2.0 | 4GB | Production (low-medium traffic) |
| **Large** | 4.0 | 8GB | Production (high traffic) |
| **XL** | 8.0 | 16GB | Enterprise, heavy workloads |

## Production Deployment

### Production-Ready Configuration

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  paladin:
    image: ghcr.io/your-org/paladin:v0.8.0  # Pinned version
    restart: unless-stopped
    environment:
      - RUST_LOG=warn  # Reduce log verbosity
      - RUST_BACKTRACE=0  # Disable backtraces
    volumes:
      - paladin-data:/app/data
      - paladin-logs:/app/logs
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 4G
      restart_policy:
        condition: on-failure
        delay: 5s
        max_attempts: 3
        window: 120s
    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
```

### Security Hardening

```bash
# Run as read-only filesystem
docker run -d \
  --read-only \
  --tmpfs /tmp \
  -v paladin-data:/app/data \
  ghcr.io/your-org/paladin:latest

# Drop capabilities
docker run -d \
  --cap-drop=ALL \
  --cap-add=NET_BIND_SERVICE \
  --security-opt=no-new-privileges \
  ghcr.io/your-org/paladin:latest
```

### Secrets Management

```bash
# Use Docker secrets (Swarm mode)
echo "$OPENAI_API_KEY" | docker secret create openai_key -

docker service create \
  --name paladin \
  --secret openai_key \
  -e OPENAI_API_KEY_FILE=/run/secrets/openai_key \
  ghcr.io/your-org/paladin:latest

# Use external secrets manager
docker run -d \
  --name paladin \
  -e AWS_REGION=us-east-1 \
  -e SECRET_NAME=paladin/openai \
  --env-file <(aws secretsmanager get-secret-value --secret-id paladin/openai --query SecretString --output text | jq -r 'to_entries|map("\(.key)=\(.value|tostring)")|.[]') \
  ghcr.io/your-org/paladin:latest
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker logs paladin

# Common issues:
# 1. Missing environment variables
docker logs paladin 2>&1 | grep "environment variable"

# 2. Port already in use
docker run -d -p 8081:8080 paladin  # Use different host port

# 3. Volume permission issues
docker run --user $(id -u):$(id -g) paladin
```

### Health Check Failing

```bash
# Test health endpoint manually
docker exec paladin curl -f http://localhost:8080/health

# Check service dependencies
docker-compose ps  # Are Redis/MinIO healthy?

# Increase health check timeout
docker run -d \
  --health-cmd "curl -f http://localhost:8080/health" \
  --health-interval=30s \
  --health-timeout=10s \
  --health-retries=5 \
  --health-start-period=60s \
  paladin
```

### High Memory Usage

```bash
# Check memory stats
docker stats paladin

# Set memory limits
docker update --memory="4g" --memory-swap="4g" paladin

# Check Garrison limits in config.yml
garrison:
  max_entries: 500  # Reduce if needed
  max_tokens: 4000
```

### Connectivity Issues

```bash
# Test network connectivity
docker exec paladin ping redis
docker exec paladin curl -v http://minio:9000

# Check DNS resolution
docker exec paladin nslookup redis

# Verify network
docker network inspect paladin-net
```

### Image Pull Failures

```bash
# Authenticate with GitHub Container Registry
echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin

# Pull with explicit platform
docker pull --platform linux/amd64 ghcr.io/your-org/paladin:latest

# Use mirror/proxy (if behind firewall)
docker pull ghcr.io/your-org/paladin:latest --registry-mirror=https://mirror.example.com
```

## Next Steps

- **[Kubernetes Deployment](kubernetes.md)** - Deploy to Kubernetes
- **[CI/CD Guide](cicd.md)** - Automated deployments
- **[Production Best Practices](production.md)** - Production checklist
- **[Monitoring](../operations/monitoring.md)** - Observability setup
