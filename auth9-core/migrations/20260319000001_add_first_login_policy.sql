-- Add first_login_policy to social_providers and enterprise_sso_connectors
-- Values: 'auto_merge', 'prompt_confirm', 'create_new'
-- Default: 'auto_merge' preserves backward compatibility

ALTER TABLE social_providers ADD COLUMN first_login_policy VARCHAR(20) NOT NULL DEFAULT 'auto_merge';
ALTER TABLE enterprise_sso_connectors ADD COLUMN first_login_policy VARCHAR(20) NOT NULL DEFAULT 'auto_merge';
