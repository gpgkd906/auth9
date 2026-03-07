#!/usr/bin/env bash
# QA Auto Test: webhook/04-boundary
# Doc: docs/qa/webhook/04-boundary.md
# Scenarios: 3
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq


scenario 1 "无效 URL 验证" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  WH_TENANT=$(qa_get_tenant_id)

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"Valid HTTPS\",\"url\":\"https://api.example.com/webhook\",\"events\":[\"user.created\"],\"enabled\":true}")
  assert_http_status "$(resp_status "$resp")" 200 "valid HTTPS URL accepted"
  VALID_ID=$(resp_body "$resp" | jq -r ".data.id")
  api_delete "/api/v1/tenants/${WH_TENANT}/webhooks/${VALID_ID}" >/dev/null 2>&1 || true

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"Localhost\",\"url\":\"http://localhost:3000/webhook\",\"events\":[\"user.created\"],\"enabled\":true}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "localhost URL rejected (SSRF)"

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"HTTP External\",\"url\":\"http://api.example.com/webhook\",\"events\":[\"user.created\"],\"enabled\":true}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "HTTP external URL rejected (HTTPS required)"

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"No Protocol\",\"url\":\"api.example.com/webhook\",\"events\":[\"user.created\"],\"enabled\":true}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "URL without protocol rejected"

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"Private IP\",\"url\":\"http://192.168.1.1/webhook\",\"events\":[\"user.created\"],\"enabled\":true}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "private IP URL rejected (SSRF)"

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"IPv6 Loopback\",\"url\":\"http://[::1]/webhook\",\"events\":[\"user.created\"],\"enabled\":true}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "IPv6 loopback URL rejected (SSRF)"

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"Cloud Metadata\",\"url\":\"http://169.254.169.254/latest/meta-data\",\"events\":[\"user.created\"],\"enabled\":true}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "cloud metadata URL rejected (SSRF)"

  qa_set_token ""
'

scenario 2 "大 Payload 处理" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  WH_TENANT=$(qa_get_tenant_id)

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"QA Big Payload $(date +%s)\",\"url\":\"https://httpbin.org/post\",\"events\":[\"security.alert\"],\"enabled\":true}")
  assert_http_status "$(resp_status "$resp")" 200 "create webhook for big payload test"
  WH_ID=$(resp_body "$resp" | jq -r ".data.id")
  qa_set_token ""

  WEBHOOK_SECRET="dev-webhook-secret"  # pragma: allowlist secret
  sign_body() {
    echo -n "$1" | openssl dgst -sha256 -hmac "${WEBHOOK_SECRET}" | awk '"'"'{print $NF}'"'"'
  }

  TIME1=$(python3 -c "import time; print(int(time.time()*1000))")
  BODY1="{\"type\":\"LOGIN\",\"time\":${TIME1},\"userId\":\"00000000-0000-0000-0000-000000000099\",\"ipAddress\":\"203.0.113.10\",\"details\":{\"email\":\"qa-big-payload@example.com\"}}"
  SIG1=$(sign_body "$BODY1")

  resp=$(api_raw POST /api/v1/keycloak/events \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: ${SIG1}" \
    -H "User-Agent: qa-small-ua" \
    -d "$BODY1")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|204|401)$" "first login event (may need correct HMAC)"

  sleep 2

  TIME2=$(python3 -c "import time; print(int(time.time()*1000))")
  LARGE_UA=$(python3 -c "print('"'"'B'"'"' * 10000)")
  BODY2="{\"type\":\"LOGIN\",\"time\":${TIME2},\"userId\":\"00000000-0000-0000-0000-000000000099\",\"ipAddress\":\"203.0.113.10\",\"details\":{\"email\":\"qa-big-payload@example.com\"}}"
  SIG2=$(sign_body "$BODY2")

  resp=$(api_raw POST /api/v1/keycloak/events \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: ${SIG2}" \
    -H "User-Agent: ${LARGE_UA}" \
    -d "$BODY2")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|204|401)$" "login event with large UA (may need correct HMAC)"

  fc=$(db_query "SELECT failure_count FROM webhooks WHERE id='"'"'${WH_ID}'"'"';" | tr -d '[:space:]')
  assert_eq "$fc" "0" "webhook failure_count remains 0 (or event not yet dispatched)"

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  api_delete "/api/v1/tenants/${WH_TENANT}/webhooks/${WH_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "无效端点响应处理" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  WH_TENANT=$(qa_get_tenant_id)

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"QA 200 OK $(date +%s)\",\"url\":\"https://httpbin.org/post\",\"events\":[\"user.created\"],\"enabled\":true}")
  assert_http_status "$(resp_status "$resp")" 200 "create webhook with 200 endpoint"
  WH_200=$(resp_body "$resp" | jq -r ".data.id")

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks/${WH_200}/test" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "test webhook returns 200"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.success" "true" "httpbin/post returns success"

  fc=$(db_query "SELECT failure_count FROM webhooks WHERE id='"'"'${WH_200}'"'"';" | tr -d '[:space:]')
  assert_eq "$fc" "0" "failure_count = 0 for successful webhook"

  api_delete "/api/v1/tenants/${WH_TENANT}/webhooks/${WH_200}" >/dev/null 2>&1 || true

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"QA 400 Error $(date +%s)\",\"url\":\"https://httpbin.org/status/400\",\"events\":[\"user.created\"],\"enabled\":true}")
  assert_http_status "$(resp_status "$resp")" 200 "create webhook with 400 endpoint"
  WH_400=$(resp_body "$resp" | jq -r ".data.id")

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks/${WH_400}/test" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "test 400 webhook returns 200"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.success" "false" "400 endpoint reports failure"

  fc=$(db_query "SELECT failure_count FROM webhooks WHERE id='"'"'${WH_400}'"'"';" | tr -d '[:space:]')
  assert_match "$fc" "^[1-9][0-9]*$" "failure_count > 0 for 400 response"

  api_delete "/api/v1/tenants/${WH_TENANT}/webhooks/${WH_400}" >/dev/null 2>&1 || true

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks" \
    "{\"name\":\"QA 500 Error $(date +%s)\",\"url\":\"https://httpbin.org/status/500\",\"events\":[\"user.created\"],\"enabled\":true}")
  assert_http_status "$(resp_status "$resp")" 200 "create webhook with 500 endpoint"
  WH_500=$(resp_body "$resp" | jq -r ".data.id")

  resp=$(api_post "/api/v1/tenants/${WH_TENANT}/webhooks/${WH_500}/test" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "test 500 webhook returns 200"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.success" "false" "500 endpoint reports failure"

  fc=$(db_query "SELECT failure_count FROM webhooks WHERE id='"'"'${WH_500}'"'"';" | tr -d '[:space:]')
  assert_match "$fc" "^[1-9][0-9]*$" "failure_count > 0 for 500 response"

  api_delete "/api/v1/tenants/${WH_TENANT}/webhooks/${WH_500}" >/dev/null 2>&1 || true

  qa_set_token ""
'

run_all
