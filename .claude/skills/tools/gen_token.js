const jwt = require('jsonwebtoken');
const fs = require('fs');
const crypto = require('crypto');
const { execSync } = require('child_process');

const path = require('path');

// Resolve project root (3 levels up from .claude/skills/tools/)
const projectRoot = path.resolve(__dirname, '..', '..', '..');
const keyPath = path.join(projectRoot, 'deploy', 'dev-certs', 'jwt', 'private.key');

// Auto-generate key if missing
if (!fs.existsSync(keyPath)) {
    console.error("JWT dev key not found, generating...");
    execSync(path.join(projectRoot, 'scripts', 'gen-dev-keys.sh'), { stdio: 'inherit' });
}

const privateKey = fs.readFileSync(keyPath, 'utf8');

// Verify key is valid
try {
    const key = crypto.createPrivateKey(privateKey);
    console.error("Key is valid");
} catch (e) {
    console.error("Key validation error:", e.message);
    process.exit(1);
}

// Resolve user ID: CLI arg > env var > database query
let userId = process.argv[2] || process.env.ADMIN_USER_ID;

if (!userId) {
    try {
        userId = execSync(
            `mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;"`,
            { encoding: 'utf8', timeout: 5000 }
        ).trim();
    } catch (e) {
        console.error("Failed to query admin user ID from database:", e.message);
        console.error("Provide user ID via: node gen_token.js <user_id> or ADMIN_USER_ID env var");
        process.exit(1);
    }
}

if (!userId) {
    console.error("No admin user found in database for email 'admin@auth9.local'");
    console.error("Provide user ID via: node gen_token.js <user_id> or ADMIN_USER_ID env var");
    process.exit(1);
}

console.error("Using user ID:", userId);

const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: userId,
  email: "admin@auth9.local",
  name: "Admin User",
  iss: "http://localhost:8080",
  aud: "auth9",
  token_type: "identity",
  iat: now,
  exp: now + 3600  // 1小时后过期
};

try {
    const token = jwt.sign(payload, privateKey, { algorithm: 'RS256', keyid: 'auth9-current' });
    process.stdout.write(token);
} catch (e) {
    console.error("JWT sign error:", e.message);
    process.exit(1);
}
