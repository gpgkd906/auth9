---
name: qa-testing
description: Execute manual QA testing for Auth9 features using browser automation. Automatically discovers QA documents in docs/qa/ organized by modules (tenant, user, rbac, service, invitation, session, webhook, auth), verifies scenarios with browser tests, checks Docker logs on errors, validates database state, saves detailed test reports to docs/report/. Use when the user asks to run QA tests, manual testing, verify feature functionality, or test specific modules.
---

# QA Testing Skill

Execute scenario-based manual QA testing for Auth9 using browser automation with Docker environment validation.

## Prerequisites

Before starting QA tests, verify:

1. **Docker environment is running** with all services healthy:
   - auth9-core (backend API)
   - auth9-portal (frontend)
   - auth9-keycloak (OIDC provider)
   - auth9-tidb (database)
   - auth9-redis (cache)

2. **Service URLs are accessible**:
   - Portal: http://localhost:3000
   - Auth9 Core: http://localhost:8080
   - Keycloak: http://localhost:8081

3. **Initial credentials are known**:
   - Portal Admin: admin / Admin123!
   - Keycloak Admin: admin / admin

## Workflow Overview

```
1. Confirm QA document with user
2. List all test scenarios
3. For each scenario:
   a. Execute test in browser
   b. If error occurs → Check Docker logs
   c. Validate database state
   d. Record result
4. Generate test report
```

## Step 1: Discover and Confirm Test Document

**CRITICAL**: Always confirm with the user which QA document to test before starting.

### 1.1 Discover Available QA Documents

Use the Glob tool to find all QA documents:

```
Glob: docs/qa/**/*.md
```

This will return all available QA test documents in the `docs/qa/` directory, organized by modules:
- `tenant/` - Tenant management
- `user/` - User management  
- `rbac/` - Role-based access control
- `service/` - Service and client management
- `invitation/` - Invitation management
- `session/` - Session and security
- `webhook/` - Webhook management
- `auth/` - Authentication flows

Exclude `docs/qa/README.md` from the document list (it's just an index).

### 1.2 Determine User Intent

**If user request is specific** (mentions a document name, module, or topic):
- Match user's request against discovered documents
- If multiple documents match (e.g., "测试用户管理" matches user/01-crud.md, user/02-advanced.md, user/03-validation.md), list all matches
- Confirm the matched document(s) with user
- Proceed to Step 2

**If user request is vague** (just says "run QA tests" or "test the system"):
- Read `docs/qa/README.md` to get module structure
- List all modules with document counts
- Ask user to choose which module or specific document to test

### 1.3 Confirmation Pattern

**Specific request example (single document)**:
```
User: "按照QA文档测试用户CRUD @user/01-crud.md"

Agent:
✅ 已发现QA文档: docs/qa/user/01-crud.md
📋 文档标题: [从文档第一行提取]
🔢 场景数: 5

确认开始测试?
```

**Specific request example (module with multiple documents)**:
```
User: "测试用户管理模块"

Agent:
📂 已发现用户管理模块的QA文档:

1. user/01-crud.md - 创建、更新、租户关联 (5 个场景)
2. user/02-advanced.md - 删除、MFA、列表 (5 个场景)
3. user/03-validation.md - 边界测试、验证 (3 个场景)

请选择:
a) 测试全部 3 个文档
b) 选择特定文档 (输入编号)
```

**Vague request example**:
```
User: "进行QA测试"

Agent:
📂 Auth9 QA测试模块概览:

1. 租户管理 (tenant/) - 2 个文档, 10 个场景
2. 用户管理 (user/) - 3 个文档, 13 个场景
3. RBAC (rbac/) - 4 个文档, 17 个场景
4. 服务与客户端 (service/) - 3 个文档, 15 个场景
5. 邀请管理 (invitation/) - 3 个文档, 15 个场景
6. 会话与安全 (session/) - 4 个文档, 20 个场景
7. Webhook (webhook/) - 4 个文档, 17 个场景
8. 认证流程 (auth/) - 5 个文档, 23 个场景

总计: 28 个文档, 130 个场景

请选择要测试的模块或文档:
```

## Step 2: Read and Parse QA Document

Read the confirmed document and extract:
- **Database schema reference** (for validation queries)
- **Test scenarios** (numbered scenarios with sections)
- **Test data preparation SQL** (if available)

Create a scenario checklist:
```markdown
## Test Scenarios for [Document Name]
- [ ] Scenario 1: [Title]
- [ ] Scenario 2: [Title]
...
```

## Step 3: Execute Each Scenario

For each scenario, follow this pattern:

### 3.1 Pre-execution

1. **Read scenario details**:
   - Initial state requirements
   - Purpose of the test
   - Test operation steps
   - Expected results
   - Expected data state (SQL queries)

2. **Prepare test data** (if required):
   - Run preparation SQL in TiDB
   - Verify initial state

### 3.2 Browser Execution

Use the `cursor-ide-browser` MCP tools to execute UI tests:

```markdown
**Browser Test Pattern**:

1. Lock browser: browser_lock (only if tab already exists)
2. Navigate: browser_navigate to http://localhost:3000
3. Snapshot: browser_snapshot to get page structure
4. Login (if not logged in):
   - Fill username: admin
   - Fill password: Admin123!
   - Click sign in
5. Execute test steps:
   - Use browser_snapshot before each interaction
   - Use browser_click, browser_type, browser_fill
   - Wait after actions: browser_wait (1-3s incremental waits)
   - Snapshot after each action to verify result
6. Verify expected results in UI
7. Unlock browser: browser_unlock when done
```

**Important Browser Rules**:
- **Always call browser_snapshot** before interactions to get element refs
- **Use short incremental waits** (1-3s) instead of long waits
- **Check for errors** in snapshot responses
- **Never lock before navigate** - lock requires existing tab

### 3.3 Error Handling

If any step fails or shows unexpected UI state:

1. **Capture error details** from browser snapshot
2. **Check Docker logs** for the relevant service:

```bash
# Check auth9-core logs (backend API)
docker logs auth9-core --tail 50

# Check auth9-portal logs (frontend)
docker logs auth9-portal --tail 50

# Check Keycloak logs
docker logs auth9-keycloak --tail 50
```

3. **Record the error**:
   - Scenario number and name
   - Step that failed
   - Error message from UI
   - Relevant log lines from Docker
   - Timestamp

### 3.4 Database Validation

After each scenario (success or failure), validate database state:

1. **Extract validation SQL** from the scenario's "预期数据状态" section

2. **Execute queries using host mysql client**:

```bash
# Connect to TiDB from host
mysql -h 127.0.0.1 -P 4000 -u root auth9_db

# Or execute single query
mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e "SELECT * FROM users WHERE email='test@example.com';"
```

3. **Compare actual vs expected**:
   - Count mismatches (e.g., expected 1 row, got 0)
   - Value mismatches (e.g., expected status='active', got status='pending')
   - Extra records (e.g., orphaned foreign key references)
   - Missing records (e.g., expected audit log entry)

4. **Record validation result**:
   - ✅ PASS: Data matches expected state
   - ❌ FAIL: Data mismatch (document differences)

## Step 4: Generate and Save Test Report

After all scenarios, generate a comprehensive report and save it to `docs/report/`.

### 4.1 Report File Naming

**Format**: `{qa_document_name}_result_{YYMMDD}.md`

**Examples**:
- Testing `docs/qa/user/01-crud.md` → Save to `docs/report/user_01-crud_result_260202.md`
- Testing `docs/qa/tenant/01-crud.md` → Save to `docs/report/tenant_01-crud_result_260202.md`
- Testing `docs/qa/rbac/02-role.md` → Save to `docs/report/rbac_02-role_result_260202.md`

**File path pattern**:
```
docs/report/{module}_{document}_result_{YYMMDD}.md
```

### 4.2 Report Structure

```markdown
# QA Test Report: {Module} - {Document Title}

**Test Date**: {YYYY-MM-DD HH:mm:ss}
**QA Document**: `docs/qa/{module}/{document}.md`
**Environment**: Docker local (all services)
**Tester**: AI Agent
**Duration**: {total_time}

## Summary

| Status | Count |
|--------|-------|
| ✅ PASS | X |
| ❌ FAIL | Y |
| ⏭️ SKIP | Z |
| **Total** | N |

**Pass Rate**: {pass_rate}%

## Detailed Results

### Scenario 1: {Title}
**Status**: ✅ PASS / ❌ FAIL
**Duration**: Xs

**Test Steps**:
- [Step 1]: ✅ Success
- [Step 2]: ✅ Success

**Database Validation**: ✅ PASS
- users table: 1 record created as expected
- audit_logs: 1 entry with correct action

---

### Scenario 2: {Title}
**Status**: ❌ FAIL
**Duration**: Xs

**Test Steps**:
- [Step 1]: ✅ Success
- [Step 2]: ❌ Failed - Error: "Email already exists"

**Error Details**:
- UI Error: "Email already exists"
- Docker Logs (auth9-core):
  ```
  [2026-02-02 10:15:32] ERROR: Duplicate key violation: users.email
  ```

**Database Validation**: ❌ FAIL
- Expected: COUNT(*) = 1
- Actual: COUNT(*) = 2 (duplicate created)

---

## Issues Summary

### 🐛 Bug 1: {Brief Description}
**Scenario**: #{number}
**Severity**: High / Medium / Low
**Logs**: `{error message}`
**Recommendation**: {fix suggestion}

## Recommendations

{List of improvements, fixes needed, or test issues}

---

*Report generated by QA Testing Skill*
*Report saved to: `docs/report/{filename}`*
```

### 4.3 Save Report

**CRITICAL**: Always save the report to the `docs/report/` directory with the correct filename format.

Steps:
1. Generate the complete report content
2. Ensure `docs/report/` directory exists (create if needed)
3. Save with proper filename: `{module}_{document}_result_{YYMMDD}.md`
4. Confirm to user: "✅ 测试报告已保存到: docs/report/{filename}"

Example:
```markdown
✅ 测试完成！

📊 测试结果:
- 通过: 11/13 (84.6%)
- 失败: 2/13 
- 跳过: 0/13

📄 测试报告已保存到: docs/report/user_01-crud_result_260202.md

❌ 发现的问题:
1. Bug #4: Connection pool exhausted (高)
2. Bug #11: Keycloak sync failure (中)

💡 建议: 修复连接池配置后重新测试失败的场景
```

## Common Database Queries

### User Management
```sql
-- Check user exists
SELECT id, email, display_name, mfa_enabled FROM users WHERE email = 'test@example.com';

-- Check tenant_users association
SELECT tu.*, t.name FROM tenant_users tu 
JOIN tenants t ON t.id = tu.tenant_id 
WHERE tu.user_id = '{user_id}';

-- Check cascade deletion
SELECT COUNT(*) FROM tenant_users WHERE user_id = '{user_id}';
SELECT COUNT(*) FROM sessions WHERE user_id = '{user_id}';
```

### Tenant Management
```sql
-- Check tenant exists
SELECT id, name, slug, status FROM tenants WHERE slug = 'test-tenant';

-- Check tenant services
SELECT * FROM services WHERE tenant_id = '{tenant_id}';
```

### RBAC
```sql
-- Check role assignment
SELECT r.name, utr.* FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
WHERE utr.tenant_user_id = '{tenant_user_id}';

-- Check permissions
SELECT p.* FROM permissions p
JOIN role_permissions rp ON rp.permission_id = p.id
WHERE rp.role_id = '{role_id}';
```

## Tips for Effective Testing

1. **Test incrementally**: Don't skip scenarios - later scenarios may depend on earlier state
2. **Use browser snapshots**: Always snapshot before clicking to get correct element refs
3. **Short waits**: Use 1-3s waits after actions, check snapshot, wait more if needed
4. **Log errors immediately**: Don't wait until the end to check logs
5. **Validate data after EVERY scenario**: Even if UI looks correct, data might be wrong
6. **Reset environment**: If tests get into bad state, suggest user reset with reset-local-env skill

## Troubleshooting

### Browser automation fails
- Check if portal is accessible: `curl http://localhost:3000`
- Check auth9-portal logs: `docker logs auth9-portal`
- Try browser_navigate again with longer timeout

### Database connection fails
- Check TiDB is healthy: `docker ps | grep tidb`
- Try reconnecting to container

### Services not responding
- Check service health: `docker ps` (look for "healthy" status)
- Restart services: `docker-compose restart <service-name>`
- If persistent, suggest reset environment

## Example Usage

**User request**: "按照QA文档进行用户管理测试"

**Agent response**:
1. Confirm: "请确认要测试 user-management.md 吗?"
2. Read document
3. List 13 scenarios
4. Execute each scenario with browser + DB validation
5. Generate test report with pass/fail counts
