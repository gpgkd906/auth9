-- WebAuthn credentials (native passkey storage)
-- No foreign keys per TiDB architecture rules

CREATE TABLE IF NOT EXISTS webauthn_credentials (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    credential_id VARCHAR(512) NOT NULL,
    credential_data JSON NOT NULL,
    user_label VARCHAR(255),
    aaguid VARCHAR(64),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NULL,
    INDEX idx_webauthn_credentials_user_id (user_id),
    UNIQUE INDEX idx_webauthn_credentials_credential_id (credential_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
