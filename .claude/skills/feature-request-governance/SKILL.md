---
name: feature-request-governance
description: "Govern feature requests from docs/feature_request/ through a structured lifecycle: scan FR docs, produce implementation plans, execute changes, self-check closure, and clean up. Use when a user asks to implement a feature request, govern FRs, process feature requests, or says '治理 FR'. Also triggers when docs/feature_request/ contains unresolved documents."
---

# Feature Request Governance

End-to-end lifecycle for feature requests: discover → plan → implement → verify → close.

## Workflow

```
1. Scan docs/feature_request/ and select target FR
2. Analyze FR → enter plan mode → produce governance plan
3. Implement plan (after user approval)
4. Self-check: verify FR requirements vs actual implementation
5. Close or update FR document
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

## Step 5: Close or Update FR

Based on the closure report:

### All requirements fulfilled → Delete FR
```bash
rm docs/feature_request/{fr_filename}.md
```
Report: "FR fully closed. Document deleted."

### Some requirements unfulfilled → Update FR
Edit the FR document to:
1. Mark fulfilled requirements with ✅ and implementation reference
2. Keep unfulfilled requirements clearly visible
3. Add an implementation log section at the bottom:

```markdown
## Implementation Log

- **Date**: {today}
- **Fulfilled**: {list}
- **Remaining**: {list}
- **Notes**: {any blockers or decisions}
```

Report: "FR partially closed. Document updated with remaining items."

## Rules

- Never implement without showing the plan first
- Never delete FR document unless ALL requirements are verified
- Keep implementation scope strictly within FR boundaries
- Follow existing codebase patterns — do not introduce new abstractions unless the FR requires it
- If the FR is outdated or conflicts with current code, flag to user before planning
