-- 1. Create clients table
CREATE TABLE IF NOT EXISTS clients (
    id CHAR(36) PRIMARY KEY,
    service_id CHAR(36) NOT NULL,
    client_id VARCHAR(255) NOT NULL UNIQUE,
    client_secret_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_clients_service (service_id),
    INDEX idx_clients_client_id (client_id),

    CONSTRAINT fk_clients_service FOREIGN KEY (service_id) REFERENCES services(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- 2. Migrate existing data
-- Note: UUID() generates a 36-char string in MySQL/TiDB
INSERT INTO clients (id, service_id, client_id, client_secret_hash, name, created_at)
SELECT UUID(), id, client_id, client_secret_hash, 'Legacy Key', created_at FROM services;

-- 3. Cleanup services table
-- Drop index on client_id first
DROP INDEX idx_services_client_id ON services;

-- Drop columns
ALTER TABLE services DROP COLUMN client_id;
ALTER TABLE services DROP COLUMN client_secret_hash;
