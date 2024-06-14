# Use the official Rust image as a base
FROM rust:latest as builder

# Create a new empty shell project
RUN USER=root cargo new --bin smartcontent-aggregator
WORKDIR /smartcontent-aggregator

# Copy the manifest
COPY ./Cargo.toml ./Cargo.toml

# Build only the dependencies to cache them
RUN cargo build --release
RUN rm src/*.rs

# Now copy the source files
COPY ./src ./src

# Build for release
RUN cargo build --release

# Use a minimal base image to run the application
FROM debian:buster-slim
COPY --from=builder /smartcontent-aggregator/target/release/smartcontent-aggregator /usr/local/bin/smartcontent-aggregator

# Run the binary
CMD ["smartcontent-aggregator"]
