# Claude Code Configuration for Auth9

This directory contains project rules and skills optimized for Claude Code.

## Directory Structure

```
.claude/
├── README.md              # This file
├── PROJECT_RULES.md       # Comprehensive project rules and conventions
└── skills/                # Task-specific skills
    ├── test-coverage.md   # Run tests and check coverage
    ├── ops.md             # Operations: logs, debugging, troubleshooting
    └── reset-local-env.md # Reset local Docker environment
```

## Using Project Rules

The `PROJECT_RULES.md` contains all project conventions and guidelines. You can reference it when:

- Starting work on the project
- Need to understand coding conventions
- Writing new code (Rust or TypeScript/Remix)
- Setting up tests
- Understanding the architecture

To use the rules, simply say:
```
"Follow the project rules in .claude/PROJECT_RULES.md when writing code"
```

Or Claude Code may automatically pick up these rules based on your custom instructions.

## Using Skills

Skills are task-specific guides that Claude Code can reference when performing operations.

### Test Coverage Skill

Use when running tests or checking coverage:

```
"Run tests and check coverage using the test-coverage skill"
"Run integration tests for auth9-core"
"Check test coverage and generate HTML report"
```

Commands you might want to run:
- `cargo test --lib` - Unit tests
- `cargo test --test '*'` - Integration tests
- `cargo tarpaulin --out Html` - Coverage report
- `npx vitest --run` - Frontend tests

### Operations Skill

Use when debugging or checking logs:

```
"Show me the logs for auth9-core using the ops skill"
"Run all tests for the backend"
"Check Kubernetes pod status"
```

Commands you might want to run:
- `docker logs -f auth9-core` - Follow backend logs
- `kubectl logs -f deployment/auth9-core -n auth9` - K8s logs
- `cargo test` - Run Rust tests
- `npm run test` - Run TypeScript tests

### Reset Local Environment Skill

Use when you need to reset the development environment:

```
"Reset the local Docker environment"
"Clean up and rebuild all containers"
```

Commands you might want to run:
- `./scripts/reset-docker.sh` - Full reset
- `docker-compose down --remove-orphans` - Stop containers
- `docker-compose up -d` - Start containers

## Setting Up Custom Instructions

You can add these instructions to your Claude Code settings or `.claude-code/settings.json`:

```json
{
  "customInstructions": "When working on the Auth9 project, follow the guidelines in .claude/PROJECT_RULES.md. Use TDD workflow with 90%+ test coverage requirement. For Rust code, use domain-driven design patterns. For TypeScript/Remix, follow Apple-style UI conventions."
}
```

## Integration with Claude Code

### Method 1: Direct Reference

In your conversations, you can directly reference these files:

```
"Read .claude/PROJECT_RULES.md and use it as context for all my requests"
"Follow the test-coverage skill in .claude/skills/test-coverage.md"
```

### Method 2: Custom Instructions

Add to your user-level or project-level custom instructions to automatically include these rules.

### Method 3: MCP Server (Advanced)

You can create a custom MCP server that exposes these skills as tools Claude Code can invoke.

## Quick Commands

### Backend (auth9-core)

```bash
# Run unit tests
cd auth9-core && cargo test --lib

# Run integration tests
cd auth9-core && cargo test --test '*'

# Check coverage
cd auth9-core && cargo tarpaulin --out Html

# View logs (Docker)
docker logs -f auth9-core

# View logs (K8s)
kubectl logs -f deployment/auth9-core -n auth9
```

### Frontend (auth9-portal)

```bash
# Run tests
cd auth9-portal && npm run test

# Run E2E tests
cd auth9-portal && npx playwright test

# Type check
cd auth9-portal && npm run typecheck

# View logs (Docker)
docker logs -f auth9-portal
```

### Environment Reset

```bash
# Full reset
./scripts/reset-docker.sh

# Or manually
docker-compose down --remove-orphans
docker-compose build --no-cache
docker-compose up -d
```

## Coverage Requirements

As specified in PROJECT_RULES.md:

| Layer | Target |
|-------|--------|
| Domain/Business logic | 95%+ |
| Service layer | 90%+ |
| Repository layer | 85%+ |
| API handlers | 80%+ |

**Overall minimum: 90%**

## TDD Workflow

Follow the RED-GREEN-REFACTOR cycle:

1. **RED**: Write a failing test first
2. **GREEN**: Write minimal code to pass the test
3. **REFACTOR**: Improve code while keeping tests green

## Support

For questions about these rules or skills:
- Check the original `.cursor` directory for source files
- Refer to project documentation in `/docs`
- See project README at repository root
