-- Create actions table for Auth9 Actions system
CREATE TABLE IF NOT EXISTS actions (
    id CHAR(36) PRIMARY KEY,
    tenant_id CHAR(36) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    trigger_id VARCHAR(50) NOT NULL,
    script TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    execution_order INT NOT NULL DEFAULT 0,
    timeout_ms INT NOT NULL DEFAULT 3000,
    last_executed_at TIMESTAMP NULL,
    execution_count BIGINT NOT NULL DEFAULT 0,
    error_count BIGINT NOT NULL DEFAULT 0,
    last_error TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,

    INDEX idx_actions_tenant_id (tenant_id),
    INDEX idx_actions_trigger_id (trigger_id),
    INDEX idx_actions_enabled (enabled),
    UNIQUE INDEX uk_tenant_trigger_name (tenant_id, trigger_id, name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Create action executions audit log table
CREATE TABLE IF NOT EXISTS action_executions (
    id CHAR(36) PRIMARY KEY,
    action_id CHAR(36) NOT NULL,
    tenant_id CHAR(36) NOT NULL,
    trigger_id VARCHAR(50) NOT NULL,
    user_id CHAR(36),
    success BOOLEAN NOT NULL,
    duration_ms INT NOT NULL,
    error_message TEXT,
    executed_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_action_executions_action_id (action_id),
    INDEX idx_action_executions_tenant_id (tenant_id),
    INDEX idx_action_executions_executed_at (executed_at),
    INDEX idx_action_executions_user_id (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
