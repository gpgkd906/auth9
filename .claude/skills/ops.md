# Operations Skill

Run tests, check logs, and troubleshoot Auth9 services.

## When to Use

Use this when:
- Running tests (Rust cargo test, TypeScript vitest/playwright)
- Fetching service logs from Docker containers
- Fetching logs from Kubernetes pods
- Debugging services
- Troubleshooting issues

---

## Running Tests

### auth9-core (Rust)

```bash
# Unit tests (fast, no external dependencies)
cd auth9-core && cargo test --lib

# Integration tests (requires Docker for testcontainers)
cd auth9-core && cargo test --test '*'

# All tests
cd auth9-core && cargo test

# Test with output
cd auth9-core && cargo test -- --nocapture

# Coverage report (use llvm-cov, not tarpaulin)
cd auth9-core && cargo llvm-cov --html
```

### auth9-portal (TypeScript)

```bash
# Unit tests (Vitest)
cd auth9-portal && npm run test

# Watch mode
cd auth9-portal && npm run test -- --watch

# E2E tests (Playwright, requires running services)
cd auth9-portal && npx playwright test

# Linting
cd auth9-portal && npm run lint

# Type checking
cd auth9-portal && npm run typecheck
```

---

## Docker Logs

### Service Container Names

| Service | Container Name |
|---------|----------------|
| Backend API | auth9-core |
| Frontend | auth9-portal |
| Database | auth9-tidb |
| Cache | auth9-redis |
| Auth Engine | auth9-keycloak |
| DB Admin | auth9-adminer |

### Common Commands

```bash
# View logs (follow mode)
docker logs -f auth9-core
docker logs -f auth9-portal

# View last N lines
docker logs --tail 100 auth9-core

# View logs with timestamps
docker logs -t auth9-core

# View logs since time
docker logs --since 10m auth9-core

# All services via docker-compose
docker-compose logs -f
docker-compose logs -f auth9-core auth9-portal
```

### Troubleshooting Patterns

```bash
# Check service health
docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"

# Restart service
docker-compose restart auth9-core

# View error logs only (Rust backend)
docker logs auth9-core 2>&1 | grep -E "(ERROR|WARN|panic)"

# Database connection issues
docker logs auth9-tidb 2>&1 | tail -50

# Redis issues
docker exec auth9-redis redis-cli ping
```

---

## Kubernetes Logs

### Cluster Info

- **Namespace**: `auth9`
- **Deployments**: `auth9-core`, `auth9-portal`
- **Labels**: `app.kubernetes.io/name=auth9-core`, `app.kubernetes.io/name=auth9-portal`

### Common Commands

```bash
# Set default namespace
kubectl config set-context --current --namespace=auth9

# View pods
kubectl get pods -n auth9
kubectl get pods -n auth9 -l app.kubernetes.io/name=auth9-core

# Follow logs
kubectl logs -f deployment/auth9-core -n auth9
kubectl logs -f deployment/auth9-portal -n auth9

# Logs from all pods of a deployment
kubectl logs -f -l app.kubernetes.io/name=auth9-core -n auth9

# Last N lines
kubectl logs --tail=100 deployment/auth9-core -n auth9

# Logs since time
kubectl logs --since=10m deployment/auth9-core -n auth9

# Previous container logs (after crash)
kubectl logs -p deployment/auth9-core -n auth9
```

### Troubleshooting Patterns

```bash
# Check pod status
kubectl get pods -n auth9 -o wide

# Describe pod for events/errors
kubectl describe pod -l app.kubernetes.io/name=auth9-core -n auth9

# Check resource usage
kubectl top pods -n auth9

# Check HPA status
kubectl get hpa -n auth9

# View recent events
kubectl get events -n auth9 --sort-by='.lastTimestamp' | tail -20

# Exec into pod for debugging
kubectl exec -it deployment/auth9-core -n auth9 -- /bin/sh
```

### Multi-container Scenarios

```bash
# Specific container in multi-container pod
kubectl logs -f deployment/auth9-core -c auth9-core -n auth9

# All containers
kubectl logs -f deployment/auth9-core --all-containers -n auth9
```

---

## Quick Reference

| Task | Docker | Kubernetes |
|------|--------|------------|
| Follow logs | `docker logs -f auth9-core` | `kubectl logs -f deploy/auth9-core -n auth9` |
| Last 100 lines | `docker logs --tail 100 auth9-core` | `kubectl logs --tail=100 deploy/auth9-core -n auth9` |
| Since 10 min | `docker logs --since 10m auth9-core` | `kubectl logs --since=10m deploy/auth9-core -n auth9` |
| Restart | `docker-compose restart auth9-core` | `kubectl rollout restart deploy/auth9-core -n auth9` |
| Shell access | `docker exec -it auth9-core /bin/sh` | `kubectl exec -it deploy/auth9-core -n auth9 -- /bin/sh` |
