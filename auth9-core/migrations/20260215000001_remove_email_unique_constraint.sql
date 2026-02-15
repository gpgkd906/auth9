-- Remove UNIQUE constraint on email column in users table
-- Rationale: With multiple Identity Providers (IdPs), different Keycloak users
-- may share the same email address. Users are uniquely identified by keycloak_id,
-- not email. The non-unique idx_users_email INDEX (from the original migration)
-- is retained for query performance.
ALTER TABLE users DROP INDEX email;
