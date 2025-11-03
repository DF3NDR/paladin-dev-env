# Multi-stage build for Rust application
FROM rust:latest AS builder

WORKDIR /

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY tests ./tests

# Build dependencies (this will be cached if dependencies don't change)
RUN cargo build --release

# Production stage
FROM debian:bookworm-slim AS production

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/paladin /app/paladin

# Copy configuration files
COPY config /app/config

# Create a non-root user
RUN useradd -m -u 1001 appuser && chown -R appuser:appuser /app
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["./paladin"]

# Development stage
FROM rust:latest AS development

WORKDIR /app

# Install development dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY tests ./tests
COPY config ./config

# Install cargo-watch for development
RUN cargo install cargo-watch

EXPOSE 8080

CMD ["cargo", "run"]

# Test stage
FROM rust:latest AS test

WORKDIR /app

# Install test dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy everything needed for tests
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY tests ./tests

# Pre-build dependencies
RUN cargo build --tests

CMD ["cargo", "test"]