#!/usr/bin/env bash
# QA Auto Test: invitation/01-create-send
# Doc: docs/qa/invitation/01-create-send.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

TENANT_ID="11111111-1111-4111-8111-111111111111"
ROLE_EDITOR_ID="44444444-4444-4444-8444-444444444444"
ROLE_VIEWER_ID="55555555-5555-4555-8555-555555555555"

scenario 1 "创建邀请" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  UNIQUE_EMAIL="qa-create-$(date +%s)@example.com"

  resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
    "{\"email\":\"${UNIQUE_EMAIL}\",\"role_ids\":[\"'"$ROLE_EDITOR_ID"'\",\"'"$ROLE_VIEWER_ID"'\"],\"expires_in_hours\":72}")
  assert_http_status "$(resp_status "$resp")" 201 "POST create invitation returns 201"

  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.status" "pending" "invitation status is pending"
  assert_json_field "$body" ".data.email" "$UNIQUE_EMAIL" "invitation email matches"
  assert_json_exists "$body" ".data.id" "invitation has id"
  assert_json_exists "$body" ".data.expires_at" "invitation has expires_at"

  INV_ID=$(echo "$body" | jq -r ".data.id")
  assert_db_not_empty "SELECT id FROM invitations WHERE id='"'"'${INV_ID}'"'"' AND status='"'"'pending'"'"';" "invitation exists in DB with pending status"

  qa_set_token ""
'

scenario 2 "邀请已存在的租户成员" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
    "{\"email\":\"admin@auth9.local\",\"role_ids\":[\"'"$ROLE_EDITOR_ID"'\"]}")
  assert_http_status "$(resp_status "$resp")" 409 "inviting existing member returns 409"

  body=$(resp_body "$resp")
  assert_contains "$body" "already a member" "error mentions already a member"

  qa_set_token ""
'

scenario 3 "重复邀请同一邮箱" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
    "{\"email\":\"pending@example.com\",\"role_ids\":[\"'"$ROLE_EDITOR_ID"'\"]}")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")

  if [[ "$status" == "409" ]] || [[ "$status" == "400" ]]; then
    assert_match "$status" "^(409|400)$" "duplicate invitation returns conflict/bad request"
    assert_contains "$body" "invitation" "error mentions invitation"
  else
    assert_http_status "$status" 201 "duplicate invitation updates and returns 201"
  fi

  count=$(db_query "SELECT COUNT(*) FROM invitations WHERE email='"'"'pending@example.com'"'"' AND tenant_id='"'"''"$TENANT_ID"''"'"' AND status='"'"'pending'"'"';")
  count=$(echo "$count" | tr -d '[:space:]')
  assert_match "$count" "^[01]$" "at most 1 pending invitation for same email"

  qa_set_token ""
'

scenario 4 "重新发送邀请" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  INV_ID="66666666-6666-4666-8666-666666666666"

  resp=$(api_post "/api/v1/tenants/{tenant_id}/invitations/${INV_ID}/resend" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "POST resend invitation returns 200"

  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.status" "pending" "invitation still pending after resend"
  assert_json_field "$body" ".data.id" "$INV_ID" "resent invitation id matches"

  qa_set_token ""
'

scenario 5 "不同过期时间选项" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  for hours in 24 48 72 168; do
    EMAIL="qa-expiry-${hours}-$(date +%s)@example.com"
    resp=$(api_post "/api/v1/tenants/'"$TENANT_ID"'/invitations" \
      "{\"email\":\"${EMAIL}\",\"role_ids\":[\"'"$ROLE_VIEWER_ID"'\"],\"expires_in_hours\":${hours}}")
    assert_http_status "$(resp_status "$resp")" 201 "create invitation with ${hours}h expiry returns 201"

    body=$(resp_body "$resp")
    assert_json_exists "$body" ".data.expires_at" "invitation with ${hours}h has expires_at"
    assert_json_field "$body" ".data.status" "pending" "invitation with ${hours}h is pending"
  done

  qa_set_token ""
'

run_all
