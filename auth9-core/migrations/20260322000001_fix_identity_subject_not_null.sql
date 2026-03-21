-- Fix NULL identity_subject values and enforce NOT NULL constraint.
-- Users created before the neutral identity migration or via direct SQL
-- may have NULL identity_subject, causing ColumnDecode errors.

-- Step 1: Backfill NULL identity_subject with a generated value
UPDATE users
SET identity_subject = CONCAT('legacy-', id)
WHERE identity_subject IS NULL;

-- Step 2: Enforce NOT NULL constraint going forward
ALTER TABLE users MODIFY COLUMN identity_subject VARCHAR(255) NOT NULL;
