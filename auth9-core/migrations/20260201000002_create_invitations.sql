-- Invitations table for user invite management

CREATE TABLE invitations (
    id CHAR(36) PRIMARY KEY COMMENT 'UUID of the invitation',
    tenant_id CHAR(36) NOT NULL COMMENT 'Tenant the user is being invited to',
    email VARCHAR(255) NOT NULL COMMENT 'Email address of the invitee',
    role_ids JSON NOT NULL COMMENT 'Array of role IDs to assign upon acceptance',
    invited_by CHAR(36) NOT NULL COMMENT 'User ID who created the invitation',
    token_hash VARCHAR(255) NOT NULL COMMENT 'Argon2 hash of the invitation token',
    status ENUM('pending', 'accepted', 'expired', 'revoked') DEFAULT 'pending' COMMENT 'Current status of the invitation',
    expires_at TIMESTAMP NOT NULL COMMENT 'When the invitation expires',
    accepted_at TIMESTAMP NULL COMMENT 'When the invitation was accepted',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    INDEX idx_tenant_id (tenant_id),
    INDEX idx_email (email),
    INDEX idx_status (status),
    INDEX idx_expires_at (expires_at),

    CONSTRAINT fk_invitations_tenant
        FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
