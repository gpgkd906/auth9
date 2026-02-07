const jwt = require('jsonwebtoken');
const fs = require('fs');
const crypto = require('crypto');

const path = require('path');
const keyPath = path.resolve(__dirname, 'jwt_private_clean.key');
const privateKey = fs.readFileSync(keyPath, 'utf8');

// Verify key is valid
try {
    const key = crypto.createPrivateKey(privateKey);
    console.error("Key is valid");
} catch (e) {
    console.error("Key validation error:", e.message);
    process.exit(1);
}

const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: "746ceba8-3ddf-4a8b-b021-a1337b7a1a35",
  email: "admin@auth9.local",
  name: "Admin User",
  iss: "http://localhost:8080",
  aud: "auth9",
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