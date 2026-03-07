#!/usr/bin/env bash
# QA Auto Test: provisioning/02-scim-user-crud
# Doc: docs/qa/provisioning/02-scim-user-crud.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_TENANT_ID=""
_CONNECTOR_ID=""
_SCIM_TOKEN=""
_SCIM_USER_ID=""
_TS=""

_setup() {
  if [[ -n "$_SCIM_TOKEN" ]]; then return 0; fi
  _TS=$(date +%s)

  _TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;")
  _CONNECTOR_ID=$(db_query "SELECT id FROM enterprise_sso_connectors WHERE tenant_id = '${_TENANT_ID}' LIMIT 1;")

  if [[ -z "$_TENANT_ID" || -z "$_CONNECTOR_ID" ]]; then
    echo "Need tenant and SSO connector for SCIM tests" >&2; return 1
  fi

  local admin_id
  admin_id=$(db_query "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;")
  local token
  token=$(gen_tenant_token "$admin_id" "$_TENANT_ID")
  qa_set_token "$token"

  local token_resp
  token_resp=$(api_post "/api/v1/tenants/${_TENANT_ID}/sso/connectors/${_CONNECTOR_ID}/scim/tokens" \
    "{\"description\":\"QA SCIM CRUD test ${_TS}\"}")
  _SCIM_TOKEN=$(resp_body "$token_resp" | jq -r ".token // empty")

  if [[ -z "$_SCIM_TOKEN" ]]; then
    echo "Failed to create SCIM token" >&2; return 1
  fi

  qa_set_token ""

  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = 'qa-scim-${_TS}@example.com');" || true
  db_exec "DELETE FROM users WHERE email = 'qa-scim-${_TS}@example.com';" || true
}

_scim_request() {
  local method="$1" path="$2" data="${3:-}"
  local args=(-s -w '\n%{http_code}' -X "$method"
    -H "Authorization: Bearer ${_SCIM_TOKEN}"
    -H "Content-Type: application/scim+json")
  if [[ -n "$data" ]]; then
    args+=(-d "$data")
  fi
  local body status_code
  body=$(curl "${args[@]}" "${API_BASE}${path}")
  status_code=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '$d')
  printf '%s\n%s' "$status_code" "$body"
}

scenario 1 "SCIM create user" '
  _setup

  local email="qa-scim-${_TS}@example.com"
  resp=$(_scim_request POST /api/v1/scim/v2/Users \
    "{\"schemas\":[\"urn:ietf:params:scim:schemas:core:2.0:User\"],\"userName\":\"${email}\",\"externalId\":\"okta-qa-${_TS}\",\"displayName\":\"QA SCIM User\",\"name\":{\"givenName\":\"QA\",\"familyName\":\"SCIM\"},\"emails\":[{\"value\":\"${email}\",\"type\":\"work\",\"primary\":true}],\"active\":true}")
  assert_http_status "$(resp_status "$resp")" 201 "POST /scim/v2/Users returns 201"

  body=$(resp_body "$resp")
  _SCIM_USER_ID=$(echo "$body" | jq -r ".dataid // empty")
  assert_ne "$_SCIM_USER_ID" "" "response has user id"
  assert_json_field "$body" ".userName" "${email}" "userName matches"
  assert_json_field "$body" ".externalId" "okta-qa-${_TS}" "externalId matches"
  assert_json_field "$body" ".data.active" "true" "user is active"
  assert_json_exists "$body" ".meta" "response has meta"

  assert_db_not_empty \
    "SELECT id FROM users WHERE email = '\''${email}'\'';" \
    "user exists in DB"
  assert_db \
    "SELECT scim_external_id FROM users WHERE email = '\''${email}'\'';" \
    "okta-qa-${_TS}" \
    "scim_external_id set in DB"
'

scenario 2 "SCIM get user and list with filter" '
  _setup

  if [[ -z "$_SCIM_USER_ID" ]]; then
    echo "No SCIM user from scenario 1" >&2; return 1
  fi

  resp_get=$(_scim_request GET "/api/v1/scim/v2/Users/${_SCIM_USER_ID}")
  assert_http_status "$(resp_status "$resp_get")" 200 "GET /scim/v2/Users/{id} returns 200"
  body_get=$(resp_body "$resp_get")
  assert_json_field "$body_get" ".id" "${_SCIM_USER_ID}" "user id matches"

  local email="qa-scim-${_TS}@example.com"
  local encoded_filter
  encoded_filter=$(python3 -c "import urllib.parse; print(urllib.parse.quote('"'"'userName eq \"'"'"'\"${email}\"'"'"'\"'"'"'))" 2>/dev/null || echo "userName%20eq%20%22${email}%22")

  resp_filter=$(_scim_request GET "/api/v1/scim/v2/Users?filter=${encoded_filter}")
  assert_http_status "$(resp_status "$resp_filter")" 200 "GET /scim/v2/Users?filter returns 200"
  body_filter=$(resp_body "$resp_filter")
  total=$(echo "$body_filter" | jq -r ".totalResults // 0")
  assert_match "$total" "^[1-9]" "filter returns at least 1 result"

  resp_list=$(_scim_request GET "/api/v1/scim/v2/Users?startIndex=1&count=10")
  assert_http_status "$(resp_status "$resp_list")" 200 "GET /scim/v2/Users list returns 200"
  body_list=$(resp_body "$resp_list")
  assert_json_exists "$body_list" ".Resources" "list response has Resources"
'

scenario 3 "SCIM PATCH user (incremental update)" '
  _setup

  if [[ -z "$_SCIM_USER_ID" ]]; then
    echo "No SCIM user from scenario 1" >&2; return 1
  fi

  resp=$(_scim_request PATCH "/api/v1/scim/v2/Users/${_SCIM_USER_ID}" \
    "{\"schemas\":[\"urn:ietf:params:scim:api:messages:2.0:PatchOp\"],\"Operations\":[{\"op\":\"replace\",\"path\":\"displayName\",\"value\":\"QA SCIM Updated\"},{\"op\":\"replace\",\"path\":\"active\",\"value\":false}]}")
  assert_http_status "$(resp_status "$resp")" 200 "PATCH /scim/v2/Users/{id} returns 200"

  body=$(resp_body "$resp")
  assert_json_field "$body" ".displayName" "QA SCIM Updated" "displayName updated"
  assert_json_field "$body" ".data.active" "false" "active set to false"

  assert_db \
    "SELECT display_name FROM users WHERE id = '\''${_SCIM_USER_ID}'\'';" \
    "QA SCIM Updated" \
    "display_name updated in DB"

  locked=$(db_query "SELECT CASE WHEN locked_until IS NOT NULL THEN '"'"'locked'"'"' ELSE '"'"'unlocked'"'"' END FROM users WHERE id = '"'"'${_SCIM_USER_ID}'"'"';")
  locked=$(echo "$locked" | tr -d "[:space:]")
  assert_eq "$locked" "locked" "user locked (active=false) in DB"
'

scenario 4 "SCIM PUT full replace user" '
  _setup

  if [[ -z "$_SCIM_USER_ID" ]]; then
    echo "No SCIM user from scenario 1" >&2; return 1
  fi

  local email="qa-scim-${_TS}@example.com"
  resp=$(_scim_request PUT "/api/v1/scim/v2/Users/${_SCIM_USER_ID}" \
    "{\"schemas\":[\"urn:ietf:params:scim:schemas:core:2.0:User\"],\"userName\":\"${email}\",\"externalId\":\"okta-qa-${_TS}\",\"displayName\":\"QA SCIM Replaced\",\"active\":true}")
  assert_http_status "$(resp_status "$resp")" 200 "PUT /scim/v2/Users/{id} returns 200"

  body=$(resp_body "$resp")
  assert_json_field "$body" ".displayName" "QA SCIM Replaced" "displayName replaced"
  assert_json_field "$body" ".data.active" "true" "user reactivated"

  assert_db \
    "SELECT display_name FROM users WHERE id = '\''${_SCIM_USER_ID}'\'';" \
    "QA SCIM Replaced" \
    "display_name replaced in DB"

  unlocked=$(db_query "SELECT CASE WHEN locked_until IS NULL THEN '"'"'unlocked'"'"' ELSE '"'"'locked'"'"' END FROM users WHERE id = '"'"'${_SCIM_USER_ID}'"'"';")
  unlocked=$(echo "$unlocked" | tr -d "[:space:]")
  assert_eq "$unlocked" "unlocked" "user unlocked (active=true) in DB"
'

scenario 5 "SCIM DELETE user (soft delete)" '
  _setup

  if [[ -z "$_SCIM_USER_ID" ]]; then
    echo "No SCIM user from scenario 1" >&2; return 1
  fi

  resp=$(_scim_request DELETE "/api/v1/scim/v2/Users/${_SCIM_USER_ID}")
  assert_http_status "$(resp_status "$resp")" 204 "DELETE /scim/v2/Users/{id} returns 204"

  assert_db_not_empty \
    "SELECT id FROM users WHERE id = '\''${_SCIM_USER_ID}'\'';" \
    "user record still exists (soft delete)"

  locked=$(db_query "SELECT CASE WHEN locked_until IS NOT NULL THEN '"'"'locked'"'"' ELSE '"'"'unlocked'"'"' END FROM users WHERE id = '"'"'${_SCIM_USER_ID}'"'"';")
  locked=$(echo "$locked" | tr -d "[:space:]")
  assert_eq "$locked" "locked" "user locked after SCIM delete"

  resp_unauth=$(_scim_request GET /api/v1/scim/v2/Users)
  assert_http_status "$(resp_status "$resp_unauth")" 200 "authenticated request still works"

  resp_no_auth=$(api_raw GET /api/v1/scim/v2/Users)
  status_no_auth=$(resp_status "$resp_no_auth")
  assert_eq "$status_no_auth" "401" "unauthenticated SCIM request returns 401"

  local email="qa-scim-${_TS}@example.com"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''${email}'\'');" || true
  db_exec "DELETE FROM users WHERE email = '\''${email}'\'';" || true
'

run_all
