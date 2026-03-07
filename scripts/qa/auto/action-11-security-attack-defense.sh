#!/usr/bin/env bash
# QA Auto Test: action/11-security-attack-defense
# Doc: docs/qa/action/11-security-attack-defense.md
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

scenario 1 "Command injection prevention" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA Cmd Inject'\'';" || true

  SCRIPT="try {\n  const result = exec(\"rm -rf /\");\n  context.claims = context.claims || {};\n  context.claims.result = result;\n} catch (e) {\n  context.claims = context.claims || {};\n  context.claims.blocked = true;\n}\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Cmd Inject\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create command inject action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "command inject test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "action succeeded (try-catch handled)"
  assert_json_field "$tbody" ".data.modified_context.claims.blocked" "true" "exec() blocked"
  assert_json_not_exists "$tbody" ".data.modified_context.claims.result" "no exec result"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "Privilege escalation via claims" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name = '\''QA Priv Escalation'\'';" || true

  SCRIPT="context.claims = context.claims || {};\ncontext.claims.roles = [\"admin\", \"superuser\"];\ncontext.claims.permissions = [\"*\"];\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Priv Escalation\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"timeout_ms\":5000}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create privilege escalation action"
  ACTION_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "privilege escalation test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "escalation script executes (claims writable)"
  assert_json_exists "$tbody" ".data.modified_context.claims.roles" "roles claim injected"

  api_delete "/api/v1/services/${SVC_ID}/actions/${ACTION_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "Token forgery attack" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_get "/api/v1/tenants")
  assert_http_status "$(resp_status "$resp")" 200 "valid token accesses tenants"

  FORGED_TOKEN="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiJ0ZXN0IiwiYWRtaW4iOnRydWUsImlhdCI6MTcwOTI4MDAwMH0.FAKE_SIGNATURE"  # pragma: allowlist secret
  qa_set_token "$FORGED_TOKEN"
  resp=$(api_get "/api/v1/tenants")
  assert_http_status "$(resp_status "$resp")" 401 "forged token returns 401"

  qa_set_token ""
  resp=$(api_raw GET /api/v1/tenants)
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403)$" "no token returns 401/403"

  qa_set_token ""
'

scenario 4 "Action script injection - isolate execution" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  _setup_ctx "$SVC_ID"

  db_exec "DELETE FROM actions WHERE service_id = '\''${SVC_ID}'\'' AND name IN ('\''QA Malicious Global'\'', '\''QA Verify Isolation'\'');" || true

  SCRIPT="globalThis.maliciousFunction = function() { return \"hacked\"; };\nObject.prototype.hacked = true;\ncontext.claims = context.claims || {};\ncontext.claims.injected = true;\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Malicious Global\",\"trigger_id\":\"post-login\",\"script\":\"${SCRIPT}\",\"enabled\":true,\"execution_order\":0}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create malicious global action"
  MAL_ID=$(resp_body "$resp" | jq -r ".data.id")

  VERIFY_SCRIPT="context.claims = context.claims || {};\ncontext.claims.is_hacked = typeof globalThis.maliciousFunction !== \"undefined\";\ncontext.claims.prototype_hacked = Object.prototype.hasOwnProperty(\"hacked\");\ncontext;"

  resp=$(api_post "/api/v1/services/${SVC_ID}/actions" \
    "{\"name\":\"QA Verify Isolation\",\"trigger_id\":\"post-login\",\"script\":\"${VERIFY_SCRIPT}\",\"enabled\":true,\"execution_order\":10}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create verification action"
  VER_ID=$(resp_body "$resp" | jq -r ".data.id")

  tresp=$(api_post "/api/v1/services/${SVC_ID}/actions/${VER_ID}/test" "$_ACTION_TEST_CTX")
  assert_http_status "$(resp_status "$tresp")" 200 "isolation verify test returns 200"
  tbody=$(resp_body "$tresp")
  assert_json_field "$tbody" ".data.success" "true" "verification action succeeds"
  assert_json_field "$tbody" ".data.modified_context.claims.is_hacked" "false" "globalThis not polluted"
  assert_json_field "$tbody" ".data.modified_context.claims.prototype_hacked" "false" "Object.prototype not polluted"

  api_delete "/api/v1/services/${SVC_ID}/actions/${MAL_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/services/${SVC_ID}/actions/${VER_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
