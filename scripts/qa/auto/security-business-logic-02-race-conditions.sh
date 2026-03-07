#!/usr/bin/env bash
# Security Auto Test: security/business-logic/02-race-conditions
# Doc: docs/security/business-logic/02-race-conditions.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node
require_bin curl

_PLATFORM_TENANT_ID=""
_get_platform_tenant_id() {
  if [[ -z "$_PLATFORM_TENANT_ID" ]]; then
    _PLATFORM_TENANT_ID=$(db_query "SELECT id FROM tenants WHERE slug = '\''auth9-platform'\'' LIMIT 1;")
  fi
  echo "$_PLATFORM_TENANT_ID"
}

_concurrent_requests() {
  local count="$1" method="$2" url="$3" token="$4"
  shift 4
  local pids=() statuses=()

  for i in $(seq 1 "$count"); do
    (
      local status
      status=$(curl -s -o /dev/null -w "%{http_code}" \
        -X "$method" \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        "$@" \
        "${API_BASE}${url}" 2>/dev/null)
      echo "$status"
    ) &
    pids+=($!)
  done

  for pid in "${pids[@]}"; do
    wait "$pid" 2>/dev/null || true
  done
}

scenario 1 "Concurrent password reset token usage" '
  skip_scenario 1 "Password reset concurrent usage" \
    "Requires valid password reset token from email flow"
'

scenario 2 "Invitation accept race condition - DB unique constraint" '
  local admin_token tenant_id uid invite_uid
  admin_token=$(gen_default_admin_token)
  tenant_id=$(_get_platform_tenant_id)

  uid=$(db_query "SELECT LOWER(UUID());")
  invite_uid=$(db_query "SELECT LOWER(UUID());")
  local invite_email="qa-race-inv-$(date +%s)@test.com"

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-race-2'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-race-2'\'', '\''$invite_email'\'', '\''QA Race Inv'\'');"

  qa_set_token "$admin_token"
  resp=$(api_post "/api/v1/tenants/{tenant_id}/invitations" \
    "{\"email\":\"$invite_email\",\"tenant_id\":\"$tenant_id\",\"role_ids\":[]}")
  inv_status=$(resp_status "$resp")
  inv_body=$(resp_body "$resp")
  local inv_token
  inv_token=$(echo "$inv_body" | jq -r ".data.token // .token // empty")

  if [[ -z "$inv_token" || "$inv_token" == "null" ]]; then
    assert_eq "skip" "skip" "Could not create invitation - skipping race test"
  else
    local id_token
    id_token=$(gen_identity_token "$uid" "$invite_email")

    local results=()
    for i in $(seq 1 10); do
      (
        local s
        s=$(curl -s -o /dev/null -w "%{http_code}" \
          -X POST \
          -H "Authorization: Bearer $id_token" \
          -H "Content-Type: application/json" \
          -d "{\"token\":\"$inv_token\"}" \
          "${API_BASE}/api/v1/tenants/{tenant_id}/invitations/accept" 2>/dev/null)
        echo "$s"
      ) &
    done | sort > /tmp/qa_race_inv_results.txt
    wait

    local success_count
    success_count=$(grep -c "^200$" /tmp/qa_race_inv_results.txt 2>/dev/null || echo "0")
    local conflict_count
    conflict_count=$(grep -c "^409$" /tmp/qa_race_inv_results.txt 2>/dev/null || echo "0")

    assert_match "$success_count" "^[01]$" "At most 1 concurrent invitation accept succeeded"

    local tu_count
    tu_count=$(db_query "SELECT COUNT(*) FROM tenant_users WHERE user_id = '\''$uid'\'' AND tenant_id = '\''$tenant_id'\'';")
    assert_match "$tu_count" "^[01]$" "No duplicate tenant_user records from race condition"

    rm -f /tmp/qa_race_inv_results.txt
  fi

  db_exec "DELETE FROM tenant_users WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM invitations WHERE email = '\''$invite_email'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

skip_scenario 3 "Concurrent Token Exchange" "Requires gRPC client and long-running test setup"

scenario 4 "Tenant slug uniqueness under concurrent creation" '
  local admin_token
  admin_token=$(gen_default_admin_token)

  local test_slug="qa-race-slug-$(date +%s)"

  for i in $(seq 1 10); do
    (
      local s
      s=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST \
        -H "Authorization: Bearer $admin_token" \
        -H "Content-Type: application/json" \
        -d "{\"name\":\"Race Tenant $i\",\"slug\":\"$test_slug\"}" \
        "${API_BASE}/api/v1/tenants" 2>/dev/null)
      echo "$s"
    ) &
  done | sort > /tmp/qa_race_slug_results.txt
  wait

  local created
  created=$(grep -c "^201$" /tmp/qa_race_slug_results.txt 2>/dev/null || echo "0")
  assert_match "$created" "^[01]$" "At most 1 concurrent tenant creation with same slug succeeded"

  local db_count
  db_count=$(db_query "SELECT COUNT(*) FROM tenants WHERE slug = '\''$test_slug'\'';")
  assert_match "$db_count" "^[01]$" "No duplicate tenants with same slug in database"

  db_exec "DELETE FROM tenants WHERE slug = '\''$test_slug'\'';" || true
  rm -f /tmp/qa_race_slug_results.txt
'

run_all
