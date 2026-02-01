-- Add password_policy column to tenants table
-- Structure: { min_length, require_uppercase, require_lowercase, require_numbers,
--              require_symbols, max_age_days, history_count, lockout_threshold, lockout_duration_mins }
ALTER TABLE tenants ADD COLUMN password_policy JSON;
