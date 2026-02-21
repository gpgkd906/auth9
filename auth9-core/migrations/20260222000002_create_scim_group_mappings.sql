-- Add SCIM tracking fields to users table
ALTER TABLE users
    ADD COLUMN scim_external_id VARCHAR(255) NULL AFTER keycloak_id,
    ADD COLUMN scim_provisioned_by CHAR(36) NULL AFTER scim_external_id,
    ADD INDEX idx_users_scim_external_id (scim_external_id);

-- SCIM Group to Auth9 Role mapping table
CREATE TABLE IF NOT EXISTS scim_group_role_mappings (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    connector_id CHAR(36) NOT NULL,
    scim_group_id VARCHAR(255) NOT NULL,
    scim_group_display_name VARCHAR(255),
    role_id CHAR(36) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE INDEX idx_scim_group_mapping (connector_id, scim_group_id),
    INDEX idx_scim_group_tenant (tenant_id)
);
