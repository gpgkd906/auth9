-- Add federation event columns to login_events for provider-level event tracking.
-- Also widen event_type from ENUM to VARCHAR(50) to support new federation event types.
-- TiDB requires separate ALTER TABLE statements for MODIFY + ADD.

ALTER TABLE login_events MODIFY COLUMN event_type VARCHAR(50) NOT NULL;

ALTER TABLE login_events
  ADD COLUMN provider_alias VARCHAR(255) DEFAULT NULL,
  ADD COLUMN provider_type VARCHAR(50) DEFAULT NULL;

CREATE INDEX idx_login_events_provider ON login_events (provider_alias, provider_type);
