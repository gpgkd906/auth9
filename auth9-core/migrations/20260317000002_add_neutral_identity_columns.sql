-- Add neutral identity columns while keeping legacy Keycloak columns during migration period.

ALTER TABLE users
    ADD COLUMN identity_subject VARCHAR(255) NULL AFTER keycloak_id;

UPDATE users
SET identity_subject = keycloak_id
WHERE identity_subject IS NULL AND keycloak_id IS NOT NULL;

ALTER TABLE users
    ADD UNIQUE INDEX idx_users_identity_subject_unique (identity_subject),
    ADD INDEX idx_users_identity_subject (identity_subject);

ALTER TABLE sessions
    ADD COLUMN provider_session_id VARCHAR(255) NULL AFTER keycloak_session_id;

UPDATE sessions
SET provider_session_id = keycloak_session_id
WHERE provider_session_id IS NULL AND keycloak_session_id IS NOT NULL;

ALTER TABLE sessions
    ADD INDEX idx_sessions_provider_session (provider_session_id);

ALTER TABLE enterprise_sso_connectors
    ADD COLUMN provider_alias VARCHAR(140) NULL AFTER keycloak_alias;

UPDATE enterprise_sso_connectors
SET provider_alias = keycloak_alias
WHERE provider_alias IS NULL AND keycloak_alias IS NOT NULL;

ALTER TABLE enterprise_sso_connectors
    ADD UNIQUE INDEX idx_enterprise_sso_provider_alias (provider_alias);
