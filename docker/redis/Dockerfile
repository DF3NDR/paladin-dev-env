# docker/Dockerfile

# Multi-stage build for the in4me application with Redis queue support

FROM rust:1.75 as base

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy dependency files
COPY Cargo.toml Cargo.lock ./

# Development stage
FROM base as development

# Install development tools
RUN apt-get update && apt-get install -y \
    gdb \
    valgrind \
    && rm -rf /var/lib/apt/lists/*

# Install cargo tools for development
RUN cargo install cargo-watch cargo-expand

# Copy source code
COPY src ./src
COPY tests ./tests

# Build in debug mode
RUN cargo build

CMD ["cargo", "run"]

# Testing stage
FROM base as test

# Copy source and test files
COPY src ./src
COPY tests ./tests

# Build test dependencies
RUN cargo build --tests

CMD ["cargo", "test"]

# Production build stage
FROM base as builder

# Create app user
RUN useradd -m -u 1001 appuser

# Copy source code
COPY src ./src
COPY tests ./tests

# Build the application in release mode
RUN cargo build --release

# Production runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1001 appuser

# Create necessary directories
RUN mkdir -p /app/data /app/config && \
    chown -R appuser:appuser /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/in4me /app/in4me
COPY --from=builder /app/target/release/deps /app/deps

# Copy configuration files
COPY config /app/config

# Set ownership
RUN chown -R appuser:appuser /app

# Switch to app user
USER appuser

# Set working directory
WORKDIR /app

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=30s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["./in4me"]

# Default target is production
FROM runtime as default