-- Create tenant-scoped enterprise SSO connector tables

CREATE TABLE IF NOT EXISTS enterprise_sso_connectors (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    alias VARCHAR(100) NOT NULL,
    display_name VARCHAR(255),
    provider_type VARCHAR(20) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INT NOT NULL DEFAULT 100,
    keycloak_alias VARCHAR(140) NOT NULL,
    config JSON NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE INDEX idx_enterprise_sso_tenant_alias (tenant_id, alias),
    UNIQUE INDEX idx_enterprise_sso_keycloak_alias (keycloak_alias),
    INDEX idx_enterprise_sso_tenant_enabled (tenant_id, enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS enterprise_sso_domains (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    connector_id CHAR(36) NOT NULL,
    domain VARCHAR(255) NOT NULL,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    verified_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE INDEX idx_enterprise_sso_domain_unique (domain),
    INDEX idx_enterprise_sso_domains_connector (connector_id),
    INDEX idx_enterprise_sso_domains_tenant (tenant_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
