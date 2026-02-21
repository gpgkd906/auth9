-- SCIM Bearer Token table, associated with SSO Connector
CREATE TABLE IF NOT EXISTS scim_tokens (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    connector_id CHAR(36) NOT NULL,
    token_hash VARCHAR(128) NOT NULL,
    token_prefix VARCHAR(12) NOT NULL,
    description VARCHAR(255),
    expires_at TIMESTAMP NULL,
    last_used_at TIMESTAMP NULL,
    revoked_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE INDEX idx_scim_tokens_hash (token_hash),
    INDEX idx_scim_tokens_connector (connector_id),
    INDEX idx_scim_tokens_tenant (tenant_id)
);
