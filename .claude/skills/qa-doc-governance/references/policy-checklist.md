# QA Governance Checklist

## Mandatory rules

1. Scenario count per file: `<=5` (`## 场景 N`).
2. Checklist section required in every QA doc.
3. UI-facing docs must include at least one "入口可见性" scenario.
4. UI flow must start from visible entry points, not direct URL (except negative tests).
5. Auth/session negative checks must be executable:
   - incognito/private window, or
   - clear `auth9_session`, or
   - explicit sign out.
6. `docs/qa/README.md` index must match filesystem docs.
7. `docs/qa/_manifest.yaml` must reflect current docs and scenario counts.

## Recommended split strategy for long docs

1. `base + advanced` split.
2. or split by capability (`api`, `ui`, `security`, `regression`).
3. Preserve scenario numbering meaning with explicit migration note in original file.

## Report format

1. Findings by severity: `P0/P1/P2`.
2. Fixed items list with file paths.
3. Remaining backlog with rationale.
4. Final validation output (`qa-doc-lint: PASSED`).
