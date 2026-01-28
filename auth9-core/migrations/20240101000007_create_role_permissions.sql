-- Create role_permissions table (many-to-many)
CREATE TABLE IF NOT EXISTS role_permissions (
    role_id CHAR(36) NOT NULL,
    permission_id CHAR(36) NOT NULL,
    
    PRIMARY KEY (role_id, permission_id),
    INDEX idx_role_permissions_role (role_id),
    INDEX idx_role_permissions_permission (permission_id),
    
    CONSTRAINT fk_role_permissions_role FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    CONSTRAINT fk_role_permissions_permission FOREIGN KEY (permission_id) REFERENCES permissions(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
