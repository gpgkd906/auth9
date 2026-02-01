-- Create login_events table for analytics
CREATE TABLE IF NOT EXISTS login_events (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id CHAR(36),
    email VARCHAR(320),
    tenant_id CHAR(36),
    event_type ENUM('success', 'failed_password', 'failed_mfa', 'locked', 'social') NOT NULL,
    ip_address VARCHAR(45),
    user_agent TEXT,
    device_type VARCHAR(50),
    location VARCHAR(255),
    session_id CHAR(36),
    failure_reason VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_login_events_user_id (user_id),
    INDEX idx_login_events_tenant_id (tenant_id),
    INDEX idx_login_events_created_at (created_at),
    INDEX idx_login_events_event_type (event_type),
    INDEX idx_login_events_ip_address (ip_address)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
