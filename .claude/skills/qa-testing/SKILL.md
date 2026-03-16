---
name: qa-testing
description: Execute scenario-based QA testing with browser automation, database validation, and automatic ticket creation for failures.
---

# QA Testing Skill

Execute scenario-based manual QA testing for Auth9 using Playwright browser automation with Docker environment validation.

## Prerequisites

1. **Docker services running**: auth9-core, auth9-portal, auth9-keycloak, auth9-tidb, auth9-redis
2. **Service URLs**: Portal (3000), Auth9 Core (8080), Keycloak (8081)
3. **Credentials**:
   - Portal Admin: use the account printed by `./scripts/reset-docker.sh` (avoid hard-coding in docs/skills)
   - Keycloak Admin: `admin / admin` (local default)

## Evidence Artifact Hygiene (MUST FOLLOW)

To prevent repository-root pollution during QA:

1. Create a dated evidence directory before testing:
```bash
DATE=$(date +%F)
EVIDENCE_DIR="artifacts/qa/${DATE}"
mkdir -p "$EVIDENCE_DIR"
```
2. Save all screenshots, dumps, exports, and temporary QA files under `$EVIDENCE_DIR` (or its subfolders).
3. **Never** write QA artifacts (`*.png`, `*.jpg`, `*.json`, ad-hoc logs) to repository root.
4. New helper scripts must live in `scripts/qa/`, not root.
5. If an artifact is referenced by a ticket, store it under:
   - `artifacts/qa/{date}/tickets/{ticket_filename_without_ext}/`
6. After QA run:
   - keep only evidence linked by active tickets;
   - move unrelated leftovers from root into `artifacts/qa/{date}/` immediately.

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
.claude/skills/tools/qa-api-test.sh POST /api/v1/users '{"email":"test@example.com","password":"Pass123!"}' # pragma: allowlist secret

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

## Test Domain Policy (MUST FOLLOW)

All test data (emails, URLs, domains) must comply with `docs/testing/test-domain-policy.md`. Summary:

- **Email addresses**: Only `@example.com`, `@test.com`, `@auth9.local`
- **Callback/redirect URLs**: Only `https://*.example.com/...`
- **Organization domains**: Only `*.example.com` (e.g. `acme.example.com`)
- **Security attack scenarios**: Only `evil.com`, `attacker.com`, `attacker.example`
- **When unsure**: Use `example.com`
- **NEVER** invent domains like `test-enterprise.com`, `acme.com`, `corp.com`, etc.

## Test Scripts Directory (`scripts/qa/`)

**IMPORTANT**: A collection of reusable QA test scripts already exists in `scripts/qa/`. Before writing any new test script, **always check `scripts/qa/` first** for an existing script that covers the same or similar scenario.

- **Reuse first**: Run `ls scripts/qa/` or `Glob: scripts/qa/*` to find existing scripts. If a matching script exists, use it directly (or adapt it) instead of creating a new one.
- **Create in `scripts/qa/`**: When a new test script is needed, always place it under `scripts/qa/` — never in the project root or other ad-hoc locations.
- **Naming convention**: Follow the existing patterns — e.g., `test-{feature}.{js,mjs,py,sh}` or `{feature}_test.py`.

## Workflow

**IMPORTANT: This skill is strictly for testing and reporting. NEVER attempt to fix, patch, or modify any source code during QA testing. If a test fails, create a ticket immediately and move on to the next scenario.**

```
1. Confirm QA document with user
2. List all test scenarios
3. For each scenario:
   a. Execute test in browser
   b. If error → Check Docker logs
   c. Validate database state
   d. If FAIL → Immediately create ticket in docs/ticket/ (DO NOT defer)
   e. Report scenario result (PASS/FAIL) before moving to next
4. Report final summary to user
5. Run workspace hygiene check (no QA artifacts in repository root)
```

**Ticket creation rule**: Create the ticket the moment a scenario is confirmed as FAIL — before starting the next scenario. This ensures no failure is lost if the session is interrupted, and gives the user real-time visibility into issues as they surface.

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
2. **Execute Gate Checks (步骤 0)**: If the scenario contains any「步骤 0」verification step, run it **before** proceeding. Common gates:
   - **Token 类型**: Decode token and verify `token_type` and `tenant_id` fields
   - **数据格式**: Run the REGEXP validation query to confirm no non-UUID `id` values
   - **环境状态**: Run the prerequisite check (curl/sql) to confirm required config exists
   - If a gate check fails, fix the prerequisite first (regenerate token, re-insert data with `UUID()`, configure missing service). **Do NOT skip gates and proceed** — this is the primary cause of false-positive tickets.
3. Prepare test data if required

### 3.2 Browser Execution (playwright-cli)

Use `playwright-cli` commands via Bash:

```bash
# 1. Open browser and navigate
playwright-cli open http://localhost:3000
# 2. Snapshot to get page structure and element refs
playwright-cli snapshot
# 3. Login (if needed):
playwright-cli fill e1 "admin"
playwright-cli fill e2 "SecurePass123!"
playwright-cli click e3
# 4. Execute test steps:
#    - snapshot before each interaction
playwright-cli snapshot
#    - click, type, fill for interactions
playwright-cli click e5
playwright-cli type "some text"
#    - snapshot to verify results
playwright-cli snapshot
# 5. Close browser when done
playwright-cli close
```

If screenshots are required for evidence, always save into `$EVIDENCE_DIR`:
```bash
# Example only; keep filenames scenario-oriented
playwright-cli screenshot "$EVIDENCE_DIR/scenario-01-login.png"
```

**Rules**:
- Always `playwright-cli snapshot` before interactions to get element refs
- Use snapshots (not screenshots) to verify UI state
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
- ✅ PASS: Data matches → report PASS to user, proceed to next scenario
- ❌ FAIL: Data mismatch → **immediately create ticket** (see 3.5), report FAIL to user, then proceed to next scenario

### 3.5 Immediate Ticket Creation on Failure

**CRITICAL: Create the ticket RIGHT NOW, before moving to the next scenario.** Do NOT accumulate failures for batch ticket creation later. Each failed scenario gets its own ticket written to `docs/ticket/` immediately upon confirmation of failure.

Workflow per failure:
1. Gather all evidence (error message, logs, DB state, screenshots)
2. Write ticket file to `docs/ticket/` using the naming and structure below
3. Inform user: "❌ Scenario #N FAIL — ticket created: `docs/ticket/{filename}.md`"
4. Only then proceed to the next scenario

This ensures:
- No failures are lost if the session is interrupted or context is compressed
- User has real-time visibility into each issue as it surfaces
- Ticket evidence is freshest at the moment of failure (logs, DB state haven't been polluted by subsequent tests)

When attaching evidence in ticket markdown, use paths under `artifacts/qa/{date}/...` only.

#### UIUX Visibility / Accessibility Ticket Rule

**For UIUX test documents**: In addition to functional failures, **visibility and accessibility issues MUST also result in ticket creation**. This applies even when the underlying functionality works correctly. Examples include:

- Elements not visible or hidden behind overlapping components
- Missing or incorrect ARIA labels / roles
- Insufficient color contrast ratios
- Elements not reachable via keyboard navigation (Tab / Enter / Escape)
- Focus management issues (e.g., focus not trapped in modals, focus lost after actions)
- Missing focus indicators on interactive elements
- Screen reader incompatible content or structure
- Text truncation or overflow that hides meaningful content
- Touch targets too small for mobile interaction
- Missing or misleading alt text on images

When creating a ticket for a UIUX visibility/accessibility issue, set:
- **Severity**: `Medium` (default) — raise to `High` if the issue blocks a user action or violates WCAG 2.1 Level A
- **Related Components**: `Frontend`
- **Analysis > Root Cause**: Clearly describe the visibility or accessibility deficiency

#### Ticket Naming

**Format**: `{module}_{document}_scenario{N}_{YYMMDD_HHMMSS}.md`

**Example**: `docs/ticket/user_01-crud_scenario2_260203_143052.md`

#### Ticket Structure

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

## Step 4: Final Test Summary (report to user, don't save)

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
| 403 "Identity token is only allowed for tenant selection and exchange" | Using Identity Token instead of Tenant Access Token. Run gate check: `echo $TOKEN \| cut -d. -f2 \| base64 -d \| jq .token_type` — regenerate with `gen-test-tokens.js tenant-owner` |
| 500 ColumnDecode error on API call | Non-UUID `id` values in database (from manual INSERT). Run: `SELECT id FROM {table} WHERE id NOT REGEXP '^[0-9a-f]{8}-'` — delete and re-insert with `UUID()` |
| Feature appears missing or unconfigured | Environment prerequisite not met (e.g., IdP not configured, init not run). Check the scenario's 步骤 0 for required state verification |

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

**IMPORTANT**: See `references/api-testing-cookbook.md` for full recipes and common pitfalls.

- **Service name**: `auth9.TokenExchange` (NOT `auth9.token_exchange.TokenExchange`)
- **mTLS required**: Must pass `-cacert /certs/ca.crt -cert /certs/client.crt -key /certs/client.key`
- **API key**: `x-api-key: dev-grpc-api-key` (required)
- **Host port 50051 is blocked**: Must use `grpcurl-docker.sh` or Docker network
- **Reflection disabled**: Must pass `-import-path /proto -proto auth9.proto`
- **`service_id`**: Use OAuth client_id string (e.g. `auth9-portal`), NOT service UUID
- **Portal service → `auth9-platform` tenant**: Use `auth9-platform` tenant ID, not `demo`

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
PLATFORM_TENANT_ID=$(mysql -u root -h 127.0.0.1 -P 4000 auth9 -N -e \
  "SELECT id FROM tenants WHERE slug = 'auth9-platform';")

.claude/skills/tools/grpcurl-docker.sh \
  -cacert /certs/ca.crt -cert /certs/client.crt -key /certs/client.key \
  -import-path /proto -proto auth9.proto \
  -H "x-api-key: dev-grpc-api-key" \
  -d "{\"identity_token\": \"$TOKEN\", \"tenant_id\": \"$PLATFORM_TENANT_ID\", \"service_id\": \"auth9-portal\"}" \
  auth9-grpc-tls:50051 auth9.TokenExchange/ExchangeToken
```

## Keycloak Webhook Event Simulation

**IMPORTANT**: See `references/api-testing-cookbook.md` for event type mapping and all recipes.

- **Endpoint**: `POST http://localhost:8080/api/v1/keycloak/events`
- **Secret**: `dev-webhook-secret`
- **Signature**: `x-keycloak-signature: sha256=<HMAC-SHA256 hex>`
- **JSON fields**: camelCase (`credentialType`, `authMethod`, `ipAddress`)

```bash
BODY='{"type":"LOGIN_ERROR","realmId":"auth9","userId":"00000000-0000-0000-0000-000000000001","error":"invalid_user_credentials","time":1704067200000,"details":{"username":"test","email":"test@example.com","credentialType":"otp"}}'
SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
SIG=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)

curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/keycloak/events" \
  -H "Content-Type: application/json" \
  -H "x-keycloak-signature: sha256=$SIG" \
  -d "$BODY"
```

## Keycloak Admin API Pitfalls

- **Keycloak 23**: `reset-password` endpoint returns 400 when password policy is active (no detailed error)
- **Workaround**: Use seeded admin user (`admin / SecurePass123!`) or create users without passwords
- **Realm update**: Partial PUT may fail; use GET-merge-PUT pattern for safe updates

## Script-Assisted Testing (FAST PATH)

Some QA documents have pre-built executable test scripts in `scripts/qa/auto/`. These scripts encode all API calls, assertions, and DB validations so you do NOT need to construct curl commands manually.

### How to detect

Check the QA document's corresponding entry in `docs/qa/_manifest.yaml` for a `test_script` field:
```yaml
- id: integration/04-health-check
  test_script: scripts/qa/auto/integration-04-health-check.sh
```

Or check if a matching script exists by convention:
```bash
# For docs/qa/auth/07-public-endpoints.md -> scripts/qa/auto/auth-07-public-endpoints.sh
ls scripts/qa/auto/auth-07-public-endpoints.sh
```

### Workflow when a test script exists

1. **Run the script**: `bash scripts/qa/auto/xxx.sh 2>/tmp/qa-stderr.log`
2. **Read JSONL output** — each line is a structured result:
   - `{"type":"scenario","id":1,"title":"...","status":"PASS",...}` — scenario result
   - `{"type":"assert","status":"FAIL","label":"...","expected":"...","actual":"..."}` — assertion detail
   - `{"type":"summary","total":5,"passed":4,"failed":1,...}` — final summary
3. **For PASS scenarios**: report and move on
4. **For FAIL scenarios**: read the stderr log + Docker logs, then create a ticket as usual
5. **Do NOT re-construct curl commands** that the script already covers

### Workflow when NO test script exists

Follow the standard manual testing workflow described above (construct curl/browser steps from the QA doc).

### Script library reference

Scripts use shared libraries in `scripts/qa/lib/`:
- `lib/assert.sh` — `assert_eq`, `assert_http_status`, `assert_json_field`, `assert_db`, etc.
- `lib/setup.sh` — `gen_admin_token`, `gen_tenant_token`, `api_get`, `api_post`, `db_query`, etc.
- `lib/runner.sh` — `scenario` registration, timing, JSONL output, `run_all` execution
