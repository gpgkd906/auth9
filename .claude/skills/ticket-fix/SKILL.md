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

3. **Fix if needed**
   - If confirmed, implement code fix.
   - Keep change scope minimal and aligned to ticket.

4. **Reset environment (always)**
   - Run `./scripts/reset-docker.sh` regardless of fix/no-fix.

5. **Re-run QA steps**
   - Follow the ticket’s steps and SQL validation.
   - Capture evidence (log snippets, DB results).

6. **Close ticket**
   - If verified, delete the ticket file from `docs/ticket/`.
   - Summarize fix + verification in response.

## Rules

- Always reset environment before final verification.
- Do not delete ticket unless QA re-test passes.
- If issue cannot be reproduced, explain why and still re-test.
- If re-test fails, keep the ticket and report remaining issue.

## Notes

- Use `docs/qa/` for any referenced test cases.
- Use Docker logs and DB queries from the ticket for evidence.
