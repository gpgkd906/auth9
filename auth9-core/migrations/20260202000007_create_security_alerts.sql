-- Create security_alerts table for anomaly detection
CREATE TABLE IF NOT EXISTS security_alerts (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36),
    tenant_id CHAR(36),
    alert_type ENUM('brute_force', 'new_device', 'impossible_travel', 'suspicious_ip') NOT NULL,
    severity ENUM('low', 'medium', 'high', 'critical') NOT NULL,
    details JSON,
    resolved_at TIMESTAMP NULL,
    resolved_by CHAR(36),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_security_alerts_user_id (user_id),
    INDEX idx_security_alerts_tenant_id (tenant_id),
    INDEX idx_security_alerts_severity (severity),
    INDEX idx_security_alerts_created_at (created_at),
    INDEX idx_security_alerts_alert_type (alert_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
