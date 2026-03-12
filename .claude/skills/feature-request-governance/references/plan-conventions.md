# Plan Conventions for Feature Request Governance

## Plan Structure

A governance plan must include:

1. **Context** — One paragraph summarizing the FR and why it exists
2. **FR breakdown** — If the FR contains multiple independent changes, split into sub-FRs (FR1, FR2, ...) that can be parallelized
3. **Per-FR details**:
   - Changed files table (file path + operation: new/edit/delete)
   - Implementation approach with code-level specifics
   - Key decisions and trade-offs (with rationale)
4. **Verification steps** — Per-FR commands to validate (typecheck, test, build, manual checks)
5. **Governance closure** — Post-implementation: delete FR doc if fully closed, or update FR doc to reflect remaining work

## File Change Table Format

```markdown
| File | Operation |
|------|-----------|
| `path/to/file.tsx` | Edit |
| `path/to/new-file.tsx` | New |
| `path/to/obsolete.ts` | Delete |
```

## Implementation Detail Level

Provide enough detail that implementation can proceed without further clarification:
- Component structure (JSX tree for UI changes)
- Function signatures and key logic for backend changes
- Database schema (CREATE TABLE / ALTER TABLE) for data changes
- Translation key structure for i18n changes

## Verification Commands

Always include copy-paste-ready verification:

```bash
# Type check
npm run typecheck  # or cargo clippy

# Unit tests
npm run test       # or cargo test

# Build
npm run build      # or cargo build

# Manual verification
npm run dev        # describe what to check manually
```

## Quality Checklist

Before finalizing a plan, verify:
- [ ] Every file in the change table actually needs to change
- [ ] No unnecessary abstractions or over-engineering
- [ ] Changes are minimal and focused on the FR scope
- [ ] Existing patterns in the codebase are followed (not reinvented)
- [ ] Test strategy covers happy path + error cases
