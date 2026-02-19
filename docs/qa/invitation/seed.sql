-- Invitation QA seed data
-- Required before running docs/qa/invitation/*

SET NAMES utf8mb4 COLLATE utf8mb4_unicode_ci;

SET @tenant_id = '11111111-1111-4111-8111-111111111111';
SET @service_id = '22222222-2222-4222-8222-222222222222';
SET @role_admin_id = '33333333-3333-4333-8333-333333333333';
SET @role_editor_id = '44444444-4444-4444-8444-444444444444';
SET @role_viewer_id = '55555555-5555-4555-8555-555555555555';
SET @admin_user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1);

-- Verify admin user exists
SELECT @admin_user_id AS admin_user_id;

-- Cleanup existing seed data
DELETE FROM invitations WHERE tenant_id = @tenant_id;
DELETE FROM tenant_services WHERE tenant_id = @tenant_id AND service_id = @service_id;
DELETE FROM roles WHERE id IN (@role_admin_id, @role_editor_id, @role_viewer_id)
  OR (service_id = @service_id AND name IN ('Admin', 'Editor', 'Viewer'));
DELETE FROM tenant_users WHERE tenant_id = @tenant_id;
DELETE FROM tenants WHERE id = @tenant_id;

-- Seed tenant
INSERT INTO tenants (id, name, slug, settings, status)
VALUES (@tenant_id, 'Invitation Test Tenant', 'invitation-test', '{}', 'active')
ON DUPLICATE KEY UPDATE name = VALUES(name), settings = VALUES(settings), status = VALUES(status);

-- Ensure test service exists as a global/public service (tenant_id = NULL)
-- The get_enabled_services API only returns services where s.tenant_id IS NULL,
-- linked to tenants via the tenant_services join table.
DELETE FROM services WHERE id = @service_id;
INSERT INTO services (id, tenant_id, name, base_url, redirect_uris, logout_uris, status)
VALUES (@service_id, NULL, 'Invitation Test Service', 'http://localhost:3000', '[]', '[]', 'active');

-- Enable service for tenant
INSERT INTO tenant_services (tenant_id, service_id, enabled)
VALUES (@tenant_id, @service_id, TRUE)
ON DUPLICATE KEY UPDATE enabled = VALUES(enabled);

-- Seed roles
INSERT INTO roles (id, service_id, name, description)
VALUES
  (@role_admin_id, @service_id, 'Admin', 'Invitation QA admin role'),
  (@role_editor_id, @service_id, 'Editor', 'Invitation QA editor role'),
  (@role_viewer_id, @service_id, 'Viewer', 'Invitation QA viewer role')
ON DUPLICATE KEY UPDATE description = VALUES(description);

-- Make admin user a member (for "already member" scenario)
INSERT INTO tenant_users (id, tenant_id, user_id, role_in_tenant)
VALUES ('aaaaaaa1-aaaa-4aaa-8aaa-aaaaaaaaaaa1', @tenant_id, @admin_user_id, 'owner')
ON DUPLICATE KEY UPDATE role_in_tenant = VALUES(role_in_tenant);

-- Seed invitations for list/filter/revoke/delete scenarios
INSERT INTO invitations (id, tenant_id, email, role_ids, invited_by, token_hash, status, expires_at, accepted_at, created_at, updated_at)
VALUES
  ('66666666-6666-4666-8666-666666666666', @tenant_id, 'pending@example.com', JSON_ARRAY(@role_editor_id, @role_viewer_id), @admin_user_id, 'seed_hash_pending', 'pending', DATE_ADD(NOW(), INTERVAL 72 HOUR), NULL, NOW(), NOW()),
  ('77777777-7777-4777-8777-777777777777', @tenant_id, 'expired@example.com', JSON_ARRAY(@role_viewer_id), @admin_user_id, 'seed_hash_expired', 'pending', DATE_SUB(NOW(), INTERVAL 1 DAY), NULL, DATE_SUB(NOW(), INTERVAL 2 DAY), DATE_SUB(NOW(), INTERVAL 2 DAY)),
  ('88888888-8888-4888-8888-888888888888', @tenant_id, 'revoked@example.com', JSON_ARRAY(@role_admin_id), @admin_user_id, 'seed_hash_revoked', 'revoked', DATE_ADD(NOW(), INTERVAL 72 HOUR), NULL, NOW(), NOW()),
  ('99999999-9999-4999-8999-999999999999', @tenant_id, 'accepted@example.com', JSON_ARRAY(@role_viewer_id), @admin_user_id, 'seed_hash_accepted', 'accepted', DATE_ADD(NOW(), INTERVAL 72 HOUR), NOW(), DATE_SUB(NOW(), INTERVAL 1 DAY), NOW())
ON DUPLICATE KEY UPDATE
  status = VALUES(status),
  expires_at = VALUES(expires_at),
  accepted_at = VALUES(accepted_at),
  updated_at = VALUES(updated_at);
