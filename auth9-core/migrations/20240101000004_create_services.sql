-- Create services table (OIDC clients)
CREATE TABLE IF NOT EXISTS services (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36),
    name VARCHAR(255) NOT NULL,
    client_id VARCHAR(255) NOT NULL UNIQUE,
    client_secret_hash VARCHAR(255) NOT NULL,
    base_url TEXT,
    redirect_uris JSON NOT NULL,
    logout_uris JSON NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    INDEX idx_services_tenant (tenant_id),
    INDEX idx_services_client_id (client_id),
    INDEX idx_services_status (status),
    
    CONSTRAINT fk_services_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
