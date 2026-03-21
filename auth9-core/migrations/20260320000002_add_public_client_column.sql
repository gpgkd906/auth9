-- Add public_client flag to clients table
ALTER TABLE clients ADD COLUMN public_client BOOLEAN NOT NULL DEFAULT FALSE;
