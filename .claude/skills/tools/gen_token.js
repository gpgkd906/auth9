const jwt = require('jsonwebtoken');
const fs = require('fs');
const crypto = require('crypto');
const { execSync } = require('child_process');

const path = require('path');

const projectRoot = path.resolve(__dirname, '..', '..', '..');
const envPath = path.join(projectRoot, '.env');

function loadEnv(envPath) {
    const envContent = fs.readFileSync(envPath, 'utf8');
    const env = {};
    envContent.split('\n').forEach(line => {
        const match = line.match(/^([^=]+)=(.*)$/);
        if (match) {
            env[match[1].trim()] = match[2].trim();
        }
    });
    return env;
}

const env = loadEnv(envPath);
let privateKey = env.JWT_PRIVATE_KEY || '';
if (privateKey) {
    privateKey = privateKey.replace(/\\n/g, '\n');
} else {
    const keyPath = path.join(projectRoot, 'deploy', 'dev-certs', 'jwt', 'private.key');
    if (!fs.existsSync(keyPath)) {
        console.error("JWT dev key not found, generating...");
        execSync(path.join(projectRoot, 'scripts', 'gen-dev-keys.sh'), { stdio: 'inherit' });
    }
    privateKey = fs.readFileSync(keyPath, 'utf8');
}

// Verify key is valid
try {
    const key = crypto.createPrivateKey(privateKey);
    console.error("Key is valid");
} catch (e) {
    console.error("Key validation error:", e.message);
    process.exit(1);
}

// Parse CLI args: supports positional user_id and --type=identity|access flag
let explicitUserId = null;
let tokenType = "access"; // default to access token for SCIM/API tests
let tenantId = process.env.TENANT_ID || null;

for (let i = 2; i < process.argv.length; i++) {
    const arg = process.argv[i];
    if (arg === '--type' && process.argv[i + 1]) {
        tokenType = process.argv[++i];
    } else if (arg.startsWith('--type=')) {
        tokenType = arg.split('=')[1];
    } else if (arg === '--tenant-id' && process.argv[i + 1]) {
        tenantId = process.argv[++i];
    } else if (arg.startsWith('--tenant-id=')) {
        tenantId = arg.split('=')[1];
    } else if (!arg.startsWith('--')) {
        explicitUserId = arg;
    }
}

if (!["identity", "access"].includes(tokenType)) {
    console.error("Invalid token type:", tokenType, '(must be "identity" or "access")');
    process.exit(1);
}

// Resolve user ID: CLI arg > env var > database query
let userId = explicitUserId || process.env.ADMIN_USER_ID;

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
console.error("Token type:", tokenType);

// Resolve tenant_id for access tokens
if (tokenType === "access" && !tenantId) {
    try {
        tenantId = execSync(
            `mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT tenant_id FROM tenant_users WHERE user_id = '${userId}' LIMIT 1;"`,
            { encoding: 'utf8', timeout: 5000 }
        ).trim();
    } catch (e) {
        console.error("Warning: could not resolve tenant_id from database:", e.message);
        console.error("Set TENANT_ID env var or pass --tenant-id=<id> for access tokens");
    }
}

if (tokenType === "access" && tenantId) {
    console.error("Using tenant ID:", tenantId);
}

const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: userId,
  email: "admin@auth9.local",
  name: "Admin User",
  iss: "http://localhost:8080",
  aud: "auth9",
  token_type: tokenType,
  iat: now,
  exp: now + 3600  // 1小时后过期
};

// Include tenant_id claim for access tokens
if (tokenType === "access" && tenantId) {
    payload.tenant_id = tenantId;
}

try {
    const token = jwt.sign(payload, privateKey, { algorithm: 'RS256', keyid: 'auth9-current' });
    process.stdout.write(token);
} catch (e) {
    console.error("JWT sign error:", e.message);
    process.exit(1);
}
