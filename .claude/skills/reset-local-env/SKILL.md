---
name: reset-local-env
description: Reset Auth9 local Docker development environment to a clean state.
---

# Reset Local Environment Skill

Reset Auth9 local Docker development environment.

## When to Use

Use this when:
- Need to reset local environment
- Need to clean Docker state
- Fixing dirty data issues
- Starting fresh with a clean development setup
- Encountering persistent errors after code changes
- Database schema changes require clean migration
- Keycloak configuration is corrupted
- Testing fresh installation flow
- Switching between branches with incompatible changes

---

## Quick Start

Run the reset script to get a clean local environment:

```bash
./scripts/reset-docker.sh
```

This script performs:
1. Stop and remove all containers
2. Remove project images (force rebuild)
3. Remove volumes (clean data)
4. Build all images from scratch
5. Start all services

---

## Initial Credentials

After reset, use these credentials:

| Service | URL | Username | Password |
|---------|-----|----------|----------|
| Admin Portal | http://localhost:3000 | admin@auth9.local | SecurePass123! |
| Keycloak Admin | http://localhost:8081 | admin | admin |

---

## Manual Steps (if script unavailable)

```bash
cd /path/to/auth9

COMPOSE_FILES="-f docker-compose.yml -f docker-compose.observability.yml"

# Stop and remove containers
docker-compose $COMPOSE_FILES down --remove-orphans

# Remove images
docker rmi auth9-auth9-core auth9-auth9-portal

# Remove volumes
docker volume rm auth9_tidb-data auth9_redis-data auth9_prometheus-data auth9_grafana-data auth9_loki-data auth9_tempo-data

# Rebuild and start
docker-compose $COMPOSE_FILES build --no-cache
docker-compose $COMPOSE_FILES up -d

# Wait for services
sleep 30
docker-compose $COMPOSE_FILES ps
```

---

## Service Ports

| Service | Port | Purpose |
|---------|------|---------|
| auth9-portal | 3000 | Admin dashboard |
| auth9-core | 8080 | REST API + /metrics |
| auth9-core | 50051 | gRPC |
| keycloak | 8081 | OIDC provider |
| tidb | 4000 | Database |
| redis | 6379 | Cache |
| prometheus | 9090 | Metrics collection |
| grafana | 3001 | Dashboards |
| loki | 3100 | Log aggregation |
| tempo | 3200/4317 | Distributed tracing |

---

## Verification Steps

After reset, verify services are healthy:

```bash
# Check all containers are running
docker-compose ps

# Test API health endpoint
curl http://localhost:8080/health

# Test portal is accessible
curl http://localhost:3000

# Check database connection
docker exec auth9-tidb mysql -e "SHOW DATABASES;"

# Check Redis
docker exec auth9-redis redis-cli ping
```

---

## Common Issues

### Containers won't start

```bash
# Check logs
docker-compose logs

# Check specific service
docker logs auth9-core
```

### Port conflicts

```bash
# Check what's using the ports
lsof -i :3000
lsof -i :8080
lsof -i :4000

# Kill conflicting processes or change ports in docker-compose.yml
```

### Volume permission issues

```bash
# Remove volumes with sudo if needed
docker volume ls
sudo docker volume rm auth9_tidb-data auth9_redis-data auth9_prometheus-data auth9_grafana-data auth9_loki-data auth9_tempo-data
```
