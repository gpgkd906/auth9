-- Pending / required actions for users (verify email, configure MFA, etc.)
-- No foreign keys per TiDB architecture rules

CREATE TABLE IF NOT EXISTS pending_actions (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    action_type VARCHAR(64) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'pending',
    metadata JSON,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP NULL,
    INDEX idx_pending_actions_user (user_id),
    INDEX idx_pending_actions_user_status (user_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
