---
name: qa-doc-gen
description: "Generate QA test case documents from confirmed feature implementation plans. Use this skill AFTER a feature plan has been approved by the user (via plan mode or explicit confirmation). Converts the agreed feature behavior, acceptance criteria, and edge cases into structured QA test documents under docs/qa/. Triggers when (1) user says to generate QA docs after plan approval, (2) user asks to create test cases for a newly planned feature, (3) user asks to turn a feature plan into QA testing content."
---

# QA Doc Gen

After a feature plan is confirmed, generate QA test case documents that capture the feature's expected behavior for manual testing.

## Workflow

```
1. Extract feature details from the confirmed plan
2. Determine module classification and file naming
3. Generate QA test document(s) following project format
4. Update docs/qa/README.md index
```

## Step 1: Extract from Confirmed Plan

From the confirmed plan, extract:

- **Feature name**: What the feature is called
- **Module**: Which Auth9 module it belongs to (tenant, user, rbac, service, invitation, session, webhook, auth, settings, identity-provider, passkeys, analytics, audit, integration)
- **Behavior**: Normal flow, error cases, edge cases
- **UI interactions**: Pages, buttons, forms involved
- **API endpoints**: If applicable, include method, path, request/response
- **Database changes**: New tables/columns, expected data states
- **Acceptance criteria**: What constitutes correct behavior

## Step 2: Determine File Naming

### Module mapping

Place the QA doc under the matching `docs/qa/{module}/` directory. If the feature spans multiple modules, create separate documents per module.

### File numbering

Check existing files in the target directory:

```
Glob: docs/qa/{module}/*.md
```

Use the next available number: `{NN}-{descriptive-name}.md`

Example: If `docs/qa/tenant/` has `01-crud.md`, `02-list-settings.md`, `03-status-lifecycle.md`, the next file is `04-{name}.md`.

### Scenario count rule

Each document has **at most 5 numbered scenarios**. If a feature needs more than 5 scenarios, split into multiple documents.

## Step 3: Generate QA Document

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

## Step 4: Update README Index

After creating the QA document(s), update `docs/qa/README.md`:

1. Add the new document to its module's index table
2. Update the module's document count and scenario count
3. Update the 统计概览 table totals
4. Update the 更新日志 with today's date and a brief description
