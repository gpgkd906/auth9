-- Create tenant_users table (many-to-many relationship)
CREATE TABLE IF NOT EXISTS tenant_users (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    user_id CHAR(36) NOT NULL,
    role_in_tenant VARCHAR(50) NOT NULL DEFAULT 'member',
    joined_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE KEY uk_tenant_user (tenant_id, user_id),
    INDEX idx_tenant_users_tenant (tenant_id),
    INDEX idx_tenant_users_user (user_id),
    
    CONSTRAINT fk_tenant_users_tenant FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE,
    CONSTRAINT fk_tenant_users_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
