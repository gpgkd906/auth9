-- Phase 6: Drop legacy Keycloak columns now that neutral columns are populated and stable

-- R5.2: Rename keycloak_client_id → backend_client_id in saml_applications
ALTER TABLE saml_applications
    ADD COLUMN backend_client_id VARCHAR(255) NULL AFTER attribute_mappings;

UPDATE saml_applications
SET backend_client_id = keycloak_client_id
WHERE backend_client_id IS NULL AND keycloak_client_id IS NOT NULL;

ALTER TABLE saml_applications
    MODIFY COLUMN backend_client_id VARCHAR(255) NOT NULL;

ALTER TABLE saml_applications
    ADD UNIQUE INDEX idx_saml_app_backend_client (backend_client_id);

-- R6.2: DROP legacy columns
ALTER TABLE users DROP COLUMN keycloak_id;
ALTER TABLE sessions DROP COLUMN keycloak_session_id;
ALTER TABLE enterprise_sso_connectors DROP COLUMN keycloak_alias;

ALTER TABLE saml_applications
    DROP INDEX idx_saml_app_kc_client;

ALTER TABLE saml_applications
    DROP COLUMN keycloak_client_id;
