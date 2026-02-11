---
name: qa-testing
description: Execute scenario-based QA testing with browser automation, database validation, and automatic ticket creation for failures.
---

# QA Testing Skill

Execute scenario-based manual QA testing for Auth9 using Playwright browser automation with Docker environment validation.

## Prerequisites

1. **Docker services running**: auth9-core, auth9-portal, auth9-keycloak, auth9-tidb, auth9-redis
2. **Service URLs**: Portal (3000), Auth9 Core (8080), Keycloak (8081)
3. **Credentials**: Portal Admin `admin / Admin123!`, Keycloak Admin `admin / admin`

## API Token Generation (IMPORTANT - Read First)

QA testing against auth9-core API requires a Bearer token. **Do NOT explore the codebase to figure out how to get a token.** Use the helper tools below directly.

### Quick Token Generation

```bash
# Generate admin JWT token (valid 1 hour, RS256 signed)
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# Use in curl requests
curl -s http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer $TOKEN"
```

### API Test Helper Script

For repeated API calls, use the wrapper script that auto-injects the token:

```bash
# GET request
.claude/skills/tools/qa-api-test.sh GET /api/v1/tenants

# POST with JSON body
.claude/skills/tools/qa-api-test.sh POST /api/v1/users '{"email":"test@example.com","password":"Pass123!"}'

# PUT with JSON body
.claude/skills/tools/qa-api-test.sh PUT /api/v1/tenants/{id}/password-policy '{"min_length":12}'
```

### Token Details

- **Subject**: `746ceba8-3ddf-4a8b-b021-a1337b7a1a35` (admin@auth9.local)
- **Algorithm**: RS256 (private key: `.claude/skills/tools/jwt_private_clean.key`)
- **Issuer**: `http://localhost:8080`
- **TTL**: 1 hour
- **Dependency**: `node` + `jsonwebtoken` npm package (already in project root `gen_token.js`)

### Concurrent / Load Testing

Use `hey` for concurrent request testing:

```bash
hey -n 20 -c 20 -m POST \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com"}' \
  http://localhost:8080/api/v1/auth/forgot-password
```

Note: Rate limiting is active on some endpoints (e.g., forgot-password: 5 req/min, token: 10 req/min).

## Workflow

**IMPORTANT: This skill is strictly for testing and reporting. NEVER attempt to fix, patch, or modify any source code during QA testing. If a test fails, create a ticket and move on to the next scenario.**

```
1. Confirm QA document with user
2. List all test scenarios
3. For each scenario:
   a. Execute test in browser
   b. If error → Check Docker logs
   c. Validate database state
   d. If FAIL → Create ticket in docs/ticket/
4. Report summary to user
```

## Step 1: Discover and Confirm Test Document

**CRITICAL**: Always confirm with user which QA document to test.

### 1.1 Discover Documents

```
Glob: docs/qa/**/*.md
```

Modules: `tenant/`, `user/`, `rbac/`, `service/`, `invitation/`, `session/`, `webhook/`, `auth/`

Exclude `docs/qa/README.md` (just an index).

### 1.2 Determine User Intent

- **Specific request**: Match against documents, confirm, proceed
- **Vague request**: Read `docs/qa/README.md`, list modules, ask user to choose

## Step 2: Parse QA Document

Extract from confirmed document:
- Database schema reference
- Test scenarios (numbered)
- Test data preparation SQL

## Step 3: Execute Each Scenario

### 3.1 Pre-execution

1. Read scenario details (initial state, steps, expected results, expected data state)
2. Prepare test data if required

### 3.2 Browser Execution (Playwright MCP)

Use `mcp__plugin_playwright_playwright__*` tools:

```
1. Navigate: browser_navigate to http://localhost:3000
2. Snapshot: browser_snapshot to get page structure and element refs
3. Login (if needed):
   - browser_type or browser_fill_form for credentials
   - browser_click on sign in button
4. Execute test steps:
   - browser_snapshot before each interaction
   - browser_click, browser_type for interactions
   - browser_wait_for after actions (1-3s)
   - browser_snapshot to verify results
5. Verify expected UI state
```

**Rules**:
- Always `browser_snapshot` before interactions to get element refs
- Use short incremental waits (1-3s)
- Check for errors in snapshot responses

### 3.3 Error Handling

If step fails:

1. Capture error from browser snapshot
2. Check Docker logs:
```bash
docker logs auth9-core --tail 50
docker logs auth9-portal --tail 50
docker logs auth9-keycloak --tail 50
```
3. Record: scenario, step, error message, logs, timestamp

### 3.4 Database Validation

After each scenario:

```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e "SELECT ..."
```

Compare actual vs expected:
- ✅ PASS: Data matches
- ❌ FAIL: Data mismatch → create ticket

## Step 4: Create Ticket for Failures

### 4.1 Ticket Naming

**Format**: `{module}_{document}_scenario{N}_{YYMMDD_HHMMSS}.md`

**Example**: `docs/ticket/user_01-crud_scenario2_260203_143052.md`

### 4.2 Ticket Structure

```markdown
# Ticket: {Scenario Title}

**Created**: {YYYY-MM-DD HH:mm:ss}
**QA Document**: `docs/qa/{module}/{document}.md`
**Scenario**: #{number}
**Status**: FAILED

---

## 测试内容
{Brief description}
**Test Location**: {UI path or API endpoint}

---

## 预期结果
{Expected outcome}

**Expected Database State**:
```sql
{SQL queries}
```

---

## 再现方法

### Prerequisites
{Initial state requirements}

### Steps to Reproduce
1. {Step 1}
2. {Step 2}
...

### Environment
- Portal: http://localhost:3000
- Auth9 Core: http://localhost:8080
- Keycloak: http://localhost:8081

---

## 实际结果

**UI Error**:
```
{Error message}
```

**Database State**:
```sql
{Actual data}
```

**Data Mismatch**:
- Expected: {value}
- Actual: {value}

### Service Logs
```
{Relevant log lines}
```

---

## Analysis

**Root Cause**: {Analysis}
**Severity**: High / Medium / Low
**Related Components**: Frontend / Backend / Database / Keycloak / Redis

---
*Ticket generated by QA Testing Skill*
```

### 4.3 Test Summary (report to user, don't save)

```markdown
✅ 测试完成！

📊 测试结果:
- 通过: 11/13 (84.6%)
- 失败: 2/13

🎫 创建的 Tickets:
1. docs/ticket/user_01-crud_scenario4_260203_143052.md
   - Scenario #4: Update user profile
   - Severity: High
```

## Common Database Queries

```sql
-- User
SELECT id, email, display_name, mfa_enabled FROM users WHERE email = 'test@example.com';
SELECT tu.*, t.name FROM tenant_users tu JOIN tenants t ON t.id = tu.tenant_id WHERE tu.user_id = '{id}';

-- Tenant
SELECT id, name, slug, status FROM tenants WHERE slug = 'test-tenant';
SELECT * FROM services WHERE tenant_id = '{id}';

-- RBAC
SELECT r.name, utr.* FROM user_tenant_roles utr JOIN roles r ON r.id = utr.role_id WHERE utr.tenant_user_id = '{id}';
SELECT p.* FROM permissions p JOIN role_permissions rp ON rp.permission_id = p.id WHERE rp.role_id = '{id}';
```

## Testing Tips

1. Don't skip scenarios - later ones may depend on earlier state
2. Always snapshot before clicking
3. Use 1-3s waits, check snapshot, wait more if needed
4. Validate data after EVERY scenario
5. Reset environment with `reset-local-env` skill if needed

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Browser fails | `curl http://localhost:3000`, check auth9-portal logs |
| DB connection fails | `docker ps \| grep tidb`, reconnect |
| Services not responding | `docker ps`, restart services, reset env |

## Mailpit (Email Testing)

- Web UI: http://localhost:8025
- API: http://localhost:8025/api/v1/

```bash
# Get messages
curl http://localhost:8025/api/v1/messages

# Search
curl "http://localhost:8025/api/v1/search?query=to:test@example.com"

# Clear all
curl -X DELETE http://localhost:8025/api/v1/messages
```

**Invitation flow**: Clear mailpit → Send invitation → Get email → Extract link → Complete test

## MFA Testing (TOTP)

```bash
# Parse QR code
zbarimg --raw totp-qr.png
# Output: otpauth://totp/auth9:user@example.com?secret=JBSWY3DPEHPK3PXP&issuer=auth9

# Extract secret
SECRET=$(zbarimg --raw totp-qr.png | sed -n 's/.*secret=\([^&]*\).*/\1/p')

# Generate TOTP code
oathtool --totp -b "$SECRET"
```

**API**: `POST /api/users/{id}/mfa/enable`, `POST /api/users/{id}/mfa/disable`

## Token Exchange gRPC Testing

- **Port**: 50051
- **Service**: `auth9.TokenExchange`

```bash
# List methods
grpcurl -plaintext localhost:50051 list auth9.TokenExchange

# Exchange token
grpcurl -plaintext -d '{"identity_token":"<TOKEN>","tenant_id":"<ID>","service_id":"<ID>"}' \
  localhost:50051 auth9.TokenExchange/ExchangeToken

# Validate token
grpcurl -plaintext -d '{"access_token":"<TOKEN>","audience":"<SERVICE_ID>"}' \
  localhost:50051 auth9.TokenExchange/ValidateToken

# Introspect token
grpcurl -plaintext -d '{"token":"<TOKEN>"}' \
  localhost:50051 auth9.TokenExchange/IntrospectToken
```

**Get test data**:
```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants LIMIT 1;"
mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM services WHERE tenant_id = '<ID>' LIMIT 1;"
```
