---
name: feature-request-governance
description: "Govern feature requests from docs/feature_request/ through a structured lifecycle: scan FR docs, produce implementation plans, execute changes, self-check closure, generate QA docs, execute QA testing, and clean up. Use when a user asks to implement a feature request, govern FRs, process feature requests, or says '治理 FR'. Also triggers when docs/feature_request/ contains unresolved documents."
---

# Feature Request Governance

End-to-end lifecycle for feature requests: discover → plan → implement → verify → QA doc → QA test → close.

## Workflow

```
1. Scan docs/feature_request/ and select target FR
2. Analyze FR → enter plan mode → produce governance plan
3. Implement plan (after user approval)
4. Self-check: verify FR requirements vs actual implementation
5. Generate QA test documentation for the implemented feature
6. Execute QA testing against the generated docs
7. Close or update FR document
```

## Step 1: Discover and Select FR

List all `.md` files in `docs/feature_request/`.

- **Zero files**: Report "No pending feature requests" and stop.
- **One file**: Auto-select it, show title and summary to user.
- **Multiple files**: Present numbered list with title and severity, ask user which to govern. Accept number or filename.

For each FR file, extract:
- Title (first `# ` heading)
- Source (if linked to a QA scenario or ticket)
- Severity
- Key requirements (bullet points from 期望行为 / Expected Behavior)

## Step 2: Analyze and Plan

Read the selected FR document thoroughly. Then:

1. **Identify affected modules** — Map FR requirements to codebase modules (auth9-core services/repositories/api, auth9-portal routes/components, database migrations, i18n, etc.)
2. **Read current implementation** — Read relevant source files to understand current state before proposing changes.
3. **Enter plan mode** — Use `EnterPlanMode` tool to create a structured governance plan.

Plan must follow the conventions in `references/plan-conventions.md`. Key requirements:

- Split independent sub-changes into parallel FRs when possible
- Provide file-level change table with operations
- Include implementation details at code level (not just descriptions)
- Include verification commands
- Minimize scope: only change what the FR requires

Present the plan to the user for approval before proceeding.

## Step 3: Implement

After user approves the plan:

1. Execute changes file by file following the plan
2. Run verification commands after each logical unit:
   - Type checking (`npm run typecheck` / `cargo clippy`)
   - Unit tests (`npm run test` / `cargo test`)
   - Build (`npm run build` / `cargo build`)
3. Fix any failures before moving to next unit
4. Do NOT commit — leave that to the user

## Step 4: Self-Check (FR Closure Verification)

After implementation, verify every requirement in the FR document:

1. Re-read the FR document
2. For each requirement in 期望行为 / Expected Behavior:
   - Find the corresponding implementation (code, test, config)
   - Mark as **fulfilled** or **unfulfilled** with evidence
3. Check for scope creep — implementation should not exceed FR scope

Build a closure report:

```
## FR Closure Report

| # | Requirement | Status | Evidence |
|---|-------------|--------|----------|
| 1 | [requirement text] | ✅ Fulfilled | [file:line or test name] |
| 2 | [requirement text] | ❌ Unfulfilled | [reason] |
```

## Step 5: Generate QA Test Documentation + Reset Environment (Parallel)

After implementation passes self-check, **launch two tasks in parallel**:

1. **Generate QA test documentation** (this step) — produces QA docs for the new behavior
2. **Reset local Docker environment** — run `./scripts/reset-docker.sh` in the background so the environment is ready with the latest binary by the time QA docs are done and Step 6 begins

This parallelization ensures the freshly-built code is deployed to Docker before QA testing starts, eliminating the wait-for-restart bottleneck.

```
┌─────────────────────────┐     ┌──────────────────────────┐
│  Generate QA docs       │     │  Reset Docker env         │
│  (Steps 5.1–5.5)       │     │  (background)             │
│  ~2-3 min               │     │  ~1-2 min                 │
└─────────┬───────────────┘     └──────────┬───────────────┘
          │                                 │
          └─────────────┬───────────────────┘
                        ▼
              Step 6: Execute QA Testing
              (environment is now ready)
```

The QA doc generation follows the `qa-doc-gen` skill conventions.

### 5.1 Determine QA scope

From the FR document and implementation, extract:

- **Module name**: Map to existing QA module directory (e.g., `tenant`, `user`, `rbac`, `service`, `auth`, `session`, `webhook`, `invitation`, `integration`, `settings`)
- **API endpoints**: Method + path + request/response shape
- **Database mutations**: Tables and columns affected
- **UI surfaces**: Pages, forms, navigation entries (if applicable)
- **Behavior boundaries**: Success flows, error cases, permission checks, edge cases

### 5.2 Generate QA doc

Create QA document(s) under `docs/qa/{module}/` following the template in `qa-doc-gen/references/qa-doc-template.md`.

**Key rules**:
- At most **5 scenarios per document** (split if needed)
- All text in **Chinese**, technical terms in English
- Follow `docs/testing/test-domain-policy.md` for test data (emails, URLs, domains)
- Include **Gate Check (步骤 0)** for scenarios requiring specific token types, data formats, or environment state
- API scenarios must include complete **curl commands**
- Data-mutating scenarios must include **预期数据状态** with verification SQL
- UI scenarios must verify **entry point visibility** (navigation links, sidebar items) — never test only by direct URL

**Scenario coverage priorities**:

| Type | When to include |
|------|----------------|
| Normal/happy path | Always |
| Duplicate/conflict | When creating unique resources |
| Invalid input | When FR specifies validation rules |
| Permission/auth | When endpoints are protected |
| Boundary | When FR specifies limits or constraints |
| Cascade effects | When delete operations have dependencies |

### 5.3 Run cross-doc impact analysis

Search `docs/qa/`, `docs/security/`, `docs/uiux/` for documents that reference changed endpoints, routes, or behaviors. Patch stale steps/assertions in impacted docs.

### 5.4 Update indexes

Update `docs/qa/README.md` (and `docs/security/README.md`, `docs/uiux/README.md` if affected) to register new documents.

### 5.5 If a test script is warranted

For API-heavy features, consider creating an automated test script under `scripts/qa/auto/` using the shared libraries in `scripts/qa/lib/` (`assert.sh`, `setup.sh`, `runner.sh`). Register the script in `docs/qa/_manifest.yaml` if the manifest exists.

## Step 6: Execute QA Testing

After QA docs are generated, execute the tests following the `qa-testing` skill workflow. This validates that the implementation actually works end-to-end.

### 6.1 Prerequisites check

If the environment reset was launched in parallel during Step 5, wait for it to complete first. Then verify:

```bash
# Check Docker services
docker ps --format "table {{.Names}}\t{{.Status}}" | grep auth9

# Verify API is responsive (with latest binary)
curl -sf http://localhost:8080/health && echo "OK" || echo "FAIL"
```

If services are not running (and no parallel reset was started), inform the user and ask whether to:
- Reset environment (`./scripts/reset-docker.sh`)
- Skip QA testing (mark as deferred in closure report)

### 6.2 Generate API token

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

### 6.3 Execute scenarios

For each scenario in the generated QA doc:

1. **Run Gate Checks (步骤 0)** if present — do NOT skip
2. **Execute test steps** — API calls via curl, or browser automation via `playwright-cli`
3. **Validate database state** — Run verification SQL
4. **Record result**: PASS or FAIL

**On FAIL**: Immediately create a ticket in `docs/ticket/` using the naming format `{module}_{document}_scenario{N}_{YYMMDD_HHMMSS}.md`. Do NOT defer ticket creation.

**On script-assisted scenarios**: If `scripts/qa/auto/` has a matching test script, run it instead of manual curl commands.

### 6.4 Report QA results

Build a QA summary:

```
## QA Testing Results

**QA Document**: docs/qa/{module}/{filename}.md
**Date**: {today}
**Results**: {passed}/{total} passed

| # | Scenario | Result | Notes |
|---|----------|--------|-------|
| 1 | {title}  | PASS   |       |
| 2 | {title}  | FAIL   | Ticket: docs/ticket/{name}.md |
```

### 6.5 Handle QA failures

- **All PASS**: Proceed to Step 7 (close FR)
- **FAIL with implementation bug**: Fix the bug, re-run failed scenarios. Do NOT close the FR until all scenarios pass.
- **FAIL with QA doc error** (wrong expectation, stale assertion): Fix the QA doc, re-run. Document the correction.
- **FAIL with environment issue** (Docker down, service misconfigured): Note in report, ask user whether to retry or defer.

## Step 7: Close or Update FR

Based on the closure report AND QA testing results:

### All requirements fulfilled AND QA passed → Delete FR
```bash
rm docs/feature_request/{fr_filename}.md
```
Report: "FR fully closed. QA passed. Document deleted."

### Some requirements unfulfilled OR QA failed → Update FR
Edit the FR document to:
1. Mark fulfilled requirements with ✅ and implementation reference
2. Keep unfulfilled requirements clearly visible
3. Add an implementation log section at the bottom:

```markdown
## Implementation Log

- **Date**: {today}
- **Fulfilled**: {list}
- **Remaining**: {list}
- **QA Status**: {passed}/{total} scenarios passed
- **QA Document**: docs/qa/{module}/{filename}.md
- **Tickets**: {list of created tickets, if any}
- **Notes**: {any blockers or decisions}
```

Report: "FR partially closed. Document updated with remaining items and QA results."

## Rules

- Never implement without showing the plan first
- Never delete FR document unless ALL requirements are verified AND QA testing passes
- Keep implementation scope strictly within FR boundaries
- Follow existing codebase patterns — do not introduce new abstractions unless the FR requires it
- If the FR is outdated or conflicts with current code, flag to user before planning
- QA docs must be generated before FR closure — no FR is considered complete without QA coverage
- QA testing failures that stem from implementation bugs must be fixed before FR closure
- Evidence artifacts go in `artifacts/qa/{date}/`, never in repository root
