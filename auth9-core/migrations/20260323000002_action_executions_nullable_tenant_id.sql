-- Make action_executions.tenant_id nullable to match actions table.
-- Migration 20260221000001 made actions.tenant_id nullable (for PostChangePassword
-- fallback and service-level actions), but forgot to do the same for
-- action_executions.tenant_id.  When an action with tenant_id=NULL executes,
-- record_execution fails with "Column 'tenant_id' cannot be null".
ALTER TABLE action_executions MODIFY tenant_id CHAR(36) NULL;
