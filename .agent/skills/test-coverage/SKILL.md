---
name: test-coverage
description: Run tests and check coverage for Auth9 project (Backend & Frontend)
---

# Test Coverage Skill

This skill provides instructions on how to correctly run tests and check coverage for the Auth9 project, handling environment-specific configurations.

## Backend (Auth9 Core)

The backend uses `cargo` for testing and `cargo-tarpaulin` for coverage.
**Note**: We use `--run-types Tests` and `--ignore-config` to avoid issues with doctests and nightly toolchain requirements.

### Run Unit & Integration Tests
```bash
cd auth9-core
cargo test
```

### Run Coverage Analysis
```bash
cd auth9-core
cargo tarpaulin --ignore-config --run-types Tests --out Json --output-dir target/tarpaulin
```
*Output will be saved to `auth9-core/target/tarpaulin/tarpaulin-report.json`.*

## Frontend (Auth9 Portal)

The frontend uses `vitest` for testing and coverage.
**Note**: Always use `--run` flag to avoid watch mode in CI/automation scenarios.

### Run Unit Tests (without watch mode)
```bash
cd auth9-portal
npx vitest --run
```

### Run Tests and Coverage
```bash
cd auth9-portal
npm run test:coverage
```
*The `test:coverage` script already includes `--run` flag to avoid watch mode.*
*Coverage report will be displayed in the terminal and saved to `auth9-portal/coverage`.*

## Troubleshooting

- **Backend Compilation Errors**: If you encounter "Release" or "Pre-release" errors or missing dependencies, try running `cargo clean` first.
- **Backend Environment**: Ensure `.env` exists in `auth9-core` (copy from `.env.example`).
