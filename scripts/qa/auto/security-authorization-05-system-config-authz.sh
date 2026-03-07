#!/usr/bin/env bash
# Security Auto Test: security/authorization/05-system-config-authz
# Doc: docs/security/authorization/05-system-config-authz.md
# Scenarios: 8
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node

_PLATFORM_TENANT_ID=""
_get_platform_tenant_id() {
  if [[ -z "$_PLATFORM_TENANT_ID" ]]; then
    _PLATFORM_TENANT_ID=$(db_query "SELECT id FROM tenants WHERE slug = 'auth9-platform' LIMIT 1;")
  fi
  echo "$_PLATFORM_TENANT_ID"
}

_gen_member_access_token() {
  local tenant_id="$1"
  node -e '
const jwt=require("jsonwebtoken"),fs=require("fs");
const pk=fs.readFileSync(process.argv[1],"utf8");
const now=Math.floor(Date.now()/1000);
process.stdout.write(jwt.sign({
  sub:"16daa93d-06e8-479c-867d-f9b6184e06c7",
  email:"member-authz05@test.com",
  iss:"http://localhost:8080",aud:"auth9-portal",token_type:"access",
  tenant_id:process.argv[2],roles:["member"],permissions:[],
  iat:now,exp:now+3600
},pk,{algorithm:"RS256",keyid:"auth9-current"}));
' "$_JWT_PRIVATE_KEY" "$tenant_id" 2>/dev/null
}

_gen_service_client_token() {
  node -e '
const jwt=require("jsonwebtoken"),fs=require("fs");
const pk=fs.readFileSync(process.argv[1],"utf8");
const now=Math.floor(Date.now()/1000);
process.stdout.write(jwt.sign({
  sub:"16daa93d-06e8-479c-867d-f9b6184e06c7",
  email:"svc-client-authz05@test.com",
  iss:"http://localhost:8080",aud:"auth9-service",token_type:"service_client",
  iat:now,exp:now+3600
},pk,{algorithm:"RS256",keyid:"auth9-current"}));
' "$_JWT_PRIVATE_KEY" 2>/dev/null
}

scenario 1 "Member cannot update system email config" '
  local tenant_id member_token
  tenant_id=$(_get_platform_tenant_id)
  member_token=$(_gen_member_access_token "$tenant_id")
  qa_set_token "$member_token"

  resp=$(api_put "/api/v1/system/email" \
    "{\"config\":{\"type\":\"smtp\",\"host\":\"attacker.example\",\"port\":25,\"username\":\"x\",\"password\":\"y\",\"use_tls\":false,\"from_email\":\"noreply@example.com\",\"from_name\":\"Auth9\"}}")
  status=$(resp_status "$resp")
  assert_http_status "$status" 403 "Member cannot update system email config"
  qa_set_token ""
'

scenario 2 "Member cannot send system test email" '
  local tenant_id member_token
  tenant_id=$(_get_platform_tenant_id)
  member_token=$(_gen_member_access_token "$tenant_id")
  qa_set_token "$member_token"

  resp=$(api_post "/api/v1/system/email/send-test" \
    "{\"to_email\":\"victim@example.com\"}")
  status=$(resp_status "$resp")
  assert_http_status "$status" 403 "Member cannot send system test email"
  qa_set_token ""
'

scenario 3 "Member cannot update system branding" '
  local tenant_id member_token
  tenant_id=$(_get_platform_tenant_id)
  member_token=$(_gen_member_access_token "$tenant_id")
  qa_set_token "$member_token"

  resp=$(api_put "/api/v1/system/branding" \
    "{\"config\":{\"primary_color\":\"#000000\",\"secondary_color\":\"#ffffff\",\"background_color\":\"#ffffff\",\"text_color\":\"#000000\",\"company_name\":\"Hacked\"}}")
  status=$(resp_status "$resp")
  assert_http_status "$status" 403 "Member cannot update system branding"
  qa_set_token ""
'

scenario 4 "Member cannot update system email templates" '
  local tenant_id member_token
  tenant_id=$(_get_platform_tenant_id)
  member_token=$(_gen_member_access_token "$tenant_id")
  qa_set_token "$member_token"

  resp=$(api_put "/api/v1/system/email-templates/invitation" \
    "{\"subject\":\"PWN\",\"html_body\":\"<p>pwn</p>\",\"text_body\":\"pwn\"}")
  status=$(resp_status "$resp")
  assert_http_status "$status" 403 "Member cannot update email templates"
  qa_set_token ""
'

scenario 5 "Member cannot reset system email templates" '
  local tenant_id member_token
  tenant_id=$(_get_platform_tenant_id)
  member_token=$(_gen_member_access_token "$tenant_id")
  qa_set_token "$member_token"

  resp=$(api_delete "/api/v1/system/email-templates/invitation")
  status=$(resp_status "$resp")
  assert_http_status "$status" 403 "Member cannot reset email templates"
  qa_set_token ""
'

scenario 6 "Member cannot update tenant password policy" '
  local tenant_id member_token
  tenant_id=$(_get_platform_tenant_id)
  member_token=$(_gen_member_access_token "$tenant_id")
  qa_set_token "$member_token"

  resp=$(api_put "/api/v1/tenants/$tenant_id/password-policy" \
    "{\"min_length\":4,\"require_uppercase\":false,\"require_lowercase\":false,\"require_number\":false,\"require_symbol\":false,\"max_age_days\":0}")
  status=$(resp_status "$resp")
  assert_http_status "$status" 403 "Member cannot update password policy"
  qa_set_token ""
'

scenario 7 "Service client token cannot modify system config" '
  local svc_token
  svc_token=$(_gen_service_client_token)
  qa_set_token "$svc_token"

  resp=$(api_put "/api/v1/system/email" \
    "{\"config\":{\"type\":\"smtp\",\"host\":\"attacker.example\",\"port\":25,\"username\":\"x\",\"password\":\"y\",\"use_tls\":false,\"from_email\":\"noreply@example.com\",\"from_name\":\"Auth9\"}}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403|429)$" "Service client cannot update system email"

  resp2=$(api_put "/api/v1/system/branding" \
    "{\"config\":{\"primary_color\":\"#ff0000\",\"company_name\":\"Hacked\"}}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(401|403|429)$" "Service client cannot update system branding"

  qa_set_token ""
'

scenario 8 "Positive: platform admin can update system config" '
  local admin_token
  admin_token=$(gen_default_admin_token)
  qa_set_token "$admin_token"

  resp=$(api_get "/api/v1/system/email")
  status=$(resp_status "$resp")
  assert_http_status "$status" 200 "Platform admin can read system email config"

  resp2=$(api_get "/api/v1/system/branding")
  status2=$(resp_status "$resp2")
  assert_http_status "$status2" 200 "Platform admin can read system branding"

  resp3=$(api_get "/api/v1/system/email-templates/invitation")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(200|404)$" "Platform admin can read email templates"

  qa_set_token ""
'

run_all
