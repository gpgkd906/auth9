-- Create sessions table for session management
CREATE TABLE IF NOT EXISTS sessions (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    keycloak_session_id VARCHAR(255),
    device_type VARCHAR(50),
    device_name VARCHAR(255),
    ip_address VARCHAR(45),
    location VARCHAR(255),
    user_agent TEXT,
    last_active_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    revoked_at TIMESTAMP NULL,

    INDEX idx_sessions_user_id (user_id),
    INDEX idx_sessions_keycloak_session (keycloak_session_id),
    INDEX idx_sessions_last_active (last_active_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
