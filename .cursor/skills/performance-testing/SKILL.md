# Performance Testing

Run performance benchmarks for auth9-core using the `benchmark.sh` script. Use when the user wants to test API performance, measure QPS, check latency, or validate performance requirements.

## Prerequisites

- **hey** HTTP load testing tool
- Docker services (TiDB, Redis, Keycloak)
- auth9-core service running in release mode

## Auto-Setup (Run Before Benchmark)

Before running benchmarks, check and setup prerequisites automatically:

### Step 1: Install hey (if not installed)

```bash
# Check if hey is installed
which hey || brew install hey
```

### Step 2: Start Docker services (if not running)

```bash
# Check if Docker containers are running
docker ps | grep -q tidb || docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# Wait for services to be ready (TiDB takes ~10-15s)
sleep 15
```

### Step 3: Start auth9-core (if not running)

```bash
# Check if service is responding
curl -s http://localhost:8080/health > /dev/null 2>&1 || \
  (cd auth9-core && cargo run --release &)

# Wait for service to start
sleep 5
```

### One-liner Setup

```bash
# Full auto-setup: install hey, start docker, start service
which hey || brew install hey && \
docker ps | grep -q tidb || docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d && \
sleep 15 && \
(curl -s http://localhost:8080/health > /dev/null 2>&1 || (cd auth9-core && cargo run --release &)) && \
sleep 5 && \
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

## What It Tests

| Endpoint | Purpose | Measures |
|----------|---------|----------|
| `/health` | Pure compute performance | Rust/Axum baseline QPS |
| `/ready` | DB + Redis connectivity | I/O bound performance |
| `/api/v1/tenants` | Business logic | Real API performance |

## Output Metrics

- **Max Stable QPS**: Highest throughput before performance degradation
- **Best Concurrency**: Optimal concurrent connections for max QPS
- **P50/P99 Latency**: Response time percentiles

## Performance Targets

| Rating | /health QPS | Notes |
|--------|-------------|-------|
| Excellent | > 30,000 | Optimal Rust/Axum performance |
| Good | > 10,000 | Production ready |
| Fair | > 5,000 | Check for bottlenecks |
| Low | < 5,000 | Likely running in debug mode |

## Troubleshooting

**Low QPS?**
- Ensure running with `--release` flag
- Check if other processes consume resources
- Verify database/Redis connections are healthy

**Service not running?**
```bash
cd auth9-core && cargo run --release
```
