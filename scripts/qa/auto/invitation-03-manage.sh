#!/usr/bin/env bash
# QA Auto Test: invitation/03-manage
# Doc: docs/qa/invitation/03-manage.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

TENANT_ID="11111111-1111-4111-8111-111111111111"
ROLE_ADMIN_ID="33333333-3333-4333-8333-333333333333"
ROLE_EDITOR_ID="44444444-4444-4444-8444-444444444444"
ROLE_VIEWER_ID="55555555-5555-4555-8555-555555555555"

scenario 1 "撤销邀请" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  UNIQUE_EMAIL="qa-revoke-$(date +%s)@example.com"
  resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
    "{\"email\":\"${UNIQUE_EMAIL}\",\"role_ids\":[\"'"$ROLE_EDITOR_ID"'\"],\"expires_in_hours\":72}")
  assert_http_status "$(resp_status "$resp")" 201 "create invitation for revoke test"
  INV_ID=$(resp_body "$resp" | jq -r ".data.id")

  resp=$(api_post "/api/v1/tenants/{tenant_id}/invitations/${INV_ID}/revoke" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "POST revoke invitation returns 200"

  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.status" "revoked" "invitation status is revoked"

  assert_db "SELECT status FROM invitations WHERE id='"'"'${INV_ID}'"'"';" "revoked" "DB status is revoked"

  qa_set_token ""
'

scenario 2 "删除邀请" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  UNIQUE_EMAIL="qa-delete-$(date +%s)@example.com"
  resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
    "{\"email\":\"${UNIQUE_EMAIL}\",\"role_ids\":[\"'"$ROLE_VIEWER_ID"'\"],\"expires_in_hours\":72}")
  assert_http_status "$(resp_status "$resp")" 201 "create invitation for delete test"
  INV_ID=$(resp_body "$resp" | jq -r ".data.id")

  resp=$(api_delete "/api/v1/tenants/{tenant_id}/invitations/${INV_ID}")
  assert_http_status "$(resp_status "$resp")" 200 "DELETE invitation returns 200"

  assert_db "SELECT COUNT(*) FROM invitations WHERE id='"'"'${INV_ID}'"'"';" "0" "invitation deleted from DB"

  qa_set_token ""
'

scenario 3 "邀请列表过滤" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_get "/api/v1/tenants/'"$TENANT_ID"'/invitations")
  assert_http_status "$(resp_status "$resp")" 200 "GET invitation list returns 200"
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "invitation list has data"

  resp=$(api_get "/api/v1/tenants/'"$TENANT_ID"'/invitations?status=pending")
  assert_http_status "$(resp_status "$resp")" 200 "GET pending invitations returns 200"
  body=$(resp_body "$resp")
  pending_statuses=$(echo "$body" | jq -r '"'"'[.data[] | .status] | unique | .[]'"'"' 2>/dev/null || echo "")
  if [[ -n "$pending_statuses" ]]; then
    assert_eq "$pending_statuses" "pending" "pending filter only returns pending status"
  fi

  resp=$(api_get "/api/v1/tenants/'"$TENANT_ID"'/invitations?status=accepted")
  assert_http_status "$(resp_status "$resp")" 200 "GET accepted invitations returns 200"

  resp=$(api_get "/api/v1/tenants/'"$TENANT_ID"'/invitations?status=expired")
  assert_http_status "$(resp_status "$resp")" 200 "GET expired invitations returns 200"

  resp=$(api_get "/api/v1/tenants/'"$TENANT_ID"'/invitations?status=revoked")
  assert_http_status "$(resp_status "$resp")" 200 "GET revoked invitations returns 200"

  qa_set_token ""
'

scenario 4 "邀请包含多个角色" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  MULTI_EMAIL="qa-multirole-$(date +%s)@example.com"
  resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
    "{\"email\":\"${MULTI_EMAIL}\",\"role_ids\":[\"'"$ROLE_ADMIN_ID"'\",\"'"$ROLE_EDITOR_ID"'\",\"'"$ROLE_VIEWER_ID"'\"],\"expires_in_hours\":72}")
  assert_http_status "$(resp_status "$resp")" 201 "create multi-role invitation returns 201"

  body=$(resp_body "$resp")
  role_count=$(echo "$body" | jq ".data.role_ids | length")
  assert_eq "$role_count" "3" "invitation contains 3 role IDs"

  qa_set_token ""
'

scenario 5 "邀请邮箱格式验证" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
    "{\"email\":\"invalid-email\",\"role_ids\":[\"'"$ROLE_VIEWER_ID"'\"]}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "invalid email rejected"

  resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
    "{\"email\":\"user@\",\"role_ids\":[\"'"$ROLE_VIEWER_ID"'\"]}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "incomplete email rejected"

  VALID_EMAIL="qa-valid-$(date +%s)@example.com"
  resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
    "{\"email\":\"${VALID_EMAIL}\",\"role_ids\":[\"'"$ROLE_VIEWER_ID"'\"]}")
  assert_http_status "$(resp_status "$resp")" 201 "valid email accepted"

  qa_set_token ""
'

run_all
