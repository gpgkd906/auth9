-- User verification status (email verified, etc.)
-- No foreign keys per TiDB architecture rules
-- Origin: auth9-oidc (consolidated into auth9-core migration system)

CREATE TABLE IF NOT EXISTS user_verification_status (
    user_id CHAR(36) PRIMARY KEY,
    email_verified TINYINT(1) NOT NULL DEFAULT 0,
    email_verified_at TIMESTAMP NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
