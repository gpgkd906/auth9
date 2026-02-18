---
name: qa-doc-gen
description: "Generate or update QA/security/UIUX test documentation after confirmed feature implementation plans or completed refactors. Use this skill AFTER plan approval or code completion to: (1) add new QA/security test docs for new behavior, and (2) run cross-doc impact analysis across docs/qa/, docs/security/, and docs/uiux/ to update stale steps, expectations, and security assertions. Triggers when users ask to create QA docs, update test docs after implementation, or sync QA/security/UIUX docs after behavior changes."
---

# QA Doc Gen

After a feature plan is confirmed or a refactor is completed, generate and synchronize test documentation so all QA/security/UIUX docs match real behavior.

## Workflow

```
1. Extract behavior changes from confirmed plan and implemented code
2. Generate/update QA and security test documents for new behavior
3. Run cross-doc impact scan on docs/qa, docs/security, docs/uiux
4. Patch all impacted existing docs to remove stale steps/assertions
5. Update README/index logs and report impact summary
```

## Step 1: Extract Change Set

From the confirmed plan and merged implementation, extract:

- **Feature name**: What the feature is called
- **Module**: Which Auth9 module it belongs to (tenant, user, rbac, service, invitation, session, webhook, auth, settings, identity-provider, passkeys, analytics, audit, integration)
- **Behavior**: Normal flow, error cases, edge cases
- **Behavior deltas**: What changed compared to old docs (redirects, auth rules, token types, permission boundaries, UI routes, API contracts)
- **UI interactions**: Pages, buttons, forms involved
- **API endpoints**: If applicable, include method, path, request/response
- **Database changes**: New tables/columns, expected data states
- **Acceptance criteria**: What constitutes correct behavior

## Step 2: Generate or Update New Docs

If the change introduces a new behavior surface, create or extend the primary QA/security doc first.

### QA file naming

Place QA docs under `docs/qa/{module}/` and follow numbering rules.

### Security doc update targets

Update existing security suites under `docs/security/` when behavior affects threat model, abuse path, authz/authn boundary, token lifecycle, or transport/security controls.

### Scenario count rule

Each QA/security doc has **at most 5 numbered scenarios**. Split documents when needed.

## Step 3: Write New/Changed QA Docs

Read `references/qa-doc-template.md` for the exact format template.

### Content generation rules

1. **Language**: All descriptive text in Chinese. Technical terms (SQL, API paths, field names) in English.
2. **Scenarios must cover**: Happy path + error/rejection cases + boundary conditions
3. **UI button/menu names**: Use Chinese book title marks e.g. 「创建」「保存」
4. **Dynamic values**: Use `{placeholder}` syntax in SQL and curl commands
5. **curl examples**: Provide complete commands for API-tested scenarios
6. **SQL verification**: Every scenario with data mutations needs a 预期数据状态 section with verification SQL
7. **Checklist**: End with a checklist table listing all scenarios (including any 通用场景)

### Scenario design guidelines

For each feature behavior in the plan, generate scenarios covering:

| Type | Example |
|------|---------|
| Normal flow | Create a resource with valid data |
| Duplicate/conflict | Create with existing unique key |
| Invalid input | Missing required fields, bad format |
| Permission | Unauthorized user attempts action |
| Boundary | Max length, empty string, special chars |
| Cascade effects | Delete with dependent data |

Not every type is needed for every feature - select the relevant ones.

## Step 4: Run Cross-Doc Impact Analysis (Mandatory)

Always scan and classify potential impacts in:

- `docs/qa/**/*.md`
- `docs/security/**/*.md`
- `docs/uiux/**/*.md`

Use search patterns based on behavior deltas, for example:

- route changes (`/dashboard`, `/tenant/select`, `/auth/callback`)
- token model changes (`id_token`, `tenant token`, `token exchange`)
- permission/auth changes (`401`, `403`, `scope`, `audience`, `role`)
- UI navigation text and expected redirect paths

For each impacted document, do one of:

1. **Patch required**: update steps/expected results/security assertions/UI flow
2. **Note required**: add prerequisite note or branch-path note to avoid tester confusion
3. **No change**: explicitly record why unaffected

Never stop at creating only new docs when old docs are stale.

## Step 5: Update Indexes and Changelog

After edits, update affected indexes/changelogs:

1. `docs/qa/README.md` (new docs and/or cross-doc alignment log)
2. `docs/security/README.md` when security docs changed
3. `docs/uiux/README.md` when UIUX docs changed

## Output Requirements

In the final response, always include:

1. New docs created/updated for the feature itself
2. Cross-doc impact list grouped by `qa/security/uiux`
3. Updated files and rationale per file
4. Remaining docs reviewed but unchanged (with reason)
