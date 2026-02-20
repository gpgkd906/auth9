-- ABAC policy sets per tenant
CREATE TABLE IF NOT EXISTS abac_policy_sets (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    mode VARCHAR(16) NOT NULL DEFAULT 'disabled',
    published_version_id CHAR(36) NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE KEY uk_abac_policy_sets_tenant (tenant_id),
    INDEX idx_abac_policy_sets_mode (mode),
    INDEX idx_abac_policy_sets_published (published_version_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ABAC versioned policy documents
CREATE TABLE IF NOT EXISTS abac_policy_set_versions (
    id CHAR(36) PRIMARY KEY,
    policy_set_id CHAR(36) NOT NULL,
    version_no INT NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'draft',
    policy_json JSON NOT NULL,
    change_note VARCHAR(255) NULL,
    created_by CHAR(36) NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    published_at TIMESTAMP NULL,

    UNIQUE KEY uk_abac_policy_versions_no (policy_set_id, version_no),
    INDEX idx_abac_policy_versions_set (policy_set_id),
    INDEX idx_abac_policy_versions_status (status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
