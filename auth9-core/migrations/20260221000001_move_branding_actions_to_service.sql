-- Move Branding & Actions to Service level
-- Each Service (application) can now have independent branding and action hooks

-- 1. service_branding table
CREATE TABLE service_branding (
    id CHAR(36) PRIMARY KEY,
    service_id CHAR(36) NOT NULL,
    config JSON NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE INDEX uk_service_branding_service_id (service_id)
);

-- 2. actions table: add service_id column
ALTER TABLE actions ADD COLUMN service_id CHAR(36) NULL AFTER tenant_id;
ALTER TABLE actions ADD INDEX idx_actions_service_id (service_id);

-- 3. action_executions table: add service_id column
ALTER TABLE action_executions ADD COLUMN service_id CHAR(36) NULL AFTER tenant_id;
ALTER TABLE action_executions ADD INDEX idx_action_executions_service_id (service_id);

-- 4. Data migration: assign existing actions to their tenant's first service
UPDATE actions a SET a.service_id = (
    SELECT s.id FROM services s WHERE s.tenant_id = a.tenant_id LIMIT 1
) WHERE a.service_id IS NULL;

-- 5. Replace unique constraint: tenant_id+trigger+name -> service_id+trigger+name
ALTER TABLE actions DROP INDEX uk_tenant_trigger_name;
ALTER TABLE actions ADD UNIQUE INDEX uk_service_trigger_name (service_id, trigger_id, name);

-- 6. Make service_id NOT NULL now that data is migrated
ALTER TABLE actions MODIFY service_id CHAR(36) NOT NULL;

-- 7. Make tenant_id nullable (kept for PostChangePassword fallback)
ALTER TABLE actions MODIFY tenant_id CHAR(36) NULL;
