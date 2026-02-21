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
- **UI entry points**: Navigation links, Quick Links, sidebar items, or buttons that lead to the feature (where users discover and access the feature)
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

### UI 可见性规则（功能追加/更新/重构时必须遵循）

当变更涉及 UI 部分时（新功能入口、页面跳转变化、导航结构调整等），测试文档**必须**包含用户可见性验证场景。禁止仅通过直接输入 URL 来测试页面——用户无法发现的功能等于不存在。

具体要求：

1. **新功能入口**：如果新增了页面/功能，必须有场景验证用户可以从现有 UI 导航到达该页面（侧边栏、Quick Links、按钮、菜单项等），而非直接访问 URL
2. **跳转逻辑变化**：如果修改了页面跳转目标、重定向路径、或导航结构，必须有场景验证新的跳转路径正确，且旧入口（如有保留）仍可达
3. **入口信息准确性**：入口处显示的计数、状态、徽章等摘要信息必须与目标页面的实际数据一致
4. **Portal UI 测试流程的导航起点**：后续 CRUD 等操作场景中，Portal UI 流程应从用户可见入口导航进入目标页面，而非直接输入 URL

示例：为 Tenant 下新增 Actions 功能时，第一个场景应验证 Tenant 详情页 Quick Links 中出现「Actions」入口（图标、计数、跳转），后续创建/编辑场景的 Portal UI 流程应从该入口导航进入。

### Scenario design guidelines

For each feature behavior in the plan, generate scenarios covering:

| Type | Example |
|------|---------|
| UI entry point | Verify navigation link/button exists and leads to the feature page |
| Normal flow | Create a resource with valid data |
| Duplicate/conflict | Create with existing unique key |
| Invalid input | Missing required fields, bad format |
| Permission | Unauthorized user attempts action |
| Boundary | Max length, empty string, special chars |
| Cascade effects | Delete with dependent data |

Not every type is needed for every feature - select the relevant ones. **UI entry point** is mandatory when the change adds or modifies a UI-accessible feature.

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
