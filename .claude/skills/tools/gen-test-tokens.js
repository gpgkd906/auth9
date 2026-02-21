#!/usr/bin/env node
/**
 * Generate different types of JWT tokens for Auth9 security testing
 * Usage:
 *   node gen-test-tokens.js [type] [--tenant-id <uuid>] [--user-id <uuid>]
 *
 * Types: platform-admin, identity-user, tenant-access, tenant-owner, service-client
 */

const jwt = require('jsonwebtoken');
const fs = require('fs');
const path = require('path');

const keyPath = path.resolve(__dirname, 'jwt_private_clean.key');
const privateKey = fs.readFileSync(keyPath, 'utf8');

const now = Math.floor(Date.now() / 1000);
const issuer = "http://localhost:8080";
const ttl = 3600; // 1 hour

// Test data
const DEFAULT_TENANT_ID = "73fa0f3b-ee55-44a1-8fde-787b7a925107"; // Demo Organization (may differ per environment)
const DEFAULT_USER_ID = "47116b28-b60b-4b73-a9d0-baace9245cf0";
// Non-admin user ID for identity-user/tenant-access tokens. Must NOT match any real
// admin user in the DB, otherwise is_platform_admin_with_db() will grant platform
// admin privileges via the auth9-platform tenant role check, bypassing all isolation.
const NON_ADMIN_USER_ID = "00000000-0000-0000-0000-000000000099";
const SERVICE_ID = "00000000-0000-0000-0000-000000000001"; // Valid UUID for testing

function parseArgs(argv) {
    const out = { type: argv[2] || 'platform-admin' };
    for (let i = 3; i < argv.length; i++) {
        const arg = argv[i];
        if (arg === '--tenant-id') out.tenantId = argv[++i];
        else if (arg === '--user-id') out.userId = argv[++i];
    }
    return out;
}

function generateToken(type, { tenantId, userId }) {
    let payload;
    const TENANT_ID = tenantId || process.env.AUTH9_TEST_TENANT_ID || process.env.TENANT_ID || DEFAULT_TENANT_ID;
    const ADMIN_USER_ID = userId || process.env.AUTH9_TEST_USER_ID || process.env.USER_ID || DEFAULT_USER_ID;

    switch (type) {
        case 'platform-admin':
            // Platform admin Identity Token (email in PLATFORM_ADMIN_EMAILS)
            payload = {
                sub: ADMIN_USER_ID,
                email: "admin@auth9.local",
                name: "Platform Admin",
                iss: issuer,
                aud: "auth9",
                iat: now,
                exp: now + ttl
            };
            break;

        case 'identity-user':
            // Regular user Identity Token (NOT a platform admin)
            // Uses NON_ADMIN_USER_ID to avoid DB-based platform admin bypass
            payload = {
                sub: userId || NON_ADMIN_USER_ID,
                email: "regular-user@example.com",
                name: "Regular User",
                iss: issuer,
                aud: "auth9",
                iat: now,
                exp: now + ttl
            };
            break;

        case 'tenant-access':
            // Tenant Access Token (for tenant member, not owner)
            // Uses NON_ADMIN_USER_ID to avoid DB-based platform admin bypass
            payload = {
                sub: userId || NON_ADMIN_USER_ID,
                email: "regular-user@example.com",
                iss: issuer,
                aud: "test-service-client-id", // Service client_id as audience
                tenant_id: TENANT_ID,
                roles: ["member"],
                permissions: ["read:profile"],
                iat: now,
                exp: now + ttl
            };
            break;

        case 'tenant-owner':
            // Tenant Owner Access Token
            payload = {
                sub: ADMIN_USER_ID,
                email: "admin@auth9.local",
                iss: issuer,
                aud: "test-service-client-id",
                tenant_id: TENANT_ID,
                roles: ["owner", "admin"],
                permissions: ["*"],
                iat: now,
                exp: now + ttl
            };
            break;

        case 'service-client':
            // Service Client Token
            payload = {
                sub: SERVICE_ID,
                email: `service+${SERVICE_ID}@auth9.local`,
                iss: issuer,
                aud: "auth9-service", // Distinct audience for service tokens
                tenant_id: TENANT_ID,
                iat: now,
                exp: now + ttl
            };
            break;

        default:
            console.error(`Unknown token type: ${type}`);
            console.error('Valid types: platform-admin, identity-user, tenant-access, tenant-owner, service-client');
            process.exit(1);
    }

    const token = jwt.sign(payload, privateKey, { algorithm: 'RS256' });
    return token;
}

const { type: tokenType, tenantId, userId } = parseArgs(process.argv);
const token = generateToken(tokenType, { tenantId, userId });
process.stdout.write(token);
