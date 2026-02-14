#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [ ! -f "$SCRIPT_DIR/jwt_private_clean.key" ]; then
  echo "Error: jwt_private_clean.key not found in $SCRIPT_DIR" >&2
  exit 1
fi

USER_EMAIL="${1:-testnormal@example.com}"
USER_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM users WHERE email = '$USER_EMAIL' LIMIT 1;" 2>/dev/null || echo "")

if [ -z "$USER_ID" ]; then
  echo "Error: User '$USER_EMAIL' not found in database" >&2
  exit 1
fi

USER_NAME=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT display_name FROM users WHERE email = '$USER_EMAIL' LIMIT 1;" 2>/dev/null || echo "Normal User")

node -e "
const jwt = require('jsonwebtoken');
const fs = require('fs');
const path = require('path');

const keyPath = path.resolve('$SCRIPT_DIR', 'jwt_private_clean.key');
const privateKey = fs.readFileSync(keyPath, 'utf8');

const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: '$USER_ID',
  email: '$USER_EMAIL',
  name: '$USER_NAME',
  iss: 'http://localhost:8080',
  aud: 'auth9',
  iat: now,
  exp: now + 3600
};

try {
  const token = jwt.sign(payload, privateKey, { algorithm: 'RS256' });
  console.log(token);
} catch (e) {
  console.error('JWT sign error:', e.message);
  process.exit(1);
}
"
