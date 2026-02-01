-- Add unique constraint to prevent duplicate services from race conditions
--
-- Problem: When multiple init containers run simultaneously, they can create
-- duplicate services because the check-then-insert is not atomic.
--
-- Solution: Add unique constraints so INSERT IGNORE can be used safely.

-- Step 1: Clean up any existing duplicate services (keep oldest by id)
-- This handles the case where duplicates already exist
DELETE s1 FROM services s1
INNER JOIN (
    SELECT name, tenant_id, MIN(id) as keep_id
    FROM services
    GROUP BY name, tenant_id
    HAVING COUNT(*) > 1
) s2 ON s1.name = s2.name
    AND (s1.tenant_id = s2.tenant_id OR (s1.tenant_id IS NULL AND s2.tenant_id IS NULL))
    AND s1.id != s2.keep_id;

-- Step 2: Clean up orphaned clients pointing to deleted services
DELETE FROM clients WHERE service_id NOT IN (SELECT id FROM services);

-- Step 3: For global services (tenant_id IS NULL), we need name to be unique
-- MySQL/TiDB treats NULL as distinct in unique indexes, so we need a workaround
-- Use a generated column with COALESCE to handle NULL tenant_id
ALTER TABLE services
ADD COLUMN tenant_id_key CHAR(36) AS (COALESCE(tenant_id, '00000000-0000-0000-0000-000000000000')) STORED;

-- Step 4: Create unique index on the generated column + name
CREATE UNIQUE INDEX idx_services_tenant_name_unique ON services(tenant_id_key, name);
