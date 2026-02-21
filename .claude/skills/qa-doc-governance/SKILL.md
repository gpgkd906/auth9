---
name: qa-doc-governance
description: "Govern and periodically remediate QA documentation quality across docs/qa, docs/security, and docs/uiux. Use when users ask to audit QA docs, run recurring documentation checks, fix doc drift after refactors, enforce scenario limits/checklists/UI entry visibility, or synchronize README/manifest/changelog consistency."
---

# QA Doc Governance

Run a repeatable governance loop for QA documentation quality, consistency, and discoverability.

## Workflow

1. Baseline audit.
2. Classify findings by severity and scope.
3. Apply targeted remediation.
4. Sync index and governance artifacts.
5. Validate and publish audit result.

## Step 1: Baseline Audit

1. Run `scripts/qa-doc-lint.sh`.
2. Run `./.claude/skills/qa-doc-governance/scripts/run-audit.sh` for expanded metrics.
3. If needed, read `references/policy-checklist.md` for rule definitions.

## Step 2: Classify Findings

Use this priority order:

1. `P0`: Broken navigability or unusable test flow.
: Examples: missing checklist, misleading auth/session steps, README index drift.
2. `P1`: Governance drift.
: Examples: >5 scenarios in one file, missing UI entry visibility in UI-facing docs.
3. `P2`: Style consistency improvements.
: Examples: naming alignment, wording normalization.

## Step 3: Remediate

Apply these rules in order:

1. Fix executable correctness first.
: Replace "close browser => unauthenticated" with explicit methods (incognito, clear `auth9_session`, sign out).
2. Enforce visibility-first UI flows.
: UI scenarios start from visible entry points (sidebar, tab, button, quick links), not direct URL.
3. Enforce scenario cap.
: Keep each file `<=5` numbered scenarios. Split long files into `base/advanced` or topic-specific docs.
4. Keep checklists mandatory.
: Every doc includes `## 检查清单` or `## 回归测试检查清单`.

## Step 4: Sync Governance Artifacts

1. Regenerate `docs/qa/_manifest.yaml` with current scenario counts and flags.
2. Update `docs/qa/README.md`:
: module index, document counts, scenario totals, and changelog.
3. If QA behavior changes affect security/UIUX expectations, add alignment notes to:
: `docs/security/README.md`, `docs/uiux/README.md`.

## Step 5: Validate and Report

1. Re-run `scripts/qa-doc-lint.sh` until pass.
2. Include audit report summary with:
: fixed issues, remaining warnings, and files changed.
3. If warnings remain intentionally, record them as explicit backlog items with file list.

## Periodic Execution Template

Use this skill for weekly or release-bound governance runs:

1. `weekly`: full `docs/qa` lint + drift scan + README/manifest sync.
2. `pre-release`: run full governance loop after major refactor or route/auth changes.
3. `post-incident`: verify related QA/security/uiux docs and add regression notes.

## Resources

- Governance checklist: `references/policy-checklist.md`
- Audit script: `scripts/run-audit.sh`
