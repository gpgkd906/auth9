# Auth9 Core - Rust Backend
# Multi-stage build for minimal image size

# ==================== Chef Stage ====================
FROM rust:1.93 AS chef

# Install cargo-chef
RUN cargo install cargo-chef

WORKDIR /app

# ==================== Planner Stage ====================
FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ==================== Builder Stage ====================
FROM chef AS builder

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Build dependencies using cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code and build application
COPY . .
RUN cargo build --release

# ==================== Runtime Stage ====================
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false auth9

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/auth9-core /usr/local/bin/auth9-core

# Copy migrations
COPY migrations ./migrations

# Set ownership
RUN chown -R auth9:auth9 /app

# Switch to non-root user
USER auth9

# Environment
ENV RUST_LOG=info
ENV HTTP_HOST=0.0.0.0
ENV HTTP_PORT=8080
ENV GRPC_HOST=0.0.0.0
ENV GRPC_PORT=50051

# Expose ports
EXPOSE 8080 50051

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run
CMD ["auth9-core"]
