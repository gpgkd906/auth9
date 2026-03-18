FROM rust:1.86-slim AS builder
WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.toml
COPY src src
COPY migrations migrations
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/auth9-oidc /usr/local/bin/auth9-oidc

EXPOSE 8090
CMD ["auth9-oidc"]
