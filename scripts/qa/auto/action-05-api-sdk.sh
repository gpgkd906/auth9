#!/usr/bin/env bash
# QA Auto Test: action/05-api-sdk
# Doc: docs/qa/action/05-api-sdk.md
# Scenarios: 5 (scenarios 3-4 are SDK/TypeScript tests, tested via REST equivalents)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "REST API - full CRUD lifecycle" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA API CRUD Test'\'';" || true

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA API CRUD Test\",\"description\":\"Created via REST API\",\"trigger_id\":\"post-login\",\"script\":\"context.claims = context.claims || {}; context.claims.api_test = true; context;\",\"enabled\":true,\"execution_order\":0,\"timeout_ms\":3000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "POST create action returns 201"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.name" "QA API CRUD Test" "action name matches"
  assert_json_field "$body" ".data.trigger_id" "post-login" "trigger_id matches"
  assert_json_exists "$body" ".data.id" "action id exists"
  ACTION_ID=$(echo "$body" | jq -r ".data.id")

  resp=$(api_get "/api/v1/services/${SVC_ID}/actions")
  assert_http_status "$(resp_status "$resp")" 200 "GET list actions returns 200"
  list_body=$(resp_body "$resp")
  found=$(echo "$list_body" | jq -r "[.data[] | select(.id == \"${ACTION_ID}\")] | length")
  assert_eq "$found" "1" "created action appears in list"

  resp=$(api_get "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}")
  assert_http_status "$(resp_status "$resp")" 200 "GET single action returns 200"
  assert_json_field "$(resp_body "$resp")" ".data.name" "QA API CRUD Test" "get returns correct action"

  resp=$(api_patch "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" \
    "{\"description\":\"Updated description\",\"enabled\":false}")
  assert_http_status "$(resp_status "$resp")" 200 "PATCH update action returns 200"
  assert_json_field "$(resp_body "$resp")" ".data.enabled" "false" "action disabled after update"

  resp=$(api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}")
  del_status=$(resp_status "$resp")
  assert_match "$del_status" "^(200|204)$" "DELETE action returns 200/204"

  resp=$(api_get "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}")
  assert_http_status "$(resp_status "$resp")" 404 "GET deleted action returns 404"

  qa_set_token ""
'

scenario 2 "REST API - trigger filter" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  CREATED_IDS=()
  for trigger in post-login pre-user-registration; do
    db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA Trigger Filter ${trigger}'\'';" || true
    resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
      "{\"name\":\"QA Trigger Filter ${trigger}\",\"trigger_id\":\"${trigger}\",\"script\":\"context;\"}")
    assert_match "$(resp_status "$resp")" "^(200|201)$" "create action for trigger ${trigger}"
    CREATED_IDS+=($(resp_body "$resp" | jq -r ".data.id"))
  done

  resp=$(api_get "/api/v1/services/${SVC_ID}/actions?trigger_id=post-login")
  assert_http_status "$(resp_status "$resp")" 200 "GET with trigger filter returns 200"
  body=$(resp_body "$resp")
  non_matching=$(echo "$body" | jq '"'"'[.data[] | select(.trigger_id != "post-login")] | length'"'"')
  assert_eq "$non_matching" "0" "all returned actions have trigger_id=post-login"

  for aid in "${CREATED_IDS[@]}"; do
    api_delete "/api/v1/services/${SVC_ID}/actions/${aid}" >/dev/null 2>&1 || true
  done
  qa_set_token ""
'

scenario 3 "REST API - CRUD (SDK equivalent via curl)" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA SDK CRUD Test'\'';" || true

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA SDK CRUD Test\",\"trigger_id\":\"post-login\",\"script\":\"context.claims = context.claims || {}; context.claims.sdk_test = true; context;\",\"enabled\":true}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "SDK-equiv create returns 201"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  resp=$(api_get "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}")
  assert_http_status "$(resp_status "$resp")" 200 "SDK-equiv get returns 200"
  assert_json_field "$(resp_body "$resp")" ".data.name" "QA SDK CRUD Test" "get matches name"

  resp=$(api_patch "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" \
    "{\"description\":\"Updated via SDK equivalent\"}")
  assert_http_status "$(resp_status "$resp")" 200 "SDK-equiv update returns 200"
  assert_json_field "$(resp_body "$resp")" ".data.description" "Updated via SDK equivalent" "description updated"

  resp=$(api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}")
  del_status=$(resp_status "$resp")
  assert_match "$del_status" "^(200|204)$" "SDK-equiv delete succeeds"

  qa_set_token ""
'

scenario 4 "REST API - batch upsert" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name IN ('\''QA Batch A'\'', '\''QA Batch B'\'');" || true

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions/batch" \
    "{\"actions\":[{\"name\":\"QA Batch A\",\"trigger_id\":\"post-login\",\"script\":\"context;\"},{\"name\":\"QA Batch B\",\"trigger_id\":\"post-login\",\"script\":\"context;\"}]}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|201)$" "batch upsert returns 200/201"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "batch response has data"

  resp=$(api_get "/api/v1/services/${SVC_ID}/actions")
  list_body=$(resp_body "$resp")
  countA=$(echo "$list_body" | jq '"'"'[.data[] | select(.name == "QA Batch A")] | length'"'"')
  countB=$(echo "$list_body" | jq '"'"'[.data[] | select(.name == "QA Batch B")] | length'"'"')
  assert_eq "$countA" "1" "batch action A exists"
  assert_eq "$countB" "1" "batch action B exists"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name IN ('\''QA Batch A'\'', '\''QA Batch B'\'');" || true
  qa_set_token ""
'

scenario 5 "REST API - test action endpoint" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA Test Endpoint'\'';" || true

  SCRIPT="if (context.user.email.endsWith(\\\"@blocked.com\\\")) { throw new Error(\\\"Blocked domain\\\"); } context.claims = context.claims || {}; context.claims.tested = true;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Test Endpoint\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create test action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  test_ctx="{\"context\":{\"user\":{\"id\":\"u1\",\"email\":\"user@allowed.com\",\"mfa_enabled\":false},\"tenant\":{\"id\":\"${SVC_ID}\",\"slug\":\"test\",\"name\":\"Test\"},\"request\":{\"timestamp\":\"2026-03-07T00:00:00Z\"}}}"
  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$test_ctx")
  assert_http_status "$(resp_status "$tresp")" 200 "test with allowed email returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "allowed email succeeds"
  assert_json_field "$tbody" ".data.modified_context.claims.tested" "true" "tested claim present"

  blocked_ctx="{\"context\":{\"user\":{\"id\":\"u1\",\"email\":\"user@blocked.com\",\"mfa_enabled\":false},\"tenant\":{\"id\":\"${SVC_ID}\",\"slug\":\"test\",\"name\":\"Test\"},\"request\":{\"timestamp\":\"2026-03-07T00:00:00Z\"}}}"
  tresp2=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$blocked_ctx")
  assert_http_status "$(resp_status "$tresp2")" 200 "test with blocked email returns 200"
  tbody2=$(resp_body "$tresp2")
  assert_json_field "$tbody2" ".data.success" "false" "blocked email fails"
  error_msg=$(echo "$tbody2" | jq -r ".data.error_message // .data.error // \"\"")
  assert_contains "$error_msg" "Blocked domain" "error mentions blocked domain"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
