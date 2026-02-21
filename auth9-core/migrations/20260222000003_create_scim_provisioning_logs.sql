-- SCIM provisioning audit log table
CREATE TABLE IF NOT EXISTS scim_provisioning_logs (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    connector_id CHAR(36) NOT NULL,
    operation VARCHAR(20) NOT NULL,
    resource_type VARCHAR(20) NOT NULL,
    scim_resource_id VARCHAR(255),
    auth9_resource_id CHAR(36),
    status VARCHAR(10) NOT NULL,
    error_detail TEXT,
    response_status INT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_scim_log_connector (connector_id),
    INDEX idx_scim_log_created (created_at)
);
