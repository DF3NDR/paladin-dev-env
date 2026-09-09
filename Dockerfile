# Multi-stage Dockerfile for Paladin
# Optimized for production deployment with minimal image size
# Supports multi-architecture builds (amd64, arm64)

# =============================================================================
# Stage 1: Builder
# Builds the application with all dependencies
# Note: Using Bookworm (Debian 12) for OpenSSL 3.x support
# =============================================================================
FROM rust:1.93-slim-bookworm AS builder
WORKDIR /app

# Install required build dependencies.
# `curl` is needed by the `utoipa-swagger-ui` build script (pulled by `paladin-web`) to
# download the Swagger UI bundle during the workspace build.
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    g++ \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy all source files
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY crates ./crates
COPY benches ./benches
# `paladin-cli eval run` compiles the shared E2E graph builders from tests/helpers
# via a `#[path]` include (plan 28-16); the lib target needs the file present.
COPY tests ./tests
# SQL migrations are embedded in the binary at compile time (D-17,
# crates/paladin-memory/src/migrations.rs) -- no migrations/ directory to copy.
# config.yml is gitignored (env-specific); provide at runtime via volume mount

# Build the application in release mode
# --workspace ensures all crates are resolved correctly
# --features cli: the `paladin` binary carries `required-features = ["cli"]` (ADR-0023) —
# omitting this flag now fails the build.
RUN cargo build --release --workspace --bin paladin --features cli

# Strip debug symbols to reduce binary size
RUN strip target/release/paladin

# =============================================================================
# Stage 2: Runtime
# Minimal runtime image with only the binary
# =============================================================================
FROM debian:12-slim
WORKDIR /app

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/paladin /usr/local/bin/paladin
# Migrations are embedded in the binary (D-17); config.yml must be provided at
# runtime via volume mount

# Create non-root user
RUN groupadd -g 65532 paladin && \
    useradd -u 65532 -g paladin -s /bin/false -M paladin && \
    chown -R paladin:paladin /app

# Use non-root user
USER paladin:paladin

# Expose ports
EXPOSE 8080 9090

# Health check (distroless has limited shell, so we use simple approach)
# Note: Kubernetes liveness/readiness probes will handle health checking
HEALTHCHECK NONE

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/paladin"]

# Default command (can be overridden)
CMD ["--help"]
