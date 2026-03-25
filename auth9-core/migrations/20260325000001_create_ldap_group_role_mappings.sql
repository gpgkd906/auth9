-- LDAP group-to-Auth9 role mapping for enterprise SSO connectors
CREATE TABLE IF NOT EXISTS ldap_group_role_mappings (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    connector_id CHAR(36) NOT NULL,
    ldap_group_dn VARCHAR(1024) NOT NULL,
    ldap_group_display_name VARCHAR(255),
    role_id CHAR(36) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE INDEX idx_ldap_grm_connector_group_role (connector_id, ldap_group_dn, role_id),
    INDEX idx_ldap_grm_tenant (tenant_id),
    INDEX idx_ldap_grm_connector (connector_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
