#!/usr/bin/env bash
# QA Environment Setup Library
# Provides token generation, DB helpers, and HTTP request wrappers.
#
# Usage: source this file (usually via runner.sh which sources it for you).
#   source "$(dirname "${BASH_SOURCE[0]}")/setup.sh"

set -euo pipefail

_QA_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_QA_PROJECT_ROOT="$(cd "$_QA_LIB_DIR/../../.." && pwd)"

# ---------------------------------------------------------------------------
# Environment variables (override via env before sourcing)
# ---------------------------------------------------------------------------
API_BASE="${API_BASE:-http://localhost:8080}"
MYSQL_HOST="${MYSQL_HOST:-127.0.0.1}"
MYSQL_PORT="${MYSQL_PORT:-4000}"
MYSQL_USER="${MYSQL_USER:-root}"
MYSQL_DB="${MYSQL_DB:-auth9}"

_TOKEN_TOOLS_DIR="$_QA_PROJECT_ROOT/.claude/skills/tools"
_JWT_PRIVATE_KEY="${JWT_PRIVATE_KEY:-$_QA_PROJECT_ROOT/deploy/dev-certs/jwt/private.key}"

# Cached tokens (generated lazily)
_ADMIN_TOKEN=""
_TENANT_TOKEN=""

# ---------------------------------------------------------------------------
# Agent-Tool helpers: accept env vars for IDs, fall back to DB lookup
# ---------------------------------------------------------------------------
qa_get_tenant_id() {
  if [[ -n "${QA_TENANT_ID:-}" ]]; then
    echo "$QA_TENANT_ID"
    return
  fi
  db_query "SELECT id FROM tenants ORDER BY created_at ASC LIMIT 1;" | tr -d "[:space:]"
}

qa_get_service_id() {
  if [[ -n "${QA_SERVICE_ID:-}" ]]; then
    echo "$QA_SERVICE_ID"
    return
  fi
  local tid
  tid=$(qa_get_tenant_id)
  local sid
  sid=$(db_query "SELECT id FROM services WHERE tenant_id = '${tid}' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$sid" ]]; then
    sid=$(db_query "SELECT id FROM services WHERE tenant_id IS NOT NULL LIMIT 1;" | tr -d "[:space:]")
  fi
  if [[ -z "$sid" ]]; then
    sid=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  fi
  echo "$sid"
}

# Populates _QA_SVC_ID and _QA_SVC_TENANT. Call directly (not in subshell).
# Then use: $SVC_ID=$_QA_SVC_ID; TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
_QA_SVC_ID=""
_QA_SVC_TENANT=""
qa_setup_service_with_tenant() {
  if [[ -n "${QA_SERVICE_ID:-}" ]]; then
    _QA_SVC_ID="$QA_SERVICE_ID"
    _QA_SVC_TENANT="${QA_TENANT_ID:-$(db_query "SELECT tenant_id FROM services WHERE id = '${QA_SERVICE_ID}' LIMIT 1;" | tr -d "[:space:]")}"
    return
  fi
  local row
  row=$(db_query "SELECT id, tenant_id FROM services WHERE tenant_id IS NOT NULL LIMIT 1;")
  _QA_SVC_ID=$(echo "$row" | awk '{print $1}' | tr -d "[:space:]")
  _QA_SVC_TENANT=$(echo "$row" | awk '{print $2}' | tr -d "[:space:]")
}

qa_get_admin_id() {
  if [[ -n "${QA_ADMIN_ID:-}" ]]; then
    echo "$QA_ADMIN_ID"
    return
  fi
  db_query "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;" | tr -d "[:space:]"
}

gen_token_for_tenant() {
  local tenant_id="$1"
  local admin_id
  admin_id=$(qa_get_admin_id)
  gen_tenant_token "$admin_id" "$tenant_id"
}

# ---------------------------------------------------------------------------
# Dependency check
# ---------------------------------------------------------------------------
require_bin() {
  if ! command -v "$1" &>/dev/null; then
    echo "Missing required command: $1" >&2
    exit 2
  fi
}

# ---------------------------------------------------------------------------
# Database helpers
# ---------------------------------------------------------------------------
db_query() {
  mysql -h "$MYSQL_HOST" -P "$MYSQL_PORT" -u "$MYSQL_USER" "$MYSQL_DB" -N -e "$1"
}

db_query_silent() {
  mysql -h "$MYSQL_HOST" -P "$MYSQL_PORT" -u "$MYSQL_USER" "$MYSQL_DB" -N -e "$1" &>/dev/null
}

db_exec() {
  mysql -h "$MYSQL_HOST" -P "$MYSQL_PORT" -u "$MYSQL_USER" "$MYSQL_DB" -e "$1" &>/dev/null
}

# ---------------------------------------------------------------------------
# Token generation
# ---------------------------------------------------------------------------
gen_admin_token() {
  if [[ -n "$_ADMIN_TOKEN" ]]; then
    echo "$_ADMIN_TOKEN"
    return
  fi
  _ADMIN_TOKEN=$("$_TOKEN_TOOLS_DIR/gen-admin-token.sh" 2>/dev/null)
  echo "$_ADMIN_TOKEN"
}

gen_tenant_token() {
  local user_id="${1:-}" tenant_id="${2:-}"
  node "$_TOKEN_TOOLS_DIR/gen_tenant_access_token.js" "$user_id" "$tenant_id" 2>/dev/null
}

gen_default_admin_token() {
  if [[ -n "${QA_TOKEN:-}" ]]; then
    echo "$QA_TOKEN"
    return
  fi
  if [[ -n "$_TENANT_TOKEN" ]]; then
    echo "$_TENANT_TOKEN"
    return
  fi
  local admin_id tenant_id
  admin_id=$(qa_get_admin_id)
  tenant_id=$(qa_get_tenant_id)
  _TENANT_TOKEN=$(gen_tenant_token "$admin_id" "$tenant_id")
  echo "$_TENANT_TOKEN"
}

gen_identity_token() {
  local user_id="$1" email="$2"
  node -e '
const jwt=require("jsonwebtoken");
const fs=require("fs");
const now=Math.floor(Date.now()/1000);
const privateKey=fs.readFileSync(process.argv[1],"utf8");
const payload={
  sub: process.argv[2],
  email: process.argv[3],
  iss: "http://localhost:8080",
  aud: "auth9",
  token_type: "identity",
  iat: now,
  exp: now + 3600,
  sid: "sid-" + process.argv[2].slice(0,8)
};
process.stdout.write(jwt.sign(payload, privateKey, {algorithm:"RS256", keyid:"auth9-current"}));
' "$_JWT_PRIVATE_KEY" "$user_id" "$email" 2>/dev/null
}

# ---------------------------------------------------------------------------
# HTTP request wrappers
# Returns: first line = status code, remaining lines = body
# Usage:
#   resp=$(api_get /health)
#   status=$(resp_status "$resp")
#   body=$(resp_body "$resp")
# ---------------------------------------------------------------------------
_QA_AUTH_TOKEN=""

qa_set_token() {
  _QA_AUTH_TOKEN="$1"
}

_build_curl_args() {
  _CURL_AUTH_ARGS=()
  if [[ -n "$_QA_AUTH_TOKEN" ]]; then
    _CURL_AUTH_ARGS+=(-H "Authorization: Bearer $_QA_AUTH_TOKEN")
  fi
}

api_get() {
  local path="$1"
  _build_curl_args
  local body status_code
  body=$(curl -s -w '\n%{http_code}' "${_CURL_AUTH_ARGS[@]+"${_CURL_AUTH_ARGS[@]}"}" "${API_BASE}${path}")
  status_code=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '$d')
  printf '%s\n%s' "$status_code" "$body"
}

api_post() {
  local path="$1"; local data="${2:-"{}"}"
  _build_curl_args
  local body status_code
  body=$(curl -s -w '\n%{http_code}' -X POST \
    "${_CURL_AUTH_ARGS[@]+"${_CURL_AUTH_ARGS[@]}"}" \
    -H "Content-Type: application/json" \
    -d "$data" \
    "${API_BASE}${path}")
  status_code=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '$d')
  printf '%s\n%s' "$status_code" "$body"
}

api_put() {
  local path="$1"; local data="${2:-"{}"}"
  _build_curl_args
  local body status_code
  body=$(curl -s -w '\n%{http_code}' -X PUT \
    "${_CURL_AUTH_ARGS[@]+"${_CURL_AUTH_ARGS[@]}"}" \
    -H "Content-Type: application/json" \
    -d "$data" \
    "${API_BASE}${path}")
  status_code=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '$d')
  printf '%s\n%s' "$status_code" "$body"
}

api_patch() {
  local path="$1"; local data="${2:-"{}"}"
  _build_curl_args
  local body status_code
  body=$(curl -s -w '\n%{http_code}' -X PATCH \
    "${_CURL_AUTH_ARGS[@]+"${_CURL_AUTH_ARGS[@]}"}" \
    -H "Content-Type: application/json" \
    -d "$data" \
    "${API_BASE}${path}")
  status_code=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '$d')
  printf '%s\n%s' "$status_code" "$body"
}

api_delete() {
  local path="$1"
  _build_curl_args
  local body status_code
  body=$(curl -s -w '\n%{http_code}' -X DELETE \
    "${_CURL_AUTH_ARGS[@]+"${_CURL_AUTH_ARGS[@]}"}" \
    "${API_BASE}${path}")
  status_code=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '$d')
  printf '%s\n%s' "$status_code" "$body"
}

api_raw() {
  local method="$1" path="$2"
  shift 2
  local body status_code
  body=$(curl -s -w '\n%{http_code}' -X "$method" "$@" "${API_BASE}${path}")
  status_code=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '$d')
  printf '%s\n%s' "$status_code" "$body"
}

# ---------------------------------------------------------------------------
# Response helpers
# ---------------------------------------------------------------------------
resp_status() {
  echo "$1" | head -1
}

resp_body() {
  echo "$1" | tail -n +2
}
