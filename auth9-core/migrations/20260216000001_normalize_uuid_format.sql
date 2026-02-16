-- Normalize UUID columns to lowercase-hyphenated format (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
-- Fixes data inconsistency where some UUIDs were stored without hyphens (e.g. from TiDB UUID())

-- roles.id
UPDATE roles SET id = LOWER(CONCAT(
    LEFT(REPLACE(id, '-', ''), 8), '-',
    SUBSTR(REPLACE(id, '-', ''), 9, 4), '-',
    SUBSTR(REPLACE(id, '-', ''), 13, 4), '-',
    SUBSTR(REPLACE(id, '-', ''), 17, 4), '-',
    SUBSTR(REPLACE(id, '-', ''), 21)
)) WHERE id NOT LIKE '%-%';

-- permissions.id
UPDATE permissions SET id = LOWER(CONCAT(
    LEFT(REPLACE(id, '-', ''), 8), '-',
    SUBSTR(REPLACE(id, '-', ''), 9, 4), '-',
    SUBSTR(REPLACE(id, '-', ''), 13, 4), '-',
    SUBSTR(REPLACE(id, '-', ''), 17, 4), '-',
    SUBSTR(REPLACE(id, '-', ''), 21)
)) WHERE id NOT LIKE '%-%';

-- role_permissions referencing columns
UPDATE role_permissions SET role_id = LOWER(CONCAT(
    LEFT(REPLACE(role_id, '-', ''), 8), '-',
    SUBSTR(REPLACE(role_id, '-', ''), 9, 4), '-',
    SUBSTR(REPLACE(role_id, '-', ''), 13, 4), '-',
    SUBSTR(REPLACE(role_id, '-', ''), 17, 4), '-',
    SUBSTR(REPLACE(role_id, '-', ''), 21)
)) WHERE role_id NOT LIKE '%-%';

UPDATE role_permissions SET permission_id = LOWER(CONCAT(
    LEFT(REPLACE(permission_id, '-', ''), 8), '-',
    SUBSTR(REPLACE(permission_id, '-', ''), 9, 4), '-',
    SUBSTR(REPLACE(permission_id, '-', ''), 13, 4), '-',
    SUBSTR(REPLACE(permission_id, '-', ''), 17, 4), '-',
    SUBSTR(REPLACE(permission_id, '-', ''), 21)
)) WHERE permission_id NOT LIKE '%-%';

-- user_tenant_roles referencing columns
UPDATE user_tenant_roles SET role_id = LOWER(CONCAT(
    LEFT(REPLACE(role_id, '-', ''), 8), '-',
    SUBSTR(REPLACE(role_id, '-', ''), 9, 4), '-',
    SUBSTR(REPLACE(role_id, '-', ''), 13, 4), '-',
    SUBSTR(REPLACE(role_id, '-', ''), 17, 4), '-',
    SUBSTR(REPLACE(role_id, '-', ''), 21)
)) WHERE role_id NOT LIKE '%-%';
