const jwt = require('jsonwebtoken');
const fs = require('fs');
const crypto = require('crypto');

const path = require('path');

const projectRoot = path.resolve(__dirname, '..', '..', '..');
const keyPath = path.join(projectRoot, 'deploy', 'dev-certs', 'jwt', 'private.key');

const privateKey = fs.readFileSync(keyPath, 'utf8');

const userId = process.argv[2] || process.env.ADMIN_USER_ID;
const tenantId = process.argv[3] || process.env.TENANT_ID || '144237f8-76f0-4635-9aba-d235a6e0c6fb';
const serviceId = process.argv[4] || 'auth9-portal';

if (!userId) {
  try {
    userId = require('child_process').execSync(
      `mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;"`,
      { encoding: 'utf8', timeout: 5000 }
    ).trim();
  } catch (e) {
    console.error("Failed to query admin user ID from database:", e.message);
    process.exit(1);
  }
}

console.error("Using user ID:", userId);
console.error("Using tenant ID:", tenantId);

const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: userId,
  email: "admin@auth9.local",
  iss: "http://localhost:8080",
  aud: serviceId,
  token_type: "access",
  tenant_id: tenantId,
  roles: ["admin"],
  permissions: ["rbac:*", "user:*", "service:*", "action:*"],
  iat: now,
  exp: now + 3600
};

try {
  const token = jwt.sign(payload, privateKey, { algorithm: 'RS256', keyid: 'auth9-current' });
  process.stdout.write(token);
} catch (e) {
  console.error("JWT sign error:", e.message);
  process.exit(1);
}
