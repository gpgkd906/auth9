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

-- Seed login events for security alert detection (scenarios 2 & 3)
-- These prior events enable new_device and impossible_travel detection on next login.
DELETE FROM login_events WHERE user_id = '50587266-c621-42d7-9d3d-8fc8e0ed00ef';

-- Prior successful login from a known device (enables new_device detection on next login from different device)
INSERT INTO login_events (user_id, email, event_type, ip_address, user_agent, device_type, location, created_at) VALUES
('50587266-c621-42d7-9d3d-8fc8e0ed00ef', 'target@example.com', 'success', '203.0.113.10', 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Chrome/120.0', 'desktop', 'IP:203.0.113.10', DATE_SUB(NOW(), INTERVAL 1 DAY));

-- Recent successful login from location A (enables impossible_travel detection when next login has different location)
INSERT INTO login_events (user_id, email, event_type, ip_address, user_agent, device_type, location, created_at) VALUES
('50587266-c621-42d7-9d3d-8fc8e0ed00ef', 'target@example.com', 'success', '203.0.113.10', 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) Chrome/120.0', 'desktop', 'IP:203.0.113.10', DATE_SUB(NOW(), INTERVAL 10 MINUTE));

-- Clean up any existing seed security alerts
DELETE FROM security_alerts WHERE user_id = '50587266-c621-42d7-9d3d-8fc8e0ed00ef';
