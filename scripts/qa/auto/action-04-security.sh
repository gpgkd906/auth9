#!/usr/bin/env bash
# QA Auto Test: action/04-security
# Doc: docs/qa/action/04-security.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "V8 sandbox isolation - filesystem access blocked" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  SCRIPT="try {\n  const content = Deno.readTextFile(\"/etc/passwd\");\n  context.claims = context.claims || {};\n  context.claims.leaked_data = content;\n} catch (e) {\n  context.claims = context.claims || {};\n  context.claims.blocked = true;\n  context.claims.error = String(e);\n}\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Sandbox FS Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create sandbox fs action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  test_ctx="{\"context\":{\"user\":{\"id\":\"u1\",\"email\":\"test@example.com\",\"mfa_enabled\":false},\"tenant\":{\"id\":\"${SVC_ID}\",\"slug\":\"test\",\"name\":\"Test\"},\"request\":{\"timestamp\":\"2026-03-07T00:00:00Z\"}}}"
  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$test_ctx")
  assert_http_status "$(resp_status "$tresp")" 200 "test action returns 200"

  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "action execution succeeded (try-catch handled)"
  assert_json_not_exists "$tbody" ".data.modified_context.claims.leaked_data" "no leaked_data in claims"
  assert_json_field "$tbody" ".data.modified_context.claims.blocked" "true" "blocked claim is true"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "V8 sandbox isolation - Node.js require blocked" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  SCRIPT="try {\n  const fs = require(\"fs\");\n  const content = fs.readFileSync(\"/etc/passwd\", \"utf-8\");\n  context.claims = context.claims || {};\n  context.claims.leaked_data = content;\n} catch (e) {\n  context.claims = context.claims || {};\n  context.claims.blocked = true;\n}\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Sandbox Require Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create require test action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  test_ctx="{\"context\":{\"user\":{\"id\":\"u1\",\"email\":\"test@example.com\",\"mfa_enabled\":false},\"tenant\":{\"id\":\"${SVC_ID}\",\"slug\":\"test\",\"name\":\"Test\"},\"request\":{\"timestamp\":\"2026-03-07T00:00:00Z\"}}}"
  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$test_ctx")
  assert_http_status "$(resp_status "$tresp")" 200 "test action returns 200"

  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "action execution succeeded (try-catch handled)"
  assert_json_not_exists "$tbody" ".data.modified_context.claims.leaked_data" "no leaked_data via require"
  assert_json_field "$tbody" ".data.modified_context.claims.blocked" "true" "blocked claim is true"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "V8 sandbox isolation - process object blocked" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  SCRIPT="try {\n  context.claims = context.claims || {};\n  context.claims.env = process.env;\n  context.claims.jwt_secret = process.env.JWT_SECRET;\n} catch (e) {\n  context.claims = context.claims || {};\n  context.claims.blocked = true;\n}\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Sandbox Process Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create process test action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  test_ctx="{\"context\":{\"user\":{\"id\":\"u1\",\"email\":\"test@example.com\",\"mfa_enabled\":false},\"tenant\":{\"id\":\"${SVC_ID}\",\"slug\":\"test\",\"name\":\"Test\"},\"request\":{\"timestamp\":\"2026-03-07T00:00:00Z\"}}}"
  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$test_ctx")
  assert_http_status "$(resp_status "$tresp")" 200 "test action returns 200"

  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "action execution succeeded (try-catch handled)"
  assert_json_not_exists "$tbody" ".data.modified_context.claims.jwt_secret" "no JWT_SECRET leaked"
  assert_json_field "$tbody" ".data.modified_context.claims.blocked" "true" "blocked claim is true"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 4 "Resource exhaustion - infinite loop timeout" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  SCRIPT="while (true) { const x = 1 + 1; }\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Infinite Loop Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":1000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create infinite loop action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  test_ctx="{\"context\":{\"user\":{\"id\":\"u1\",\"email\":\"test@example.com\",\"mfa_enabled\":false},\"tenant\":{\"id\":\"${SVC_ID}\",\"slug\":\"test\",\"name\":\"Test\"},\"request\":{\"timestamp\":\"2026-03-07T00:00:00Z\"}}}"
  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$test_ctx")
  assert_http_status "$(resp_status "$tresp")" 200 "test action returns 200"

  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "false" "action execution failed (timeout)"
  error_msg=$(echo "$tbody" | jq -r ".data.error_message // .data.error // \"\"")
  assert_match "$error_msg" "[Tt]imeout\|exceeded\|timed.out" "error mentions timeout"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
