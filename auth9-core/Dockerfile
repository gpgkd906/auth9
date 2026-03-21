# Auth9 Core - Rust Backend
# Multi-stage build for minimal image size
# Build context: repository root (needed for auth9-oidc path dependency)

# ==================== Chef Stage ====================
FROM lukemathwalker/cargo-chef:latest-rust-1.93-bookworm AS chef

# Install protobuf compiler (shared by planner and builder)
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# ==================== Planner Stage ====================
FROM chef AS planner

COPY auth9-core/ ./auth9-core/
COPY auth9-oidc/ ./auth9-oidc/
WORKDIR /app/auth9-core
RUN cargo chef prepare --recipe-path /app/recipe.json

# ==================== Builder Stage ====================
FROM chef AS builder

# Copy auth9-oidc first (dependency)
COPY auth9-oidc/ ./auth9-oidc/

# Build dependencies using cargo-chef (mount cache survives --no-cache rebuilds)
WORKDIR /app/auth9-core
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/auth9-core/target \
    cargo chef cook --release --recipe-path recipe.json

# Copy source code and build application
COPY auth9-core/ ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/auth9-core/target \
    cargo build --release && \
    cp /app/auth9-core/target/release/auth9-core /app/auth9-core-bin

# ==================== Runtime Stage ====================
FROM debian:bookworm-slim

# Install runtime dependencies and apply security patches
RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false auth9

WORKDIR /app

# Copy binary from builder (from explicit path, not inside mount cache)
COPY --from=builder /app/auth9-core-bin /usr/local/bin/auth9-core

# Copy migrations
COPY auth9-core/migrations ./migrations

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
