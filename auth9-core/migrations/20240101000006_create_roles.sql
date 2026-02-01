-- Create roles table
CREATE TABLE IF NOT EXISTS roles (
    id CHAR(36) PRIMARY KEY,
    service_id CHAR(36) NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    parent_role_id CHAR(36),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    
    UNIQUE KEY uk_role_service_name (service_id, name),
    INDEX idx_roles_service (service_id),
    INDEX idx_roles_parent (parent_role_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
