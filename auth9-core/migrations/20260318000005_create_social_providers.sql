-- Social login provider configuration (Auth9-managed, replaces Keycloak identity providers)

CREATE TABLE IF NOT EXISTS social_providers (
    id CHAR(36) PRIMARY KEY,
    alias VARCHAR(100) NOT NULL,
    display_name VARCHAR(255),
    provider_type VARCHAR(20) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    trust_email BOOLEAN NOT NULL DEFAULT FALSE,
    store_token BOOLEAN NOT NULL DEFAULT FALSE,
    link_only BOOLEAN NOT NULL DEFAULT FALSE,
    config JSON NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE INDEX idx_social_providers_alias (alias),
    INDEX idx_social_providers_enabled (enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
