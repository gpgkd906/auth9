-- Restore UNIQUE constraint on email to prevent race conditions in concurrent
-- user creation. The constraint was removed in 20260215000001 for multi-IdP,
-- but the application layer still enforces email uniqueness. Without the DB
-- constraint, concurrent requests can create duplicate email rows.

-- First remove any duplicate emails (keep oldest record)
DELETE u1 FROM users u1
INNER JOIN users u2
WHERE u1.email = u2.email
  AND u1.created_at > u2.created_at;

-- Add UNIQUE index
ALTER TABLE users ADD UNIQUE INDEX idx_users_email_unique (email);
