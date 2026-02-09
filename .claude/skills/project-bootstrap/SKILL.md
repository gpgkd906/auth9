---
name: project-bootstrap
description: Initialize a brand-new project from scratch with a default full-stack scaffold (Rust core + React Router 7 portal + Docker + Kubernetes). Use when a developer asks to bootstrap an empty repo or start a new project and no specific tech stack is requested.
---

# Project Bootstrap

Create a ready-to-run project skeleton with core, portal, docker, and k8s resources. Default stack matches Auth9 (Rust + React Router 7 + TypeScript + Vite).

## Quick Start

1. Choose a project name and target directory.
2. Run the initializer script.

```bash
.claude/skills/project-bootstrap/scripts/init_project.py \
  --name acme \
  --root /path/to/acme
```

Optional overrides:

```bash
.claude/skills/project-bootstrap/scripts/init_project.py \
  --name acme \
  --root /path/to/acme \
  --core-port 8080 \
  --portal-port 3000 \
  --namespace acme
```

## Workflow

1. Confirm requested stack. If no explicit preference, use default stack.
2. Run the initializer script with required parameters.
3. Verify local run instructions:
   - Core: `cd core && cargo run`
   - Portal: `cd portal && npm install && npm run dev`
   - Docker: `cd docker && docker-compose up -d`
4. Hand off the generated skeleton and next steps.

## What Gets Generated

- `core/` Rust service with `/health` and `/ready`
- `portal/` React Router 7 + Vite + TypeScript app
- `docker/` Dockerfiles and `docker-compose.yml`
- `k8s/` Base manifests for core and portal
- `scripts/` Local reset script
- `deploy/` K8s deploy/upgrade/cleanup scripts

## Script Templates

- `scripts/reset-docker.sh` – reset local Docker environment
- `deploy/deploy.sh` – apply k8s base manifests
- `deploy/upgrade.sh` – restart k8s deployments
- `deploy/cleanup.sh` – cleanup k8s namespace

Each script supports optional Auth9 extras via `ENABLE_AUTH9_EXTRAS=true`.

## Templates

Templates live in:

```
assets/template/
```

Use placeholders:
- `{{project_name}}`
- `{{core_port}}`
- `{{portal_port}}`
- `{{namespace}}`

## References

- `references/stack.md` for optional extensions (Keycloak/DB/Redis/etc.)
