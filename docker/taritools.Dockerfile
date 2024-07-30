# Stage 1: Build the binary
FROM rust:1-slim-bookworm AS builder
ARG FEATURES="sqlite"

# Install necessary dependencies including C compiler
RUN apt-get update && apt-get install -y \
    build-essential \
    ca-certificates \
    gcc \
    libssl-dev \
    pkg-config \
    libsqlite3-dev \
    libpq-dev \
    curl && \
    rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /usr/src/tari_payment_server

# Copy the source code
COPY . .

# Build the binary
RUN cargo build --release --features=${FEATURES}

# Stage 2: Create a minimal image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
COPY --from=builder /usr/src/tari_payment_server/target/release/taritools /usr/local/bin/taritools

# Set the entrypoint
ENTRYPOINT ["/usr/local/bin/taritools"]
