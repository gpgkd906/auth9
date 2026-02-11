---
name: ticket-fix
description: Fix QA tickets by reading docs/ticket/*.md, verifying the issue against real implementation, applying fixes when needed, resetting environment, re-running QA steps from the ticket, and deleting the ticket file once verified. Use when a user asks to fix a ticket or resolve a QA failure.
---

# Ticket Fix

Resolve QA tickets end-to-end: read the ticket, confirm the issue, fix or dismiss, reset environment, re-run QA steps, and clean up the ticket on success.

## Workflow

1. **Discover tickets**
   - List `docs/ticket/*.md` and confirm which ticket(s) to handle if not specified.

2. **Read ticket and validate**
   - Parse: scenario, steps, expected/actual, environment, SQL checks.
   - Reproduce quickly against current implementation.
   - If issue no longer reproducible, document why and still run QA verification.

3. **Verify issue authenticity (NEW - AGENT VALIDATION)**
   - Thoroughly reproduce the issue against current implementation
   - Check codebase to confirm the root cause exists
   - Validate with database queries, logs, and test cases
   - **Only proceed with fix if agent confirms issue is real**
   - If issue cannot be reproduced or root cause not found, diagnose and report why
   - Document evidence clearly before proceeding to fix

4. **Fix if needed**
   - If confirmed, implement code fix.
   - Keep change scope minimal and aligned to ticket.

5. **Reset environment (always)**
   - Run `./scripts/reset-docker.sh` regardless of fix/no-fix.

6. **Re-run QA steps**
   - Follow the ticket's steps and SQL validation.
   - Capture evidence (log snippets, DB results).

7. **Close ticket**
   - If verified, delete the ticket file from `docs/ticket/`.
   - Summarize fix + verification in response.

## Rules

- **Agent must verify authenticity**: Always reproduce and validate issue before fixing
- Thoroughly test the issue scenario step-by-step to confirm reproducibility
- Examine code to locate the root cause and confirm it exists
- Document verification evidence (logs, query results, test output)
- **Only fix if issue is confirmed real** - do not fix suspected/unconfirmed issues
- If issue cannot be reproduced, explain diagnosis clearly but do not apply fixes
- Always reset environment before final verification
- Do not delete ticket unless QA re-test passes
- If re-test fails, keep the ticket and report remaining issue
- Avoid false positive fixes by strict verification before any code changes

## Notes

- Use `docs/qa/` for any referenced test cases.
- Use Docker logs and DB queries from the ticket for evidence.
