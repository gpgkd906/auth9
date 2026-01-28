-- Create permissions table
CREATE TABLE IF NOT EXISTS permissions (
    id CHAR(36) PRIMARY KEY,
    service_id CHAR(36) NOT NULL,
    code VARCHAR(100) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    
    UNIQUE KEY uk_permission_service_code (service_id, code),
    INDEX idx_permissions_service (service_id),
    
    CONSTRAINT fk_permissions_service FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
