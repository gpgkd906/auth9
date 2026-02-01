# QA Testing Reference

This document provides detailed SQL queries and validation patterns for Auth9 QA testing.

## Database Schema Quick Reference

### Core Tables

```sql
-- Users
users (id, keycloak_id, email, display_name, avatar_url, mfa_enabled, created_at, updated_at)

-- Tenants
tenants (id, name, slug, logo_url, settings, status, created_at, updated_at)

-- Tenant Users (join table)
tenant_users (id, tenant_id, user_id, role_in_tenant, joined_at)

-- Services
services (id, tenant_id, name, description, client_id, client_secret, redirect_uris, status, created_at, updated_at)

-- Roles
roles (id, service_id, name, description, created_at, updated_at)

-- Permissions
permissions (id, service_id, resource, action, description, created_at, updated_at)

-- Role Permissions (join table)
role_permissions (id, role_id, permission_id, created_at)

-- User Tenant Roles (join table)
user_tenant_roles (id, tenant_user_id, role_id, assigned_at)

-- Audit Logs
audit_logs (id, user_id, action, resource_type, resource_id, old_value, new_value, ip_address, user_agent, created_at)

-- Sessions
sessions (id, user_id, tenant_id, access_token, refresh_token, expires_at, created_at, updated_at)
```

## Validation Query Templates

### User Management

#### Check User Exists
```sql
SELECT 
    id, 
    keycloak_id, 
    email, 
    display_name, 
    mfa_enabled, 
    created_at
FROM users 
WHERE email = '{email}';
```

#### Check User Count
```sql
SELECT COUNT(*) as user_count 
FROM users 
WHERE email = '{email}';
```

#### Check User's Tenants
```sql
SELECT 
    t.id as tenant_id,
    t.name as tenant_name,
    t.slug,
    tu.role_in_tenant,
    tu.joined_at
FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
WHERE tu.user_id = '{user_id}'
ORDER BY tu.joined_at DESC;
```

#### Check User's Roles in Tenant
```sql
SELECT 
    r.name as role_name,
    r.description,
    s.name as service_name,
    utr.assigned_at
FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
JOIN services s ON s.id = r.service_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' 
  AND tu.tenant_id = '{tenant_id}';
```

#### Check Cascade Deletion (User)
```sql
-- Check user deleted
SELECT COUNT(*) FROM users WHERE id = '{user_id}';
-- Expected: 0

-- Check tenant associations deleted
SELECT COUNT(*) FROM tenant_users WHERE user_id = '{user_id}';
-- Expected: 0

-- Check sessions deleted
SELECT COUNT(*) FROM sessions WHERE user_id = '{user_id}';
-- Expected: 0

-- Check role assignments deleted
SELECT COUNT(*) FROM user_tenant_roles 
WHERE tenant_user_id IN (
    SELECT id FROM tenant_users WHERE user_id = '{user_id}'
);
-- Expected: 0
```

### Tenant Management

#### Check Tenant Exists
```sql
SELECT 
    id, 
    name, 
    slug, 
    logo_url, 
    status, 
    settings, 
    created_at
FROM tenants 
WHERE slug = '{slug}';
```

#### Check Tenant Members
```sql
SELECT 
    u.id as user_id,
    u.email,
    u.display_name,
    tu.role_in_tenant,
    tu.joined_at
FROM tenant_users tu
JOIN users u ON u.id = tu.user_id
WHERE tu.tenant_id = '{tenant_id}'
ORDER BY tu.joined_at DESC;
```

#### Check Tenant Services
```sql
SELECT 
    id,
    name,
    description,
    client_id,
    status,
    created_at
FROM services
WHERE tenant_id = '{tenant_id}'
ORDER BY created_at DESC;
```

#### Check Cascade Deletion (Tenant)
```sql
-- Check tenant deleted
SELECT COUNT(*) FROM tenants WHERE id = '{tenant_id}';
-- Expected: 0

-- Check tenant users deleted
SELECT COUNT(*) FROM tenant_users WHERE tenant_id = '{tenant_id}';
-- Expected: 0

-- Check services deleted
SELECT COUNT(*) FROM services WHERE tenant_id = '{tenant_id}';
-- Expected: 0

-- Check webhooks deleted (if table exists)
SELECT COUNT(*) FROM webhooks WHERE tenant_id = '{tenant_id}';
-- Expected: 0
```

### RBAC (Roles and Permissions)

#### Check Role Exists
```sql
SELECT 
    r.id,
    r.name,
    r.description,
    s.name as service_name,
    r.created_at
FROM roles r
JOIN services s ON s.id = r.service_id
WHERE r.name = '{role_name}' 
  AND r.service_id = '{service_id}';
```

#### Check Role Permissions
```sql
SELECT 
    p.resource,
    p.action,
    p.description
FROM role_permissions rp
JOIN permissions p ON p.id = rp.permission_id
WHERE rp.role_id = '{role_id}'
ORDER BY p.resource, p.action;
```

#### Check User Has Role
```sql
SELECT 
    r.name as role_name,
    u.email,
    t.name as tenant_name
FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
JOIN users u ON u.id = tu.user_id
JOIN tenants t ON t.id = tu.tenant_id
WHERE u.id = '{user_id}' 
  AND t.id = '{tenant_id}'
  AND r.id = '{role_id}';
```

#### Check Cascade Deletion (Role)
```sql
-- Check role deleted
SELECT COUNT(*) FROM roles WHERE id = '{role_id}';
-- Expected: 0

-- Check role permissions deleted
SELECT COUNT(*) FROM role_permissions WHERE role_id = '{role_id}';
-- Expected: 0

-- Check user assignments deleted
SELECT COUNT(*) FROM user_tenant_roles WHERE role_id = '{role_id}';
-- Expected: 0
```

### Service Clients

#### Check Service Client Exists
```sql
SELECT 
    id,
    tenant_id,
    name,
    description,
    client_id,
    redirect_uris,
    status,
    created_at
FROM services
WHERE client_id = '{client_id}';
```

#### Check Service Roles
```sql
SELECT 
    id,
    name,
    description,
    created_at
FROM roles
WHERE service_id = '{service_id}'
ORDER BY created_at DESC;
```

#### Check Service Permissions
```sql
SELECT 
    id,
    resource,
    action,
    description
FROM permissions
WHERE service_id = '{service_id}'
ORDER BY resource, action;
```

### Audit Logs

#### Check Recent Audit Logs
```sql
SELECT 
    action,
    resource_type,
    resource_id,
    user_id,
    created_at
FROM audit_logs
WHERE resource_type = '{resource_type}'
ORDER BY created_at DESC
LIMIT 10;
```

#### Check Specific Action
```sql
SELECT 
    action,
    resource_type,
    old_value,
    new_value,
    created_at
FROM audit_logs
WHERE resource_type = '{resource_type}'
  AND resource_id = '{resource_id}'
ORDER BY created_at DESC
LIMIT 1;
```

## Common Validation Patterns

### Pattern 1: Verify Creation
```sql
-- 1. Check record exists
SELECT COUNT(*) FROM {table} WHERE {key} = '{value}';
-- Expected: 1

-- 2. Check field values
SELECT {fields} FROM {table} WHERE {key} = '{value}';
-- Compare with expected values

-- 3. Check audit log
SELECT action, new_value FROM audit_logs 
WHERE resource_type = '{type}' 
  AND action = 'create'
ORDER BY created_at DESC LIMIT 1;
```

### Pattern 2: Verify Update
```sql
-- 1. Check updated field
SELECT {updated_field}, updated_at 
FROM {table} 
WHERE {key} = '{value}';
-- Compare with expected value

-- 2. Check updated_at timestamp
SELECT updated_at FROM {table} WHERE {key} = '{value}';
-- Should be recent (within last minute)

-- 3. Check audit log
SELECT action, old_value, new_value FROM audit_logs
WHERE resource_type = '{type}'
  AND resource_id = '{id}'
ORDER BY created_at DESC LIMIT 1;
```

### Pattern 3: Verify Deletion
```sql
-- 1. Check main record deleted
SELECT COUNT(*) FROM {table} WHERE id = '{id}';
-- Expected: 0

-- 2. Check cascaded deletions
SELECT COUNT(*) FROM {related_table} WHERE {foreign_key} = '{id}';
-- Expected: 0 (repeat for all related tables)

-- 3. Check audit log
SELECT action FROM audit_logs
WHERE resource_type = '{type}'
  AND resource_id = '{id}'
ORDER BY created_at DESC LIMIT 1;
-- Expected: action = 'delete'
```

### Pattern 4: Verify Duplicate Prevention
```sql
-- Check only one record exists
SELECT COUNT(*) FROM {table} WHERE {unique_field} = '{value}';
-- Expected: 1 (or 0 if should be rejected)
```

### Pattern 5: Verify Association
```sql
-- Check join table has correct association
SELECT COUNT(*) FROM {join_table}
WHERE {foreign_key_1} = '{id_1}'
  AND {foreign_key_2} = '{id_2}';
-- Expected: 1

-- Check association fields (role, status, etc.)
SELECT {association_fields} FROM {join_table}
WHERE {foreign_key_1} = '{id_1}'
  AND {foreign_key_2} = '{id_2}';
-- Compare with expected values
```

## Docker Commands Reference

### Check Service Logs
```bash
# Last 50 lines
docker logs auth9-core --tail 50
docker logs auth9-portal --tail 50
docker logs auth9-keycloak --tail 50

# Follow logs in real-time
docker logs -f auth9-core

# Logs with timestamps
docker logs --timestamps auth9-core --tail 50
```

### Execute Database Queries
```bash
# Single query using host mysql client
mysql -h 127.0.0.1 -P 4000 -u root auth9_db \
  -e "SELECT * FROM users LIMIT 5;"

# Interactive shell
mysql -h 127.0.0.1 -P 4000 -u root auth9_db

# Execute SQL file
mysql -h 127.0.0.1 -P 4000 -u root auth9_db < test_data.sql

# If mysql client not installed
brew install mysql-client
```

### Check Service Health
```bash
# Check all containers
docker ps

# Check specific service health
docker ps --filter "name=auth9-core" --format "{{.Status}}"

# Restart service
docker-compose restart auth9-core
```

## Keycloak Verification

### Check User in Keycloak Admin Console

1. Open http://localhost:8081
2. Login with admin/admin
3. Select realm (tenant slug)
4. Go to "Users" section
5. Search by email or username

### Verify User Attributes

Expected fields:
- Email: Should match Auth9 database
- First Name / Last Name: From display_name
- Enabled: true
- Email Verified: true (if verified in Auth9)

### Verify MFA Configuration

When MFA enabled:
- User should have "Credentials" tab
- OTP credential should be configured

When MFA disabled:
- OTP credentials should be removed

## Troubleshooting Common Issues

### Issue: User not found in Keycloak

**Check**:
```sql
SELECT keycloak_id FROM users WHERE email = '{email}';
```

**Verify**:
- keycloak_id is not null
- User exists in Keycloak admin console
- Realm name matches tenant slug

### Issue: Orphaned records after deletion

**Check all related tables**:
```sql
-- For deleted user
SELECT 'tenant_users' as table_name, COUNT(*) as count 
FROM tenant_users WHERE user_id = '{deleted_user_id}'
UNION ALL
SELECT 'sessions', COUNT(*) FROM sessions WHERE user_id = '{deleted_user_id}'
UNION ALL
SELECT 'audit_logs', COUNT(*) FROM audit_logs WHERE user_id = '{deleted_user_id}';
```

### Issue: Duplicate unique constraint violation

**Check for duplicates**:
```sql
-- Check email duplicates
SELECT email, COUNT(*) as count
FROM users
GROUP BY email
HAVING count > 1;

-- Check slug duplicates
SELECT slug, COUNT(*) as count
FROM tenants
GROUP BY slug
HAVING count > 1;
```

### Issue: Incorrect timestamps

**Check timestamp fields**:
```sql
SELECT 
    created_at,
    updated_at,
    TIMESTAMPDIFF(SECOND, created_at, NOW()) as seconds_since_creation
FROM {table}
WHERE id = '{id}';
```

Timestamps should be:
- In UTC timezone
- Recent (within last few seconds for new records)
- updated_at >= created_at
