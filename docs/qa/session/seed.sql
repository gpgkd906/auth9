-- Session QA seed data (target user sessions)
-- Safe to re-run: deletes only known seed IDs, then inserts.
-- NOTE: Admin user is auto-created by the auth callback on first login.
--       Do NOT seed the admin user here to avoid keycloak_id conflicts.

-- Target user (seeded if missing)
INSERT IGNORE INTO users (id, keycloak_id, email, display_name, mfa_enabled, created_at, updated_at) VALUES
('50587266-c621-42d7-9d3d-8fc8e0ed00ef', 'ced89b18-8713-46ff-b1f0-d136b1f1cc78', 'target@example.com', 'Target User', 0, NOW(), NOW());

-- Remove existing seed sessions
DELETE FROM sessions WHERE id IN (
  't0000001-0001-0001-0001-000000000001'
);

-- Target user session (1 active)
INSERT INTO sessions (id, user_id, device_type, device_name, ip_address, location, last_active_at, created_at) VALUES
('t0000001-0001-0001-0001-000000000001', '50587266-c621-42d7-9d3d-8fc8e0ed00ef', 'desktop', 'Chrome on macOS', '203.0.113.10', 'Seattle, US', NOW(), NOW());
