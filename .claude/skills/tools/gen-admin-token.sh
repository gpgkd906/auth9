#!/usr/bin/env bash
# Generate an admin JWT token for Auth9 Core API testing.
# Usage: ./gen-admin-token.sh
# Output: JWT token string to stdout
#
# Requirements: node, jsonwebtoken npm package (available in project root)
# The token is valid for 1 hour, signed with RS256 using jwt_private_clean.key.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

if [ ! -f "$SCRIPT_DIR/jwt_private_clean.key" ]; then
  echo "Error: jwt_private_clean.key not found in $SCRIPT_DIR" >&2
  exit 1
fi

node "$SCRIPT_DIR/gen_token.js" 2>/dev/null
