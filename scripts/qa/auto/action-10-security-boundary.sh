#!/usr/bin/env bash
# QA Auto Test: action/10-security-boundary
# Doc: docs/qa/action/10-security-boundary.md
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

scenario 1 "Resource exhaustion - large memory allocation" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA OOM Test'\'';" || true

  SCRIPT="const arr = []; for (let i = 0; i < 100000000; i++) { arr.push(new Array(1000).fill(\"x\")); } context.claims = context.claims || {}; context.claims.allocated = arr.length; context;"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA OOM Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create OOM action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "OOM test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "false" "OOM action fails"
  error_msg=$(echo "$tbody" | jq -r ".data.error_message // .data.error // \"\"")
  assert_match "$error_msg" "[Mm]emory\|heap\|[Oo]ut.of\|allocation\|[Tt]imeout" "error mentions memory or heap"

  health_resp=$(api_raw GET /health)
  assert_http_status "$(resp_status "$health_resp")" 200 "auth9-core still healthy after OOM"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "Service isolation - cross-service context tampering" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_IDS=($(db_query "SELECT id FROM services LIMIT 2;" | tr -d " "))
  if [[ ${#SVC_IDS[@]} -lt 2 ]]; then
    echo "Need at least 2 services for isolation test" >&2
    return 1
  fi
  SVC_A="${SVC_IDS[0]}"
  SVC_B="${SVC_IDS[1]}"
  _setup_ctx "$SVC_A"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_A}'\'' AND name = '\''QA XService Tamper'\'';" || true

  SCRIPT="context.claims = context.claims || {};\ncontext.claims.attacked_user = \"service-b-user\";\ncontext.tenant.id = \"${SVC_B}\";\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_A}/actions" \
    "{\"name\":\"QA XService Tamper\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create cross-service tamper action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_A}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "cross-service tamper test returns 200"
  tbody=$(resp_body "$tresp")

  assert_json_field "$tbody" ".data.success" "true" "tamper script executes (claims are writable)"
  assert_json_exists "$tbody" ".data.modified_context.claims.attacked_user" "attacked_user claim set"

  resp_list=$(api_get "/api/v1/services/${SVC_A}/actions")
  assert_http_status "$(resp_status "$resp_list")" 200 "service A actions still accessible"
  found=$(resp_body "$resp_list" | jq "[.data[] | select(.id == \"${ACTION_ID}\")] | length")
  assert_eq "$found" "1" "action still belongs to service A"

  api_delete "/api/v1/services/${SVC_A}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "SQL injection prevention - claims with SQL payloads" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA SQL Inject'\'';" || true

  SCRIPT="context.claims = context.claims || {};\ncontext.claims.email = \"'\''; DROP TABLE users; --\";\ncontext.claims.search = \"admin'\'' OR '\''1'\''='\''1\";\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA SQL Inject\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create SQL inject action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "SQL inject test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "SQL payload in claims accepted (JSON strings)"

  users_exist=$(db_query "SELECT COUNT(*) FROM users;" | tr -d "[:space:]")
  assert_ne "$users_exist" "0" "users table still exists (SQL injection did not execute)"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 4 "XSS prevention - claims with script payloads" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA XSS Test'\'';" || true

  SCRIPT="context.claims = context.claims || {};\ncontext.claims.display_name = \"<script>alert(1)</script>\";\ncontext.claims.bio = \"<img src=x onerror=alert(1)>\";\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA XSS Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create XSS action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "XSS test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "XSS payload stored in claims (JSON strings)"
  assert_json_exists "$tbody" ".data.modified_context.claims.display_name" "display_name claim set"
  assert_json_exists "$tbody" ".data.modified_context.claims.bio" "bio claim set"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
