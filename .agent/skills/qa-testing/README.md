# QA Testing Skill

Execute scenario-based manual QA testing for Auth9 using browser automation.

## Directory Structure

```
qa-testing/
├── SKILL.md              # Main skill instructions
├── reference.md          # SQL queries and validation patterns
├── examples.md           # Usage examples and test report templates
└── scripts/              # Helper scripts
    ├── verify_env.sh     # Check Docker environment readiness
    ├── check_logs.sh     # Fetch service logs
    └── db_query.sh       # Execute database queries
```

## Quick Start

### 1. Prepare Environment

Ensure Docker environment is running:

```bash
# Start services
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# Verify environment
bash .cursor/skills/qa-testing/scripts/verify_env.sh
```

### 2. Run QA Tests

Ask the agent to run QA tests:

```
按照QA文档进行用户管理测试 @user-management.md
```

The agent will:
1. ✅ Confirm which QA document to test
2. ✅ Verify Docker environment is ready
3. ✅ List all test scenarios
4. ✅ Execute each scenario with browser automation
5. ✅ Check logs on errors
6. ✅ Validate database state after each scenario
7. ✅ Generate final test report

## QA Documents

QA test documents are located in `docs/qa/` directory, organized by modules. The agent will automatically discover all available QA documents when you request testing.

**Directory Structure**:
```
docs/qa/
├── README.md           # Index of all QA documents
├── tenant/             # Tenant management (2 docs, 10 scenarios)
├── user/               # User management (3 docs, 13 scenarios)
├── rbac/               # RBAC (4 docs, 17 scenarios)
├── service/            # Services & clients (3 docs, 15 scenarios)
├── invitation/         # Invitation (3 docs, 15 scenarios)
├── session/            # Session & security (4 docs, 20 scenarios)
├── webhook/            # Webhook (4 docs, 17 scenarios)
└── auth/               # Authentication (5 docs, 23 scenarios)

Total: 28 documents, 130 scenarios
```

**How it works**:
- **Specific request**: Agent finds and confirms the matching document(s)
  - Example: "测试用户CRUD" → finds `user/01-crud.md`
  - Example: "测试用户管理模块" → lists all `user/*.md` files
- **Vague request**: Agent lists all available modules for you to choose
  - Example: "进行QA测试" → shows all 8 modules with document counts

## Test Reports

All test reports are automatically saved to `docs/report/` with the following naming convention:

**Format**: `{module}_{document}_result_{YYMMDD}.md`

**Examples**:
- `docs/report/user_01-crud_result_260202.md`
- `docs/report/tenant_01-crud_result_260202.md`
- `docs/report/rbac_02-role_result_260202.md`

Each report includes:
- Summary statistics (pass/fail counts, pass rate)
- Detailed results per scenario
- Error logs and root cause analysis
- Issues summary with severity
- Actionable recommendations

## Helper Scripts

### Verify Environment
```bash
bash .cursor/skills/qa-testing/scripts/verify_env.sh
```

Checks:
- Docker is running
- All services are healthy
- Service URLs are accessible
- Database connection works

### Check Service Logs
```bash
# Default: auth9-core, last 50 lines
bash .cursor/skills/qa-testing/scripts/check_logs.sh

# Custom service and lines
bash .cursor/skills/qa-testing/scripts/check_logs.sh auth9-portal 100
```

### Execute Database Query
```bash
bash .cursor/skills/qa-testing/scripts/db_query.sh "SELECT COUNT(*) FROM users;"
```

## Test Workflow

```
┌─────────────────────────────────────────┐
│  1. Agent confirms QA document          │
│     with user                           │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  2. Verify Docker environment           │
│     (verify_env.sh)                     │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  3. Read QA document                    │
│     Extract scenarios                   │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  4. For each scenario:                  │
│     ┌───────────────────────────────┐   │
│     │ a. Execute in browser         │   │
│     └───────┬───────────────────────┘   │
│             │                           │
│             ▼                           │
│     ┌───────────────────────────────┐   │
│     │ b. If error → Check logs      │   │
│     └───────┬───────────────────────┘   │
│             │                           │
│             ▼                           │
│     ┌───────────────────────────────┐   │
│     │ c. Validate database state    │   │
│     └───────┬───────────────────────┘   │
│             │                           │
│             ▼                           │
│     ┌───────────────────────────────┐   │
│     │ d. Record result              │   │
│     └───────────────────────────────┘   │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  5. Generate test report                │
│     - Summary (pass/fail counts)        │
│     - Detailed results per scenario     │
│     - Error details with logs           │
│     - Recommendations                   │
└─────────────────────────────────────────┘
```

## Features

### ✅ Browser Automation
- Uses `cursor-ide-browser` MCP tools
- Automatic login handling
- Element interaction (click, type, fill)
- UI state verification via snapshots

### ✅ Error Handling
- Captures UI errors
- Fetches Docker service logs
- Records error context (scenario, step, timestamp)

### ✅ Database Validation
- Executes SQL queries from QA documents
- Compares actual vs expected state
- Checks for orphaned records
- Verifies cascade deletions

### ✅ Comprehensive Reporting
- Pass/fail summary
- Detailed results per scenario
- Error logs and root cause analysis
- Actionable recommendations

## Example Output

```markdown
# QA Test Report: User Management

**Test Date**: 2026-02-02 10:30:45
**Duration**: 15 minutes

## Summary

| Status | Count |
|--------|-------|
| ✅ PASS | 11 |
| ❌ FAIL | 2 |
| **Total** | 13 |

**Pass Rate**: 84.6%

## Issues Found

### 🐛 Bug 1: Connection Pool Exhausted
**Scenario**: 4 (Add User to Tenant)
**Severity**: High
**Logs**: `Database error: Connection pool exhausted`

### 🐛 Bug 2: Keycloak Sync Failure
**Scenario**: 11 (Modify User Role)
**Severity**: Medium
**Logs**: `Keycloak sync failed: Connection refused`

## Recommendations
- Fix connection pool configuration
- Add Keycloak retry mechanism
```

## Reference Documentation

- **SKILL.md**: Complete instructions for the agent
- **reference.md**: SQL query templates and validation patterns
- **examples.md**: Real-world test examples and report templates

## Tips

1. **Always verify environment first** - Use `verify_env.sh` before testing
2. **Test in order** - Don't skip scenarios, they may depend on each other
3. **Check database after every scenario** - Even if UI looks correct
4. **Reset if needed** - Use `reset-local-env` skill if environment gets dirty
5. **Incremental waits** - Use short waits (1-3s), add more if needed

## Troubleshooting

### Environment not ready
```bash
# Check service status
docker ps

# Restart services
docker-compose restart

# Full reset
# Use reset-local-env skill
```

### Browser automation fails
```bash
# Check portal accessibility
curl http://localhost:3000

# Check logs
bash .cursor/skills/qa-testing/scripts/check_logs.sh auth9-portal
```

### Database validation fails
```bash
# Test connection using host mysql client
mysql -h 127.0.0.1 -P 4000 -u root -e "SELECT 1;"

# Check database exists
mysql -h 127.0.0.1 -P 4000 -u root -e "SHOW DATABASES;"

# If mysql client not found, install it
brew install mysql-client
```

## License

Part of Auth9 project.
