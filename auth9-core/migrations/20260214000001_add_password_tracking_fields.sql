-- Add password tracking fields to users table
-- password_changed_at: tracks when the password was last changed (for max_age_days enforcement)
-- locked_until: tracks account lockout expiry time

ALTER TABLE users ADD COLUMN password_changed_at TIMESTAMP NULL AFTER mfa_enabled;
ALTER TABLE users ADD COLUMN locked_until TIMESTAMP NULL AFTER password_changed_at;

-- Index for efficient password age queries
CREATE INDEX idx_users_password_changed_at ON users (password_changed_at);
