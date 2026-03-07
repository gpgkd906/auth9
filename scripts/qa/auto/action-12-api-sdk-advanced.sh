#!/usr/bin/env bash
# QA Auto Test: action/12-api-sdk-advanced
# Doc: docs/qa/action/12-api-sdk-advanced.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_ACTION_TEST_CTX=""
_setup_ctx() {
  local svc_id="$1"
  _ACTION_TEST_CTX="{\"context\":{\"user\":{\"id\":\"u1\",\"email\":\"test@example.com\",\"mfa_enabled\":false},\"tenant\":{\"id\":\"${svc_id}\",\"slug\":\"test\",\"name\":\"Test\"},\"request\":{\"timestamp\":\"2026-03-07T00:00:00Z\"}}}"
}

scenario 1 "Action execution logs query" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA Log Query'\'';" || true

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Log Query\",\"trigger_id\":\"post-login\",\"script\":\"context.claims = context.claims || {}; context.claims.log_test = true; context;\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create log query action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "test action for log"
  assert_json_field "$(resp_body "$tresp")" ".data.success" "true" "test execution succeeds"

  log_resp=$(api_get "/api/v1/services/${SVC_ID}/actions/logs?action_id=${ACTION_ID}&limit=10")
  assert_http_status "$(resp_status "$log_resp")" 200 "GET logs returns 200"
  log_body=$(resp_body "$log_resp")
  assert_json_exists "$log_body" ".data" "logs data exists"

  log_count=$(echo "$log_body" | jq ".data | length")
  assert_ne "$log_count" "0" "at least one log entry"

  if [[ "$log_count" -gt 0 ]]; then
    assert_json_exists "$log_body" ".data[0].success" "log entry has success field"
    assert_json_exists "$log_body" ".data[0].duration_ms" "log entry has duration_ms field"
  fi

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "Action execution stats query" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA Stats Query'\'';" || true

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Stats Query\",\"trigger_id\":\"post-login\",\"script\":\"context;\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create stats action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  for i in 1 2 3; do
    api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX" >/dev/null 2>&1
  done

  stats_resp=$(api_get "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/stats")
  assert_http_status "$(resp_status "$stats_resp")" 200 "GET stats returns 200"
  stats_body=$(resp_body "$stats_resp")
  assert_json_exists "$stats_body" ".data" "stats data exists"
  assert_json_exists "$stats_body" ".data.execution_count" "execution_count field exists"
  assert_json_exists "$stats_body" ".data.success_rate" "success_rate field exists"

  exec_count=$(echo "$stats_body" | jq -r ".data.execution_count")
  assert_match "$exec_count" "^[0-9]" "execution_count is numeric"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "Error handling - auth, 404, validation" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  qa_set_token "invalid-token-12345"
  resp=$(api_get "/api/v1/services/${SVC_ID}/actions")
  assert_http_status "$(resp_status "$resp")" 401 "invalid token returns 401"

  qa_set_token "$TOKEN"
  resp=$(api_get "/api/v1/services/${SVC_ID}/actions/00000000-0000-0000-0000-000000000000")
  assert_http_status "$(resp_status "$resp")" 404 "non-existent action returns 404"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"\",\"trigger_id\":\"\",\"script\":\"\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "empty fields return validation error"

  qa_set_token ""
'

scenario 4 "Concurrent action creation" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA Concurrent %'\'';" || true

  PIDS=()
  TMPDIR_CONC=$(mktemp -d)
  for i in $(seq 1 5); do
    (
      resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
        "{\"name\":\"QA Concurrent ${i}\",\"trigger_id\":\"post-login\",\"script\":\"context;\",\"enabled\":true}")
      echo "$(resp_status "$resp")" > "${TMPDIR_CONC}/status_${i}"
      resp_body "$resp" | jq -r ".data.id // \"\"" > "${TMPDIR_CONC}/id_${i}"
    ) &
    PIDS+=($!)
  done

  for pid in "${PIDS[@]}"; do
    wait "$pid" || true
  done

  success_count=0
  CREATED_IDS=()
  for i in $(seq 1 5); do
    status=$(cat "${TMPDIR_CONC}/status_${i}" 2>/dev/null || echo "0")
    if [[ "$status" == "201" ]]; then
      success_count=$((success_count + 1))
      aid=$(cat "${TMPDIR_CONC}/id_${i}" 2>/dev/null || echo "")
      if [[ -n "$aid" ]]; then
        CREATED_IDS+=("$aid")
      fi
    fi
  done
  rm -rf "$TMPDIR_CONC"

  assert_eq "$success_count" "5" "all 5 concurrent creates succeeded"

  for aid in "${CREATED_IDS[@]}"; do
    api_delete "/api/v1/services/${SVC_ID}/actions/${aid}" >/dev/null 2>&1 || true
  done
  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA Concurrent %'\'';" || true
  qa_set_token ""
'

scenario 5 "AI Agent integration - idempotent rule management" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  RULE_NAME="qa-service-x-access-control"
  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''${RULE_NAME}'\'';" || true

  SCRIPT="const allowedRoles = [\"admin\", \"developer\"];\nconst userRoles = (context.claims && context.claims.roles) || [];\nconst hasAccess = allowedRoles.some(role => userRoles.includes(role));\nif (!hasAccess) { throw new Error(\"Insufficient permissions\"); }\ncontext.claims = context.claims || {};\ncontext.claims.service_access = \"service-x\";"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"${RULE_NAME}\",\"description\":\"Auto-generated by AI Agent\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "AI agent creates rule"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  test_ctx_with_roles="{\"context\":{\"user\":{\"id\":\"u1\",\"email\":\"admin@company.com\",\"mfa_enabled\":false},\"tenant\":{\"id\":\"${SVC_ID}\",\"slug\":\"company\",\"name\":\"Company\"},\"request\":{\"timestamp\":\"2026-03-07T00:00:00Z\"},\"claims\":{\"roles\":[\"developer\"]}}}"
  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$test_ctx_with_roles")
  assert_http_status "$(resp_status "$tresp")" 200 "rule test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "developer role has access"

  resp=$(api_patch "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" \
    "{\"description\":\"Updated by AI Agent v2\"}")
  assert_http_status "$(resp_status "$resp")" 200 "AI agent updates rule"
  assert_json_field "$(resp_body "$resp")" ".data.description" "Updated by AI Agent v2" "description updated"

  resp=$(api_get "/api/v1/services/${SVC_ID}/actions")
  list_body=$(resp_body "$resp")
  rule_count=$(echo "$list_body" | jq "[.data[] | select(.name == \"${RULE_NAME}\")] | length")
  assert_eq "$rule_count" "1" "exactly one rule exists (no duplicates)"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
