-- SAML Application (IdP outbound) management
-- Auth9 acts as SAML Identity Provider, signing assertions for external Service Providers

CREATE TABLE IF NOT EXISTS saml_applications (
    id CHAR(36) NOT NULL PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    name VARCHAR(255) NOT NULL,
    entity_id VARCHAR(512) NOT NULL COMMENT 'SP Entity ID / Audience',
    acs_url VARCHAR(1024) NOT NULL COMMENT 'Assertion Consumer Service URL',
    slo_url VARCHAR(1024) NULL COMMENT 'Single Logout URL (optional)',
    name_id_format VARCHAR(128) NOT NULL DEFAULT 'urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress',
    sign_assertions BOOLEAN NOT NULL DEFAULT TRUE,
    sign_responses BOOLEAN NOT NULL DEFAULT TRUE,
    encrypt_assertions BOOLEAN NOT NULL DEFAULT FALSE,
    sp_certificate TEXT NULL COMMENT 'SP signing/encryption certificate (PEM)',
    attribute_mappings JSON NOT NULL COMMENT 'User attribute to SAML attribute mappings',
    keycloak_client_id VARCHAR(255) NOT NULL COMMENT 'Keycloak SAML Client UUID',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE INDEX idx_saml_app_tenant_entity (tenant_id, entity_id),
    INDEX idx_saml_app_tenant (tenant_id),
    UNIQUE INDEX idx_saml_app_kc_client (keycloak_client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
