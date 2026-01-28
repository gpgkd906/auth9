-- Create user_tenant_roles table (user role assignment within tenant)
CREATE TABLE IF NOT EXISTS user_tenant_roles (
    id CHAR(36) PRIMARY KEY,
    tenant_user_id CHAR(36) NOT NULL,
    role_id CHAR(36) NOT NULL,
    granted_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    granted_by CHAR(36),
    
    UNIQUE KEY uk_user_tenant_role (tenant_user_id, role_id),
    INDEX idx_user_tenant_roles_tenant_user (tenant_user_id),
    INDEX idx_user_tenant_roles_role (role_id),
    INDEX idx_user_tenant_roles_granted_by (granted_by),
    
    CONSTRAINT fk_user_tenant_roles_tenant_user FOREIGN KEY (tenant_user_id) REFERENCES tenant_users(id) ON DELETE CASCADE,
    CONSTRAINT fk_user_tenant_roles_role FOREIGN KEY (role_id) REFERENCES roles(id) ON DELETE CASCADE,
    CONSTRAINT fk_user_tenant_roles_granted_by FOREIGN KEY (granted_by) REFERENCES users(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
