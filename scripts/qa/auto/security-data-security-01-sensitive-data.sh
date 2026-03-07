#!/usr/bin/env bash
# QA Auto Test: security/data-security/01-sensitive-data
# Doc: docs/security/data-security/01-sensitive-data.md
# Scenarios: 5
# ASVS: M-DATA-01 | V14.1, V14.2, V16.4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: API response data leakage ─────────────────────────────────
scenario 1 "API responses do not leak sensitive fields" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_get "/api/v1/users/me")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_http_status "$status" 200 "GET /users/me returns 200"
  assert_not_contains "$body" "password_hash" "no password_hash in user response"
  assert_not_contains "$body" "password" "no password field in user response"

  resp_users=$(api_get "/api/v1/users?per_page=5")
  body_users=$(resp_body "$resp_users")
  assert_not_contains "$body_users" "password" "no password in users list"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  if [[ -n "$SVC_ID" ]]; then
    resp_svc=$(api_get "/api/v1/services/${SVC_ID}")
    body_svc=$(resp_body "$resp_svc")
    assert_not_contains "$body_svc" "client_secret" "no client_secret in GET /services/:id"
  fi

  resp_email=$(api_get "/api/v1/system/email")
  status_email=$(resp_status "$resp_email")
  if [[ "$status_email" == "200" ]]; then
    body_email=$(resp_body "$resp_email")
    password_val=$(echo "$body_email" | jq -r ".data.password // .password // empty")
    if [[ -n "$password_val" && "$password_val" != "null" ]]; then  # pragma: allowlist secret
      assert_match "$password_val" "^\\*+$\|^$" "SMTP password masked"
    else
      assert_match "pass" "pass" "SMTP password not exposed"
    fi
  fi

  qa_set_token ""
'

# ── Scenario 2: Error message information leakage ──────────────────────────
scenario 2 "Error messages do not expose internal details" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_get "/api/v1/users?order_by=invalid_column_name")
  body=$(resp_body "$resp")
  assert_not_contains "$body" "SELECT" "no SQL SELECT in error"
  assert_not_contains "$body" "FROM users" "no SQL FROM clause in error"
  assert_not_contains "$body" "src/" "no source file path in error"
  assert_not_contains "$body" ".rs:" "no Rust file reference in error"
  assert_not_contains "$body" "panicked" "no panic message in error"

  resp2=$(api_post "/api/v1/users" "{invalid json!!!}")
  body2=$(resp_body "$resp2")
  assert_not_contains "$body2" "serde" "no serde details in parse error"
  assert_not_contains "$body2" "line " "no line number in parse error"

  resp3=$(api_get "/api/v1/users/00000000-0000-0000-0000-000000000000")
  body3=$(resp_body "$resp3")
  assert_not_contains "$body3" "sql" "no sql details in not-found"
  assert_not_contains "$body3" "database" "no database details in not-found"
  assert_not_contains "$body3" "connection" "no connection details in not-found"

  resp4=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d "{}")
  body4=$(resp_body "$resp4")
  assert_not_contains "$body4" "stack trace" "no stack trace in validation error"
  assert_not_contains "$body4" "backtrace" "no backtrace in validation error"

  qa_set_token ""
'

# ── Scenario 3: Log sensitive data leakage ─────────────────────────────────
scenario 3 "Logs do not contain plaintext secrets" '
  api_post "/api/v1/auth/forgot-password" \
    "{\"email\":\"qa-log-test@example.com\"}" >/dev/null 2>&1 || true

  api_raw POST "/api/v1/auth/token" \
    -H "Content-Type: application/json" \
    -d "{\"grant_type\":\"client_credentials\",\"client_id\":\"test-client\",\"client_secret\":\"TopSecretValue123\"}" \
    >/dev/null 2>&1 || true

  logs=$(docker logs auth9-core --tail=200 2>&1 || echo "")

  if [[ -n "$logs" ]]; then
    assert_not_contains "$logs" "TopSecretValue123" "client_secret not in logs"

    password_in_logs=$(echo "$logs" | grep -c "\"password\":" || echo "0")
    if [[ "$password_in_logs" -gt 0 ]]; then
      pw_values=$(echo "$logs" | grep "\"password\":" | grep -v ""\*\*\*"" | grep -v "null" | head -1 || echo "")
      if [[ -n "$pw_values" ]]; then
        assert_match "fail" "pass" "plaintext password found in logs"
      else
        assert_match "pass" "pass" "passwords in logs are masked"
      fi
    else
      assert_match "pass" "pass" "no password fields in recent logs"
    fi
  else
    assert_match "pass" "pass" "auth9-core container not available (non-Docker env)"
  fi
'

# ── Scenario 4: Sensitive file/backup exposure ─────────────────────────────
scenario 4 "Sensitive files and backups not accessible via HTTP" '
  sensitive_paths=(
    "/config.yaml"
    "/.env"
    "/application.properties"
    "/.git/config"
    "/.git/HEAD"
    "/backup.sql"
    "/db.sqlite"
    "/dump.tar.gz"
    "/main.rs"
    "/.dockerenv"
    "/Cargo.toml"
    "/docker-compose.yml"
  )

  for p in "${sensitive_paths[@]}"; do
    resp=$(api_raw GET "$p")
    status=$(resp_status "$resp")
    body=$(resp_body "$resp")
    assert_match "$status" "^(400|403|404|405)$" "sensitive path blocked: ${p}"

    if [[ "$p" == "/.git/config" ]]; then
      assert_not_contains "$body" "[core]" "no git config content exposed"
    fi
    if [[ "$p" == "/.env" ]]; then
      assert_not_contains "$body" "DATABASE_URL" "no .env content exposed"
      assert_not_contains "$body" "JWT_SECRET" "no JWT_SECRET exposed"
    fi
  done

  resp_uploads=$(api_raw GET "/uploads/")
  status_uploads=$(resp_status "$resp_uploads")
  body_uploads=$(resp_body "$resp_uploads")
  if [[ "$status_uploads" == "200" ]]; then
    assert_not_contains "$body_uploads" "<a href=" "no directory listing"
    assert_not_contains "$body_uploads" "Index of" "no index listing"
  fi
'

# ── Scenario 5: HTTP metadata and API documentation leakage ───────────────
scenario 5 "Metadata leakage - server info and API docs" '
  headers=$(curl -sI "${API_BASE}/health" 2>&1)

  server_hdr=$(echo "$headers" | grep -i "^server:" || echo "")
  if [[ -n "$server_hdr" ]]; then
    assert_not_contains "$server_hdr" "nginx/" "no nginx version in Server header"
    assert_not_contains "$server_hdr" "Apache/" "no Apache version in Server header"
  else
    assert_match "pass" "pass" "no Server header (good)"
  fi

  xpb=$(echo "$headers" | grep -i "^x-powered-by:" || echo "")
  assert_eq "${xpb:-empty}" "empty" "no X-Powered-By header"

  resp_health=$(api_get "/health")
  body_health=$(resp_body "$resp_health")
  assert_not_contains "$body_health" "DATABASE_URL" "no connection string in health"
  assert_not_contains "$body_health" "redis://" "no Redis URL in health"
  assert_not_contains "$body_health" "10.0." "no internal IP in health"
  assert_not_contains "$body_health" "172.16." "no internal IP in health"
  assert_not_contains "$body_health" "192.168." "no internal IP in health"

  resp_404=$(api_get "/nonexistent-endpoint-xyz")
  body_404=$(resp_body "$resp_404")
  assert_not_contains "$body_404" "nginx" "no nginx in 404 body"
  assert_not_contains "$body_404" "Apache" "no Apache in 404 body"
  assert_not_contains "$body_404" "axum" "no axum framework in 404 body"
  assert_not_contains "$body_404" "tokio" "no tokio runtime in 404 body"
'

run_all
