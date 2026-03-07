#!/usr/bin/env bash
# QA Auto Test: action/06-async-fetch
# Doc: docs/qa/action/06-async-fetch.md
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

scenario 1 "Basic async/await script execution" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name IN ('\''QA Sync Compat'\'', '\''QA Async Basic'\'');" || true

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Sync Compat\",\"trigger_id\":\"post-login\",\"script\":\"context.claims = context.claims || {}; context.claims.sync_test = true; context;\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create sync action"
  SYNC_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${SYNC_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "sync test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "sync action succeeds"
  assert_json_field "$tbody" ".data.modified_context.claims.sync_test" "true" "sync_test claim present"

  ASYNC_SCRIPT="async function enrich() {\n  const result = await Promise.resolve({ role: \"admin\" });\n  context.claims = context.claims || {};\n  context.claims.enriched_role = result.role;\n}\nawait enrich();"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Async Basic\",\"trigger_id\":\"post-login\",\"script\":\"${ASYNC_SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create async action"
  ASYNC_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ASYNC_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "async test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "async action succeeds"
  assert_json_field "$tbody" ".data.modified_context.claims.enriched_role" "admin" "enriched_role claim present"

  api_delete "/api/v1/services/${SVC_ID}/actions/${SYNC_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/services/${SVC_ID}/actions/${ASYNC_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "fetch() external API request" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA Fetch%'\'';" || true

  SCRIPT="const resp = await fetch(\"https://httpbin.org/get\", { method: \"GET\" });\ncontext.claims = context.claims || {};\ncontext.claims.fetch_status = resp.status;\ncontext.claims.fetch_ok = resp.ok;"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Fetch GET Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":15000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create fetch GET action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "fetch GET test returns 200"
  tbody=$(resp_body "$tresp")

  if [[ "$(echo "$tbody" | jq -r '"'"'.data.success'"'"')" == "true" ]]; then
    assert_json_field "$tbody" ".data.modified_context.claims.fetch_status" "200" "fetch returned status 200"
    assert_json_field "$tbody" ".data.modified_context.claims.fetch_ok" "true" "fetch_ok is true"
  else
    error_msg=$(echo "$tbody" | jq -r ".data.error_message // .data.error // \"\"")
    assert_contains "$error_msg" "not in allowlist\|not allowed\|blocked\|Domain" "fetch blocked by allowlist (httpbin.org not configured)"
  fi

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "Security - domain allowlist and private IP blocking" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA SSRF%'\'';" || true

  SCRIPT="const resp = await fetch(\"https://evil.example.com/data\");\ncontext.claims = { fetched: true };"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA SSRF Blocked Domain\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create blocked domain action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "blocked domain test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "false" "blocked domain fetch fails"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true

  SCRIPT="const resp = await fetch(\"http://127.0.0.1:8080/health\");\ncontext.claims = { ssrf: true };"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA SSRF Loopback\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create SSRF loopback action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "SSRF loopback test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "false" "loopback fetch blocked"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true

  SCRIPT="const resp = await fetch(\"http://169.254.169.254/metadata\");\ncontext.claims = { metadata: true };"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA SSRF Metadata\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create SSRF metadata action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "SSRF metadata test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "false" "link-local metadata fetch blocked"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 4 "setTimeout and console.log" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA Timeout%'\'';" || true
  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA Console%'\'';" || true

  SCRIPT="const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));\nawait delay(100);\ncontext.claims = context.claims || {};\ncontext.claims.delayed = true;"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Timeout Delay Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create setTimeout action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "setTimeout test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "setTimeout action succeeds"
  assert_json_field "$tbody" ".data.modified_context.claims.delayed" "true" "delayed claim present"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true

  SCRIPT="console.log(\"Action started for user:\", context.user.email);\nconsole.warn(\"This is a warning\");\nconsole.error(\"This is an error log\");\ncontext.claims = context.claims || {};\ncontext.claims.logged = true;"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Console Log Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create console.log action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "console.log test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "console.log action succeeds"
  assert_json_field "$tbody" ".data.modified_context.claims.logged" "true" "logged claim present"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 5 "Promise rejection and error handling" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA Promise%'\'';" || true
  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA Async Throw%'\'';" || true
  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA Action Timeout%'\'';" || true
  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name LIKE '\''QA Fetch Error%'\'';" || true

  SCRIPT="await Promise.reject(new Error(\"User not authorized\"));"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Promise Reject Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create promise reject action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "promise reject test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "false" "promise reject fails"
  error_msg=$(echo "$tbody" | jq -r ".data.error_message // .data.error // \"\"")
  assert_contains "$error_msg" "User not authorized" "error mentions User not authorized"
  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true

  SCRIPT="async function validate() {\n  if (!context.user.mfa_enabled) {\n    throw new Error(\"MFA required for this tenant\");\n  }\n}\nawait validate();"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Async Throw Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create async throw action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "async throw test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "false" "async throw fails"
  error_msg=$(echo "$tbody" | jq -r ".data.error_message // .data.error // \"\"")
  assert_contains "$error_msg" "MFA required" "error mentions MFA required"
  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true

  SCRIPT="const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));\nawait delay(10000);\ncontext.claims = { never_reached: true };"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Action Timeout Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":2000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create timeout action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "timeout test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "false" "timeout action fails"
  assert_json_not_exists "$tbody" ".data.modified_context.claims.never_reached" "never_reached claim absent"
  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true

  SCRIPT="try {\n  const resp = await fetch(\"https://nonexistent.invalid/api\");\n  context.claims = { fetched: true };\n} catch (e) {\n  context.claims = context.claims || {};\n  context.claims.fetch_error = e.message;\n  context.claims.graceful = true;\n}"
  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Fetch Error Handle Test\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":15000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create fetch error handle action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "fetch error handle test returns 200"
  tbody=$(resp_body "$tresp")

  success_val=$(echo "$tbody" | jq -r ".data.success")
  if [[ "$success_val" == "true" ]]; then
    assert_json_field "$tbody" ".data.modified_context.claims.graceful" "true" "fetch error handled gracefully"
  else
    error_msg=$(echo "$tbody" | jq -r ".data.error_message // .data.error // \"\"")
    assert_contains "$error_msg" "not in allowlist\|not allowed\|blocked\|Domain" "fetch blocked by allowlist before try/catch"
  fi

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
