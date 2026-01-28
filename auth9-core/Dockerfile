# Auth9 Core - Rust Backend
# Multi-stage build for minimal image size

# ==================== Builder Stage ====================
FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./
COPY proto ./proto

# Create dummy main.rs to build dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src ./src

# Touch main.rs to force rebuild
RUN touch src/main.rs

# Build the actual application
RUN cargo build --release

# ==================== Runtime Stage ====================
FROM debian:bookworm-slim

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
