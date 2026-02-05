---
name: qa-testing
description: Execute manual QA testing for Auth9 features using browser automation. Automatically discovers QA documents in docs/qa/ organized by modules (tenant, user, rbac, service, invitation, session, webhook, auth), verifies scenarios with browser tests, checks Docker logs on errors, validates database state, creates individual ticket files in docs/ticket/ for each failed scenario. Use when the user asks to run QA tests, manual testing, verify feature functionality, or test specific modules.
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
   d. If FAIL → Create ticket file in docs/ticket/
4. Report test summary to user
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
mysql -h 127.0.0.1 -P 4000 -u root auth9

# Or execute single query
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "SELECT * FROM users WHERE email='test@example.com';"
```

3. **Compare actual vs expected**:
   - Count mismatches (e.g., expected 1 row, got 0)
   - Value mismatches (e.g., expected status='active', got status='pending')
   - Extra records (e.g., orphaned foreign key references)
   - Missing records (e.g., expected audit log entry)

4. **Record validation result**:
   - ✅ PASS: Data matches expected state
   - ❌ FAIL: Data mismatch (document differences)

## Step 4: Create Ticket for Failed Scenarios

When a scenario fails, immediately create a ticket file in `docs/ticket/` with detailed information.

### 4.1 Ticket File Naming

**Format**: `{module}_{document}_scenario{N}_{YYMMDD_HHMMSS}.md`

**Examples**:
- Testing `docs/qa/user/01-crud.md` scenario 2 failed → `docs/ticket/user_01-crud_scenario2_260203_143052.md`
- Testing `docs/qa/tenant/01-crud.md` scenario 5 failed → `docs/ticket/tenant_01-crud_scenario5_260203_143125.md`
- Testing `docs/qa/rbac/02-role.md` scenario 3 failed → `docs/ticket/rbac_02-role_scenario3_260203_143201.md`

**File path pattern**:
```
docs/ticket/{module}_{document}_scenario{N}_{YYMMDD_HHMMSS}.md
```

### 4.2 Ticket Structure

```markdown
# Ticket: {Scenario Title}

**Created**: {YYYY-MM-DD HH:mm:ss}
**QA Document**: `docs/qa/{module}/{document}.md`
**Scenario**: #{number}
**Status**: FAILED

---

## 测试内容

{Brief description of what was being tested}

**Test Location**: {UI path or API endpoint}
**Test Type**: {UI/API/Integration}

---

## 预期结果

{Expected outcome from QA document}

**Expected UI State**:
- {Expected UI elements or messages}

**Expected Database State**:
```sql
{SQL queries showing expected data state}
```

**Expected Results**:
- {List of expected outcomes}

---

## 再现方法

### Prerequisites
{Initial state requirements or test data setup}

### Steps to Reproduce
1. {Step 1 with specific details}
2. {Step 2 with specific details}
3. {Step 3 with specific details}
...

### Environment
- **Portal**: http://localhost:3000
- **Auth9 Core**: http://localhost:8080
- **Keycloak**: http://localhost:8081
- **Test User**: {username/email used}

---

## 实际结果

### Test Execution Failed at Step {N}

**UI Error**:
```
{Error message or unexpected UI state}
```

**Browser Snapshot**:
{Relevant UI state information}

**Database State**:
```sql
-- Actual query results
{SQL query}

-- Results:
{Actual data found}
```

**Data Mismatch**:
- Expected: {expected value}
- Actual: {actual value}
- Difference: {explanation}

### Service Logs

**auth9-core logs**:
```
{Relevant log lines from Docker container}
```

**auth9-portal logs**:
```
{Relevant log lines if applicable}
```

**Keycloak logs**:
```
{Relevant log lines if applicable}
```

---

## Analysis

**Root Cause**: {Brief analysis of what went wrong}

**Severity**: High / Medium / Low

**Impact**:
- {User impact description}
- {System impact description}

**Related Components**:
- [ ] Frontend (auth9-portal)
- [ ] Backend API (auth9-core)
- [ ] Database (TiDB)
- [ ] Keycloak
- [ ] Cache (Redis)

---

*Ticket generated by QA Testing Skill*
```

### 4.3 Create Ticket File

**CRITICAL**: Create a ticket file immediately when a scenario fails.

Steps:
1. Generate the complete ticket content with all error details
2. Ensure `docs/ticket/` directory exists (create if needed)
3. Save with proper filename: `{module}_{document}_scenario{N}_{YYMMDD_HHMMSS}.md`
4. Continue testing next scenario

### 4.4 Test Summary Report to User

After completing all scenarios, report summary to user (DO NOT save as file):

```markdown
✅ 测试完成！

📊 测试结果:
- 通过: 11/13 (84.6%)
- 失败: 2/13 
- 跳过: 0/13

🎫 创建的 Tickets:
1. docs/ticket/user_01-crud_scenario4_260203_143052.md
   - Scenario #4: Update user profile
   - Severity: High
   - Issue: Connection pool exhausted

2. docs/ticket/user_01-crud_scenario11_260203_143225.md
   - Scenario #11: Delete user with tenant associations
   - Severity: Medium
   - Issue: Keycloak sync failure

💡 下一步: 请查看 ticket 文件了解详细问题描述和再现方法
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

## Mailpit 邮件测试服务

Dev 环境提供 Mailpit 用于捕获所有发出的邮件（邀请、密码重置等）：
- Web UI: http://localhost:8025
- API: http://localhost:8025/api/v1/

### API 端点

| 端点 | 说明 |
|------|------|
| `GET /api/v1/messages` | 获取邮件列表 |
| `GET /api/v1/message/{id}` | 获取邮件详情（包含完整内容） |
| `GET /api/v1/search?query=to:user@example.com` | 搜索邮件 |
| `DELETE /api/v1/messages` | 清空所有邮件 |

### 使用示例

```bash
# 获取所有邮件
curl http://localhost:8025/api/v1/messages

# 搜索发送给特定用户的邮件
curl "http://localhost:8025/api/v1/search?query=to:test@example.com"

# 获取最新邮件的详情（jq 解析 JSON）
curl -s http://localhost:8025/api/v1/messages | jq '.messages[0]'

# 清空所有邮件（测试前重置）
curl -X DELETE http://localhost:8025/api/v1/messages
```

### 邀请测试流程

1. 清空 Mailpit 邮件: `curl -X DELETE http://localhost:8025/api/v1/messages`
2. 通过 Portal 发送邀请
3. 查询 Mailpit 获取邀请邮件: `curl http://localhost:8025/api/v1/messages`
4. 从邮件内容提取邀请链接
5. 访问邀请链接完成测试

## Example Usage

**User request**: "按照QA文档进行用户管理测试"

**Agent response**:
1. Confirm: "请确认要测试 user-management.md 吗?"
2. Read document
3. List 13 scenarios
4. Execute each scenario with browser + DB validation
5. Generate test report with pass/fail counts

## MFA 自动化测试（TOTP）

MFA 测试需要程序化生成 TOTP 验证码。使用 `zbarimg` 和 `oathtool` 命令行工具。

### 工具使用

**1. 截取 QR 码图片**

使用浏览器自动化工具截取 Keycloak TOTP 配置页面的 QR 码，保存为 PNG 文件。

**2. 解析 QR 码获取 TOTP Secret**

```bash
# 解析 QR 码获取 otpauth:// URL
zbarimg --raw totp-qr.png
# 输出: otpauth://totp/auth9:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=auth9

# 提取 secret 部分
SECRET=$(zbarimg --raw totp-qr.png | sed -n 's/.*secret=\([^&]*\).*/\1/p')
```

**3. 生成 TOTP 验证码**

```bash
# 使用 secret 生成 6 位验证码
oathtool --totp -b "$SECRET"
# 输出: 123456
```

### 测试流程

1. 通过 Auth9 API 为用户启用 MFA：`POST /api/users/{id}/mfa/enable`
2. 用户登录时 Keycloak 显示 TOTP 配置页面（含 QR 码）
3. 截取 QR 码图片
4. 使用 `zbarimg` 解析获取 secret
5. 使用 `oathtool` 生成验证码
6. 填入验证码完成 MFA 设置/验证

### 相关 API

| 操作 | 方法 | 端点 |
|------|------|------|
| 启用 MFA | POST | `/api/users/{id}/mfa/enable` |
| 禁用 MFA | POST | `/api/users/{id}/mfa/disable` |

---

## Token Exchange gRPC 测试

Token Exchange 使用 gRPC 协议。使用 `grpcurl` 命令行工具进行测试。

### gRPC 服务信息

- **端口**: 50051
- **服务**: `auth9.TokenExchange`
- **方法**: `ExchangeToken`, `ValidateToken`, `IntrospectToken`, `GetUserRoles`

### 工具使用

**列出服务方法**

```bash
grpcurl -plaintext localhost:50051 list auth9.TokenExchange
```

**ExchangeToken - 获取租户访问令牌**

```bash
grpcurl -plaintext \
  -d '{
    "identity_token": "<IDENTITY_TOKEN>",
    "tenant_id": "<TENANT_ID>",
    "service_id": "<SERVICE_ID>"
  }' \
  localhost:50051 auth9.TokenExchange/ExchangeToken
```

**ValidateToken - 验证令牌**

```bash
grpcurl -plaintext \
  -d '{
    "access_token": "<ACCESS_TOKEN>",
    "audience": "<SERVICE_ID>"
  }' \
  localhost:50051 auth9.TokenExchange/ValidateToken
```

**IntrospectToken - 令牌内省**

```bash
grpcurl -plaintext \
  -d '{
    "token": "<ACCESS_TOKEN>"
  }' \
  localhost:50051 auth9.TokenExchange/IntrospectToken
```

### 获取测试数据

```bash
# 查询租户 ID
mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants LIMIT 1;"

# 查询服务 ID
mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM services WHERE tenant_id = '<TENANT_ID>' LIMIT 1;"
```

### Identity Token 获取方式

1. 通过浏览器登录后从 localStorage/cookies 中提取
2. 使用测试用的 JwtManager 生成（参考: `auth9-core/tests/grpc/exchange_token_test.rs`）
