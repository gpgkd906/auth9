#!/usr/bin/env bash
# Generate a normal user JWT token for Auth9 Core API testing.
# Usage: ./gen-normal-user-token.sh
# Output: JWT token string to stdout

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [ ! -f "$SCRIPT_DIR/jwt_private_clean.key" ]; then
  echo "Error: jwt_private_clean.key not found in $SCRIPT_DIR" >&2
  exit 1
fi

USER_ID="67ca7acd-c073-4622-95a3-7ff12cc69c20"
EMAIL="qa-test-user@example.com"
NAME="QA Test User"

node -e "
const jwt = require('jsonwebtoken');
const fs = require('fs');
const path = require('path');

const keyPath = path.resolve('$SCRIPT_DIR', 'jwt_private_clean.key');
const privateKey = fs.readFileSync(keyPath, 'utf8');

const now = Math.floor(Date.now() / 1000);
const payload = {
  sub: '$USER_ID',
  email: '$EMAIL',
  name: '$NAME',
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
" 2>/dev/null