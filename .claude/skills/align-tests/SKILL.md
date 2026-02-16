---
name: align-tests
description: Execute unit-test alignment after large refactors or broad code changes. Trigger when the user explicitly uses the command "-- 对齐测试 --" or asks to align/fix tests after massive modifications. Run relevant unit tests, analyze failing test cases, update tests and/or implementation, and iterate until stable.
---

# Align Tests

Run a strict test-alignment workflow after major refactors.

## Workflow

1. Scope the impacted areas.
- Inspect changed files with `git status --short` and `git diff --name-only`.
- Prioritize test suites by changed modules:
  - `auth9-core/*` -> Rust unit/integration tests.
  - `auth9-portal/*` -> Vitest unit tests and type checks when relevant.

2. Run unit tests first, then broaden.
- Backend:
  - `cd auth9-core && cargo test --lib`
  - If needed: `cd auth9-core && cargo test`
- Frontend:
  - `cd auth9-portal && npm run test`
  - If failures suggest type drift: `cd auth9-portal && npm run typecheck`

3. Analyze failures by root cause.
- Classify each failure:
  - Refactor signature drift (function/argument/type changed).
  - Behavior change (assertion no longer matches intended logic).
  - Test fixture/mock drift (mocked dependency contract changed).
  - Environment/order issues (state leakage, async timing).
- Prefer preserving intended product behavior; only update expectations when behavior change is intentional.

4. Apply fixes with minimal blast radius.
- Update tests to match deliberate API/contract changes.
- Update implementation when behavior regressed unintentionally.
- Keep handlers thin; move logic fixes into `service/` for backend when applicable.
- Follow project conventions:
  - Rust: `cargo fmt`, `cargo clippy` when needed.
  - Frontend: maintain ESLint/TypeScript compatibility.

5. Re-run and converge.
- Re-run failing suites immediately after each fix.
- Re-run full relevant unit-test set before finishing.
- Stop only when tests pass or a clear blocker is identified.

6. Run and fix backend/frontend lint.
- Backend:
  - `cd auth9-core && cargo clippy`
  - If clippy suggests formatting-related changes: `cd auth9-core && cargo fmt`
- Frontend:
  - `cd auth9-portal && npm run lint`
  - If failures indicate type drift: `cd auth9-portal && npm run typecheck`
- Fix lint findings with minimal blast radius, then re-run lint commands until clean.
- After lint is clean, re-run relevant unit tests to ensure no regressions.

7. Report outcome clearly.
- Summarize:
  - Failing tests and root causes.
  - Files updated.
  - Final test status and remaining risks/blockers.

## Rules

- Do not skip failure analysis; every failing test must have a cause.
- Do not mass-disable tests to get green.
- Do not introduce external dependencies for Rust test execution.
- Do not skip lint verification for touched backend/frontend modules.
- If a blocker prevents completion, provide exact failing command and error snippet.
