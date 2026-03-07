#!/usr/bin/env bash
# QA Auto Test: action/08-execution-advanced
# Doc: docs/qa/action/08-execution-advanced.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_ACTION_TEST_CTX=""
_setup_ctx() {
  local svc_id="$1"
  _ACTION_TEST_CTX="{\"context\":{\"user\":{\"id\":\"u1\",\"email\":\"test@example.com\",\"mfa_enabled\":false},\"tenant\":{\"id\":\"${svc_id}\",\"slug\":\"test\",\"name\":\"Test\"},\"request\":{\"timestamp\":\"2026-03-07T00:00:00Z\"}}}"
}

scenario 1 "Action timeout control" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA Timeout Control'\'';" || true

  SCRIPT="const start = Date.now(); while (Date.now() - start < 2000) {} context;"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Timeout Control\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":1000,\"strict_mode\":true}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create timeout control action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "timeout control test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "false" "timeout action fails"
  error_msg=$(echo "$tbody" | jq -r ".data.error_message // .data.error // \"\"")
  assert_match "$error_msg" "[Tt]imeout\|exceeded\|timed.out" "error mentions timeout"

  duration=$(echo "$tbody" | jq -r ".data.duration_ms // 0")
  if [[ "$duration" -gt 0 ]]; then
    assert_match "$duration" "^[0-9]" "duration_ms is numeric"
  fi

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "Disabled action not executed via test endpoint" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA Disabled Action'\'';" || true

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Disabled Action\",\"trigger_id\":\"post-login\",\"script\":\"context.claims = context.claims || {}; context.claims.should_not_appear = true; context;\",\"enabled\":false}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create disabled action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  get_resp=$(api_get "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}")
  assert_json_field "$(resp_body "$get_resp")" ".data.enabled" "false" "action is disabled"

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "test endpoint still works for disabled action"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "test endpoint ignores enabled flag (by design)"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "Action context information validation" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA Context Validation'\'';" || true

  SCRIPT="if (!context.user || !context.tenant || !context.request) { throw new Error(\"Context incomplete\"); }\nif (!context.user.email || !context.user.id) { throw new Error(\"User info missing\"); }\ncontext.claims = context.claims || {};\ncontext.claims.context_validated = true;\ncontext.claims.user_email = context.user.email;\ncontext.claims.tenant_slug = context.tenant.slug;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Context Validation\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create context validation action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "context validation test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "context validation succeeds"
  assert_json_field "$tbody" ".data.modified_context.claims.context_validated" "true" "context_validated claim present"
  assert_json_field "$tbody" ".data.modified_context.claims.user_email" "test@example.com" "user email propagated"
  assert_json_field "$tbody" ".data.modified_context.claims.tenant_slug" "test" "tenant slug propagated"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 4 "Service isolation - actions scoped to service" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_IDS=($(db_query "SELECT id FROM services LIMIT 2;" | tr -d " "))
  if [[ ${#SVC_IDS[@]} -lt 2 ]]; then
    echo "Need at least 2 services for isolation test" >&2
    return 1
  fi
  SVC_A="${SVC_IDS[0]}"
  SVC_B="${SVC_IDS[1]}"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_A}'\'' AND name = '\''QA Isolation A'\'';" || true
  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_B}'\'' AND name = '\''QA Isolation B'\'';" || true

  resp_a=$(api_post "/api/v1/services/${SVC_A}/actions" \
    "{\"name\":\"QA Isolation A\",\"trigger_id\":\"post-login\",\"script\":\"context.claims = context.claims || {}; context.claims.from_a = true; context;\",\"enabled\":true}")
  assert_http_status "$(resp_status "$resp_a")" 201 "create action on service A"
  AID_A=$(resp_body "$resp_a" | jq -r ".data.id")

  resp_b=$(api_post "/api/v1/services/${SVC_B}/actions" \
    "{\"name\":\"QA Isolation B\",\"trigger_id\":\"post-login\",\"script\":\"context.claims = context.claims || {}; context.claims.from_b = true; context;\",\"enabled\":true}")
  assert_http_status "$(resp_status "$resp_b")" 201 "create action on service B"
  AID_B=$(resp_body "$resp_b" | jq -r ".data.id")

  resp=$(api_get "/api/v1/services/${SVC_A}/actions")
  list_a=$(resp_body "$resp")
  found_b_in_a=$(echo "$list_a" | jq "[.data[] | select(.id == \"${AID_B}\")] | length")
  assert_eq "$found_b_in_a" "0" "service B action not visible in service A list"

  resp=$(api_get "/api/v1/services/${SVC_B}/actions")
  list_b=$(resp_body "$resp")
  found_a_in_b=$(echo "$list_b" | jq "[.data[] | select(.id == \"${AID_A}\")] | length")
  assert_eq "$found_a_in_b" "0" "service A action not visible in service B list"

  api_delete "/api/v1/services/${SVC_A}/actions/${AID_A}" >/dev/null 2>&1 || true
  api_delete "/api/v1/services/${SVC_B}/actions/${AID_B}" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
