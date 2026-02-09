---
name: performance-testing
description: Run performance benchmarks for auth9-core API using hey load testing tool.
---

# Performance Testing

Run performance benchmarks for auth9-core using `benchmark.sh`.

## Prerequisites

- **hey** HTTP load testing tool
- Docker services (TiDB, Redis, Keycloak)
- auth9-core running in **release mode**

## Quick Start

```bash
# Install hey if needed
which hey || brew install hey

# Start Docker services
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d
sleep 15

# Start auth9-core in release mode
cd auth9-core && cargo run --release &
sleep 5

# Run benchmark
./scripts/benchmark.sh
```

## Usage

```bash
# Quick test (50-200 concurrent connections)
./scripts/benchmark.sh

# Full test (50-2000 concurrent connections)
./scripts/benchmark.sh full

# Custom target URL
BASE_URL=http://your-server:8080 ./scripts/benchmark.sh
```

## Tested Endpoints

| Endpoint | Purpose | Measures |
|----------|---------|----------|
| `/health` | Pure compute | Rust/Axum baseline QPS |
| `/ready` | DB + Redis | I/O bound performance |
| `/api/v1/tenants` | Business logic | Real API performance |

## Output Metrics

- **Max Stable QPS**: Highest throughput before degradation
- **Best Concurrency**: Optimal concurrent connections
- **P50/P99 Latency**: Response time percentiles

## Performance Targets

| Rating | /health QPS | Notes |
|--------|-------------|-------|
| Excellent | > 30,000 | Optimal performance |
| Good | > 10,000 | Production ready |
| Fair | > 5,000 | Check bottlenecks |
| Low | < 5,000 | Likely debug mode |

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Low QPS | Ensure `--release` flag, check resource usage |
| Service not running | `cd auth9-core && cargo run --release` |
| DB/Redis issues | `docker-compose ps`, restart services |
