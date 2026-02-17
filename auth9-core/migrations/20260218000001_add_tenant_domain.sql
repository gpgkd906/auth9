-- Add domain column to tenants table for B2B organization onboarding
ALTER TABLE tenants ADD COLUMN domain VARCHAR(255) DEFAULT NULL;

-- Index for domain lookups
CREATE INDEX idx_tenants_domain ON tenants (domain);
