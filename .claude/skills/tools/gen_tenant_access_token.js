const jwt = require('jsonwebtoken');
const fs = require('fs');
const crypto = require('crypto');

const path = require('path');
const keyPath = path.resolve(__dirname, 'jwt_private_clean.key');
const privateKey = fs.readFileSync(keyPath, 'utf8');

const userId = process.argv[2] || '16daa93d-06e8-479c-867d-f9b6184e06c7';
const tenantId = process.argv[3] || 'be469362-ee7f-480d-910d-75fbb8730bc4'; // auth9-platform

const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: userId,
  email: "admin@auth9.local",
  iss: "http://localhost:8080",
  aud: "auth9-portal",
  token_type: "access",
  tenant_id: tenantId,
  roles: ["admin"],
  permissions: ["rbac:*", "user:*", "service:*", "action:*"],
  iat: now,
  exp: now + 3600
};

try {
    const token = jwt.sign(payload, privateKey, { algorithm: 'RS256' });
    process.stdout.write(token);
} catch (e) {
    console.error("JWT sign error:", e.message);
    process.exit(1);
}
