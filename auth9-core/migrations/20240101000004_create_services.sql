-- Create services table (OIDC clients)
CREATE TABLE IF NOT EXISTS services (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36),
    name VARCHAR(255) NOT NULL,
    base_url TEXT,
    redirect_uris JSON NOT NULL,
    logout_uris JSON NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    -- TiDB compatible: generated column for unique constraint on (tenant_id, name)
    -- Handles NULL tenant_id by using a sentinel value
    tenant_id_key CHAR(36) AS (COALESCE(tenant_id, '00000000-0000-0000-0000-000000000000')) STORED,

    INDEX idx_services_tenant (tenant_id),
    INDEX idx_services_status (status),
    UNIQUE INDEX idx_services_tenant_name_unique (tenant_id_key, name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
