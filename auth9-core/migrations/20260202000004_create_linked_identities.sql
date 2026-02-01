-- Create linked_identities table for tracking social/SSO identities
-- Note: IdP configuration is stored in Keycloak, this table is for audit/tracking only
CREATE TABLE IF NOT EXISTS linked_identities (
    id CHAR(36) PRIMARY KEY,
    user_id CHAR(36) NOT NULL,
    provider_type VARCHAR(50) NOT NULL,
    provider_alias VARCHAR(100) NOT NULL,
    external_user_id VARCHAR(255) NOT NULL,
    external_email VARCHAR(320),
    linked_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE INDEX idx_linked_provider_external (provider_alias, external_user_id),
    INDEX idx_linked_user_id (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
