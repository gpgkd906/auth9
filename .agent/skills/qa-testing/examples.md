# QA Testing Examples

This document shows real examples of using the QA Testing skill.

## Example 1: Testing User Management

### User Request
```
按照QA文档进行用户管理测试 @user-management.md
```

### Agent Workflow

#### Step 1: Confirm Document
```
请确认要执行的QA测试文档:
1. user-management.md (用户管理) ← 用户已指定此文档
2. tenant-management.md (租户管理)
3. rbac.md (权限管理)
4. service-client.md (服务客户端)

✅ 已确认: user-management.md
```

#### Step 2: Environment Verification
```bash
# Execute verification script
bash /Volumes/Yotta/auth9/.cursor/skills/qa-testing/scripts/verify_env.sh
```

Expected output:
```
✅ Docker is running
✅ auth9-core: healthy
✅ auth9-portal: healthy
✅ auth9-keycloak: healthy
✅ auth9-tidb: healthy
✅ auth9-redis: healthy
✅ Portal (http://localhost:3000): accessible
✅ Auth9 Core (http://localhost:8080): accessible
✅ Keycloak (http://localhost:8081): accessible
✅ TiDB: connection successful
✅ Environment is ready for QA testing
```

#### Step 3: List Test Scenarios
```markdown
## User Management Test Scenarios
Total: 13 scenarios

- [ ] Scenario 1: Create user
- [ ] Scenario 2: Create duplicate email user
- [ ] Scenario 3: Update user info
- [ ] Scenario 4: Add user to tenant
- [ ] Scenario 5: Remove user from tenant
- [ ] Scenario 6: Delete user (cascade)
- [ ] Scenario 7a: Enable MFA
- [ ] Scenario 7b: Disable MFA
- [ ] Scenario 8a: User list pagination
- [ ] Scenario 8b: User search
- [ ] Scenario 9: View user's tenant list
- [ ] Scenario 10: User joins same tenant twice
- [ ] Scenario 11: Modify user role in tenant
- [ ] Scenario 12: Email format validation
- [ ] Scenario 13: Password strength validation
```

#### Step 4: Execute Scenario 1 (Create User)

**Read scenario details**:
- Initial state: No user with email `newuser@example.com`
- Test steps: Navigate to Users → Create User → Fill form → Submit
- Expected: User created successfully

**Browser automation**:
```
1. browser_navigate to http://localhost:3000
2. browser_snapshot (get login form)
3. browser_fill username with "admin"
4. browser_fill password with "Admin123!"
5. browser_click sign in button
6. browser_wait 2s
7. browser_snapshot (verify logged in)
8. browser_click "Users" menu
9. browser_wait 1s
10. browser_snapshot (get user list page)
11. browser_click "Create User" button
12. browser_wait 1s
13. browser_snapshot (get create form)
14. browser_fill email with "newuser@example.com"
15. browser_fill display_name with "新用户"
16. browser_fill password with "SecurePass123!"
17. browser_click "Create" button
18. browser_wait 2s
19. browser_snapshot (verify success message)
```

**Database validation**:
```bash
# Check user created
mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e \
  "SELECT id, email, display_name, mfa_enabled FROM users WHERE email='newuser@example.com';"
```

Expected output:
```
+--------------------------------------+----------------------+--------------+-------------+
| id                                   | email                | display_name | mfa_enabled |
+--------------------------------------+----------------------+--------------+-------------+
| 12345678-1234-1234-1234-123456789012 | newuser@example.com  | 新用户        | 0           |
+--------------------------------------+----------------------+--------------+-------------+
```

**Result**: ✅ PASS

---

#### Step 5: Execute Scenario 2 (Duplicate Email)

**Read scenario details**:
- Initial state: User with `existing@example.com` exists
- Test steps: Try to create user with same email
- Expected: Error message "Email already exists"

**Browser automation**:
```
1. browser_click "Create User" button
2. browser_wait 1s
3. browser_snapshot
4. browser_fill email with "existing@example.com"
5. browser_fill display_name with "Another User"
6. browser_fill password with "Password123!"
7. browser_click "Create" button
8. browser_wait 2s
9. browser_snapshot (check for error message)
```

**Snapshot shows**:
```
Error message visible: "Email already exists"
Create button still on page (form not submitted)
```

**Database validation**:
```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e \
  "SELECT COUNT(*) as count FROM users WHERE email='existing@example.com';"
```

Expected output:
```
+-------+
| count |
+-------+
| 1     |
+-------+
```

**Result**: ✅ PASS

---

#### Step 6: Execute Scenario 6 (Delete User with Cascade)

**Prepare test data**:
```bash
# Get user ID for testing
USER_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e \
  "SELECT id FROM users WHERE email='newuser@example.com';" -N -s)

# Verify user has associations
mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e \
  "SELECT 
    (SELECT COUNT(*) FROM tenant_users WHERE user_id='$USER_ID') as tenant_count,
    (SELECT COUNT(*) FROM sessions WHERE user_id='$USER_ID') as session_count;"
```

**Browser automation**:
```
1. browser_snapshot (get user list)
2. browser_click delete button for newuser@example.com
3. browser_wait 1s
4. browser_snapshot (get confirmation dialog)
5. browser_click confirm button
6. browser_wait 2s
7. browser_snapshot (verify user removed from list)
```

**Database validation**:
```bash
# Check user deleted
mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e \
  "SELECT COUNT(*) as user_count FROM users WHERE id='$USER_ID';"

# Check cascade: tenant_users
mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e \
  "SELECT COUNT(*) as tenant_user_count FROM tenant_users WHERE user_id='$USER_ID';"

# Check cascade: sessions
mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e \
  "SELECT COUNT(*) as session_count FROM sessions WHERE user_id='$USER_ID';"
```

Expected output:
```
All counts should be 0
```

**Result**: ✅ PASS

---

## Example 2: Handling Test Failure

### Scenario 4 Failed: Add User to Tenant

**Browser execution**:
```
1-8. [Login steps successful]
9. browser_click "Add to Tenant" button
10. browser_wait 2s
11. browser_snapshot
```

**Snapshot shows error**:
```
Error message: "Internal server error"
Status: 500
```

**Check Docker logs**:
```bash
docker logs auth9-core --tail 50
```

**Log output**:
```
[2026-02-02 10:15:32] ERROR: Failed to add user to tenant
[2026-02-02 10:15:32] ERROR: Database error: Connection pool exhausted
[2026-02-02 10:15:32] ERROR: at src/repository/tenant_user.rs:45
```

**Database validation**:
```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9_db -e \
  "SELECT COUNT(*) FROM tenant_users 
   WHERE user_id='$USER_ID' AND tenant_id='$TENANT_ID';"
```

Output:
```
+-------+
| count |
+-------+
| 0     |
+-------+
```

**Result**: ❌ FAIL
- UI Error: "Internal server error"
- Log Error: "Connection pool exhausted"
- Data State: Association not created (expected 1, got 0)

---

## Example 3: Final Test Report

```markdown
# QA Test Report: User - CRUD Operations

**Test Date**: 2026-02-02 10:30:45
**QA Document**: `docs/qa/user/01-crud.md`
**Environment**: Docker local (all services healthy)
**Tester**: AI Agent
**Duration**: 15 minutes

## Summary

| Status | Count |
|--------|-------|
| ✅ PASS | 11 |
| ❌ FAIL | 2 |
| ⏭️ SKIP | 0 |
| **Total** | 13 |

**Pass Rate**: 84.6%

## Detailed Results

### ✅ Scenario 1: Create User
**Duration**: 45s

**Test Steps**:
- Navigate to Users page: ✅
- Click Create User: ✅
- Fill form (email, name, password): ✅
- Submit form: ✅
- Verify success message: ✅

**Database Validation**: ✅ PASS
- users table: 1 record created
  - email: newuser@example.com
  - display_name: 新用户
  - mfa_enabled: false
  - keycloak_id: populated

---

### ✅ Scenario 2: Create Duplicate Email User
**Duration**: 30s

**Test Steps**:
- Click Create User: ✅
- Fill form with existing email: ✅
- Submit form: ✅
- Verify error message: ✅ "Email already exists"

**Database Validation**: ✅ PASS
- users table: Still only 1 record (no duplicate)

---

### ❌ Scenario 4: Add User to Tenant
**Duration**: 35s

**Test Steps**:
- Open user management: ✅
- Click "Add to Tenant": ✅
- Select tenant: ✅
- Select role: ✅
- Submit: ❌ Internal server error

**Error Details**:
- **UI Error**: "Internal server error" (500)
- **Docker Logs** (auth9-core):
  ```
  [2026-02-02 10:15:32] ERROR: Failed to add user to tenant
  [2026-02-02 10:15:32] ERROR: Database error: Connection pool exhausted
  [2026-02-02 10:15:32] ERROR: at src/repository/tenant_user.rs:45
  ```

**Database Validation**: ❌ FAIL
- Expected: 1 record in tenant_users
- Actual: 0 records (association not created)

**Root Cause**: Database connection pool exhausted

---

### ✅ Scenario 6: Delete User (Cascade)
**Duration**: 50s

**Test Steps**:
- Find user in list: ✅
- Click delete: ✅
- Confirm deletion: ✅
- Verify user removed: ✅

**Database Validation**: ✅ PASS
- users table: 0 records (deleted)
- tenant_users: 0 records (cascaded)
- sessions: 0 records (cascaded)
- user_tenant_roles: 0 records (cascaded via tenant_users)
- Keycloak: User deleted from realm

---

### ❌ Scenario 11: Modify User Role in Tenant
**Duration**: 40s

**Test Steps**:
- Open user tenant management: ✅
- Find target tenant: ✅
- Change role from "member" to "admin": ✅
- Submit: ❌ No response

**Error Details**:
- **UI Error**: Button clicked but no response, spinner indefinitely
- **Browser Console**: Network timeout after 30s
- **Docker Logs** (auth9-core):
  ```
  [2026-02-02 10:25:15] WARN: Update tenant_user role request received
  [2026-02-02 10:25:15] ERROR: Keycloak sync failed: Connection refused
  ```

**Database Validation**: ⚠️ PARTIAL
- tenant_users.role_in_tenant: "admin" (updated in DB)
- But Keycloak sync failed (role not reflected in tokens)

**Root Cause**: Keycloak connection issue during sync

---

## Issues Summary

### 🐛 Bug 1: Connection Pool Exhausted (Scenario 4)
**Severity**: High
**Impact**: Cannot add users to tenants
**Logs**: `Database error: Connection pool exhausted`
**Recommendation**: 
- Increase connection pool size in config
- Investigate connection leaks

### 🐛 Bug 2: Keycloak Sync Failure (Scenario 11)
**Severity**: Medium
**Impact**: Role changes not reflected in access tokens
**Logs**: `Keycloak sync failed: Connection refused`
**Recommendation**:
- Check Keycloak service connectivity
- Add retry mechanism for Keycloak API calls
- Consider async queue for sync operations

## Test Environment Health

All services were healthy during testing except:
- Intermittent Keycloak connectivity issues (2 timeouts)

## Recommendations

1. **Fix Connection Pool**: Critical blocker for tenant user operations
2. **Improve Keycloak Resilience**: Add retries and better error handling
3. **Add UI Loading States**: Scenario 11 had infinite spinner
4. **Retest Failed Scenarios**: After fixes, run Scenario 4 and 11 again

## Next Steps

- [ ] Report bugs to development team
- [ ] Fix connection pool configuration
- [ ] Add Keycloak health monitoring
- [ ] Rerun full test suite after fixes

---

*Report generated by QA Testing Skill*
*Report saved to: `docs/report/user_01-crud_result_260202.md`*
```

**After generating the report, the agent saves it**:

```
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

---

## Example 4: Using Helper Scripts

### Verify Environment Before Testing
```bash
bash /Volumes/Yotta/auth9/.cursor/skills/qa-testing/scripts/verify_env.sh
```

### Check Logs During Test
```bash
# Check backend logs
bash /Volumes/Yotta/auth9/.cursor/skills/qa-testing/scripts/check_logs.sh auth9-core 100

# Check frontend logs
bash /Volumes/Yotta/auth9/.cursor/skills/qa-testing/scripts/check_logs.sh auth9-portal 50
```

### Quick Database Query
```bash
# Check user count
bash /Volumes/Yotta/auth9/.cursor/skills/qa-testing/scripts/db_query.sh \
  "SELECT COUNT(*) as user_count FROM users;"

# Check specific user
bash /Volumes/Yotta/auth9/.cursor/skills/qa-testing/scripts/db_query.sh \
  "SELECT id, email, display_name FROM users WHERE email='test@example.com';"
```

---

## Tips for Efficient Testing

1. **Test in order**: Don't skip scenarios - later ones may depend on earlier state
2. **Snapshot frequently**: Always snapshot before clicking to get element refs
3. **Use incremental waits**: Start with 1-2s, add more if needed
4. **Validate after every scenario**: Even if UI looks good, check database
5. **Log errors immediately**: Don't wait until end to check logs
6. **Reset if needed**: If environment gets dirty, use reset-local-env skill
