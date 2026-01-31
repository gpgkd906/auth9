-- System settings table for storing configurable settings
-- Settings can be encrypted for sensitive data like API keys and passwords

CREATE TABLE system_settings (
    id INT AUTO_INCREMENT PRIMARY KEY,
    category VARCHAR(100) NOT NULL COMMENT 'Setting category (e.g., email, auth, branding)',
    setting_key VARCHAR(100) NOT NULL COMMENT 'Setting key within the category',
    value JSON NOT NULL COMMENT 'Setting value as JSON',
    encrypted BOOLEAN DEFAULT FALSE COMMENT 'Whether the value is encrypted',
    description TEXT COMMENT 'Human-readable description of the setting',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE INDEX idx_category_key (category, setting_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Insert default email provider config (disabled by default)
INSERT INTO system_settings (category, setting_key, value, encrypted, description)
VALUES (
    'email',
    'provider',
    '{"type": "none"}',
    FALSE,
    'Email provider configuration (SMTP, SES, or Oracle Email Delivery)'
);
