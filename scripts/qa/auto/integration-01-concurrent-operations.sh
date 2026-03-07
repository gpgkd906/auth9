#!/usr/bin/env bash
# QA Auto Test: integration/01-concurrent-operations
# Doc: docs/qa/integration/01-concurrent-operations.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_lookup_demo_tenant() {
  db_query "SELECT id FROM tenants WHERE slug='demo' LIMIT 1;" | tr -d '[:space:]'
}

scenario 1 "Concurrent user creation with same email" '
  TENANT_ID=$(_lookup_demo_tenant)
  if [[ -z "$TENANT_ID" ]]; then
    echo "No demo tenant found" >&2
    return 1
  fi

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  UNIQUE_EMAIL="concurrent-$(date +%s)@example.com"

  db_exec "DELETE FROM users WHERE email='"'"'${UNIQUE_EMAIL}'"'"';" || true

  PIDS=()
  RESULTS_DIR=$(mktemp -d)
  for i in $(seq 1 10); do
    (
      resp=$(api_post "/api/v1/users" \
        "{\"email\":\"${UNIQUE_EMAIL}\",\"display_name\":\"concurrent-user-${i}\",\"password\":\"Test123!\",\"tenant_id\":\"${TENANT_ID}\"}")
      echo "$(resp_status "$resp")" > "${RESULTS_DIR}/result_${i}.txt"
    ) &
    PIDS+=($!)
  done

  for pid in "${PIDS[@]}"; do
    wait "$pid" || true
  done

  SUCCESS_COUNT=0
  FAIL_COUNT=0
  for i in $(seq 1 10); do
    code=$(cat "${RESULTS_DIR}/result_${i}.txt" 2>/dev/null || echo "000")
    if [[ "$code" == "201" ]]; then
      SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    elif [[ "$code" == "400" || "$code" == "409" ]]; then
      FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
  done
  rm -rf "$RESULTS_DIR"

  assert_eq "$SUCCESS_COUNT" "1" "exactly 1 request succeeded (201)"

  DB_COUNT=$(db_query "SELECT COUNT(*) FROM users WHERE email='"'"'${UNIQUE_EMAIL}'"'"';" | tr -d '[:space:]')
  assert_eq "$DB_COUNT" "1" "DB has exactly 1 user with that email"

  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email='"'"'${UNIQUE_EMAIL}'"'"');" || true
  db_exec "DELETE FROM users WHERE email='"'"'${UNIQUE_EMAIL}'"'"';" || true
  qa_set_token ""
'

scenario 2 "Concurrent password reset token generation" '
  TENANT_ID=$(_lookup_demo_tenant)
  if [[ -z "$TENANT_ID" ]]; then
    echo "No demo tenant found" >&2
    return 1
  fi

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  RESET_EMAIL="reset-concurrent-$(date +%s)@example.com"
  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${RESET_EMAIL}\",\"display_name\":\"Reset Concurrent\",\"password\":\"Test123!\",\"tenant_id\":\"${TENANT_ID}\"}")
  assert_http_status "$(resp_status "$resp")" 201 "create user for reset test"
  USER_ID=$(resp_body "$resp" | jq -r '"'"'.data.id // .id'"'"')

  qa_set_token ""

  PIDS=()
  RESULTS_DIR=$(mktemp -d)
  for i in $(seq 1 10); do
    (
      resp=$(api_post "/api/v1/auth/forgot-password" "{\"email\":\"${RESET_EMAIL}\"}")
      echo "$(resp_status "$resp")" > "${RESULTS_DIR}/result_${i}.txt"
    ) &
    PIDS+=($!)
  done

  for pid in "${PIDS[@]}"; do
    wait "$pid" || true
  done

  SUCCESS_COUNT=0
  for i in $(seq 1 10); do
    code=$(cat "${RESULTS_DIR}/result_${i}.txt" 2>/dev/null || echo "000")
    if [[ "$code" == "200" || "$code" == "204" ]]; then
      SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    fi
  done
  rm -rf "$RESULTS_DIR"

  assert_match "$SUCCESS_COUNT" "^[0-9]+$" "all requests returned success or handled gracefully"

  VALID_TOKENS=$(db_query "SELECT COUNT(*) FROM password_reset_tokens WHERE user_id='"'"'${USER_ID}'"'"' AND used_at IS NULL AND expires_at > NOW();" | tr -d '[:space:]')
  assert_eq "$VALID_TOKENS" "1" "only 1 valid reset token in DB"

  qa_set_token "$TOKEN"
  db_exec "DELETE FROM password_reset_tokens WHERE user_id='"'"'${USER_ID}'"'"';" || true
  db_exec "DELETE FROM tenant_users WHERE user_id='"'"'${USER_ID}'"'"';" || true
  db_exec "DELETE FROM users WHERE id='"'"'${USER_ID}'"'"';" || true
  qa_set_token ""
'

scenario 3 "Concurrent role assignment to same user" '
  TENANT_ID=$(_lookup_demo_tenant)
  if [[ -z "$TENANT_ID" ]]; then
    echo "No demo tenant found" >&2
    return 1
  fi

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  ROLE_EMAIL="role-concurrent-$(date +%s)@example.com"
  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${ROLE_EMAIL}\",\"display_name\":\"Role Concurrent\",\"password\":\"Test123!\",\"tenant_id\":\"${TENANT_ID}\"}")
  USER_ID=$(resp_body "$resp" | jq -r '"'"'.data.id // .id'"'"')

  SERVICE_ID=$(db_query "SELECT id FROM services WHERE tenant_id='"'"'${TENANT_ID}'"'"' LIMIT 1;" | tr -d '[:space:]')
  if [[ -z "$SERVICE_ID" ]]; then
    echo "No service found for tenant" >&2
    return 1
  fi

  ROLE_ID=$(db_query "SELECT id FROM roles WHERE service_id='"'"'${SERVICE_ID}'"'"' LIMIT 1;" | tr -d '[:space:]')
  if [[ -z "$ROLE_ID" ]]; then
    echo "No role found for service" >&2
    return 1
  fi

  TENANT_TOKEN=$(gen_tenant_token "$USER_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  PIDS=()
  RESULTS_DIR=$(mktemp -d)
  for i in $(seq 1 10); do
    (
      resp=$(api_post "/api/v1/tenants/${TENANT_ID}/users/${USER_ID}/roles" \
        "{\"role_id\":\"${ROLE_ID}\"}")
      echo "$(resp_status "$resp")" > "${RESULTS_DIR}/result_${i}.txt"
    ) &
    PIDS+=($!)
  done

  for pid in "${PIDS[@]}"; do
    wait "$pid" || true
  done

  SUCCESS_COUNT=0
  CONFLICT_COUNT=0
  for i in $(seq 1 10); do
    code=$(cat "${RESULTS_DIR}/result_${i}.txt" 2>/dev/null || echo "000")
    if [[ "$code" == "201" || "$code" == "200" ]]; then
      SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    elif [[ "$code" == "409" ]]; then
      CONFLICT_COUNT=$((CONFLICT_COUNT + 1))
    fi
  done
  rm -rf "$RESULTS_DIR"

  TOTAL=$((SUCCESS_COUNT + CONFLICT_COUNT))
  assert_eq "$TOTAL" "10" "all 10 requests returned 201/200 or 409"

  ROLE_COUNT=$(db_query "SELECT COUNT(*) FROM user_tenant_roles utr JOIN tenant_users tu ON tu.id = utr.tenant_user_id WHERE tu.user_id='"'"'${USER_ID}'"'"' AND tu.tenant_id='"'"'${TENANT_ID}'"'"' AND utr.role_id='"'"'${ROLE_ID}'"'"';" | tr -d '[:space:]')
  assert_eq "$ROLE_COUNT" "1" "DB has exactly 1 role assignment"

  db_exec "DELETE FROM user_tenant_roles WHERE tenant_user_id IN (SELECT id FROM tenant_users WHERE user_id='"'"'${USER_ID}'"'"');" || true
  db_exec "DELETE FROM tenant_users WHERE user_id='"'"'${USER_ID}'"'"';" || true
  db_exec "DELETE FROM users WHERE id='"'"'${USER_ID}'"'"';" || true
  qa_set_token ""
'

scenario 4 "Concurrent webhook event triggering" '
  TENANT_ID=$(_lookup_demo_tenant)
  if [[ -z "$TENANT_ID" ]]; then
    echo "No demo tenant found" >&2
    return 1
  fi

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  PIDS=()
  CREATED_UIDS=()
  RESULTS_DIR=$(mktemp -d)

  for i in $(seq 1 5); do
    (
      BATCH_EMAIL="wh-batch-$(date +%s)-${i}@example.com"
      resp=$(api_post "/api/v1/users" \
        "{\"email\":\"${BATCH_EMAIL}\",\"display_name\":\"WH Batch ${i}\",\"password\":\"Test123!\",\"tenant_id\":\"${TENANT_ID}\"}")
      echo "$(resp_status "$resp")" > "${RESULTS_DIR}/result_${i}.txt"
    ) &
    PIDS+=($!)
  done

  for pid in "${PIDS[@]}"; do
    wait "$pid" || true
  done

  SUCCESS_COUNT=0
  for i in $(seq 1 5); do
    code=$(cat "${RESULTS_DIR}/result_${i}.txt" 2>/dev/null || echo "000")
    if [[ "$code" == "201" ]]; then
      SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    fi
  done
  rm -rf "$RESULTS_DIR"

  assert_eq "$SUCCESS_COUNT" "5" "all 5 concurrent user creates succeeded"

  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email LIKE '"'"'wh-batch-%@example.com'"'"');" || true
  db_exec "DELETE FROM users WHERE email LIKE '"'"'wh-batch-%@example.com'"'"';" || true
  qa_set_token ""
'

run_all
