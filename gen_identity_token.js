const jwt = require('jsonwebtoken');
const fs = require('fs');

const privateKey = fs.readFileSync('.claude/skills/tools/jwt_private_clean.key', 'utf8');
const now = Math.floor(Date.now() / 1000);

const payload = {
  sub: "cbe261fa-f95b-4ff1-bbbd-919e320435c4",
  email: "attacker@test.com",
  iss: "http://localhost:8080",
  aud: "auth9",
  token_type: "identity",
  iat: now,
  exp: now + 3600
};

const token = jwt.sign(payload, privateKey, { algorithm: 'RS256', keyid: 'auth9-current' });
console.log(token);
