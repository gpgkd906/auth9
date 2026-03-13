CREATE TABLE IF NOT EXISTS malicious_ip_blacklist (
    id CHAR(36) PRIMARY KEY,
    ip_address VARCHAR(45) NOT NULL,
    reason VARCHAR(255) NULL,
    created_by CHAR(36) NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    UNIQUE INDEX idx_malicious_ip_blacklist_ip_address (ip_address),
    INDEX idx_malicious_ip_blacklist_created_by (created_by),
    INDEX idx_malicious_ip_blacklist_created_at (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
