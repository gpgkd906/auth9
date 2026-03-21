-- Password history table for preventing password reuse
CREATE TABLE IF NOT EXISTS password_history (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_password_history_user_id (user_id),
    INDEX idx_password_history_created_at (created_at)
);
