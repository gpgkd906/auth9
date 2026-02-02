-- Tenant-Service association table
-- Stores which services are enabled for each tenant
-- All services are global by default; this table tracks which ones a tenant has enabled

CREATE TABLE IF NOT EXISTS tenant_services (
    tenant_id CHAR(36) NOT NULL,
    service_id CHAR(36) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    PRIMARY KEY (tenant_id, service_id),
    INDEX idx_tenant_services_tenant (tenant_id),
    INDEX idx_tenant_services_service (service_id)
);
