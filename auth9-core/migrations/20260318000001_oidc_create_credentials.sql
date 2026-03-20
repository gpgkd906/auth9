-- Unified credential store for all authentication factors
-- No foreign keys per TiDB architecture rules
-- Origin: auth9-oidc (consolidated into auth9-core migration system)

CREATE TABLE IF NOT EXISTS credentials (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    credential_type VARCHAR(32) NOT NULL,
    credential_data JSON NOT NULL,
    user_label VARCHAR(255),
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_credentials_user_id (user_id),
    INDEX idx_credentials_user_type (user_id, credential_type),
    UNIQUE INDEX idx_credentials_user_type_label (user_id, credential_type, user_label)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
