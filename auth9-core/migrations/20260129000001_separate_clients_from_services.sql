-- Create clients table (API keys for services)
CREATE TABLE IF NOT EXISTS clients (
    id CHAR(36) PRIMARY KEY,
    service_id CHAR(36) NOT NULL,
    client_id VARCHAR(255) NOT NULL UNIQUE,
    client_secret_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    INDEX idx_clients_service (service_id),
    INDEX idx_clients_client_id (client_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
