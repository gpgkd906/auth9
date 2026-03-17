#!/usr/bin/env bash
# QA Auto Test: integration/14-keycloak-adapter-layer
# Doc: docs/qa/integration/14-keycloak-adapter-layer.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node
require_bin docker
require_bin mysql

_gen_id_token_with_sid() {
  local user_id="$1" email="$2" sid="$3"
  node -e '
const jwt=require("jsonwebtoken"),fs=require("fs");
const now=Math.floor(Date.now()/1000);
const pk=fs.readFileSync(process.argv[1],"utf8");
process.stdout.write(jwt.sign({
  sub:process.argv[2],email:process.argv[3],
  iss:"http://localhost:8080",aud:"auth9",token_type:"identity",
  iat:now,exp:now+3600,sid:process.argv[4]
},pk,{algorithm:"RS256",keyid:"auth9-current"}));
' "$_JWT_PRIVATE_KEY" "$user_id" "$email" "$sid" 2>/dev/null
}

_kc_admin() {
  docker exec auth9-keycloak /opt/keycloak/bin/kcadm.sh "$@" \
    --server http://localhost:8080 --realm master --user admin --password admin
}

_tenant_owner_token_for_user() {
  local user_id="$1"
  local tenant_id
  tenant_id=$(qa_get_tenant_id)
  node "$_TOKEN_TOOLS_DIR/gen-test-tokens.js" tenant-owner --user-id "$user_id" --tenant-id "$tenant_id"
}

scenario 1 "Health check and adapter injection chain" '
  resp=$(api_get /health)
  assert_http_status "$(resp_status "$resp")" 200 "GET /health returns 200"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".status" "healthy" "health status is healthy"
'

scenario 2 "Session revoke path via KeycloakSessionStoreAdapter" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-fr2-session'\'' );" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-fr2-session'\'';" || true

  local uid sid_current sid_revoke token resp
  uid=$(db_query "SELECT LOWER(UUID());")
  sid_current=$(db_query "SELECT LOWER(UUID());")
  sid_revoke=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-fr2-session'\'', '\''qa-fr2-session@example.com'\'', '\''QA FR2 Session'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid_current'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.10'\'', '\''Tokyo'\'', NOW()), ('\''$sid_revoke'\'', '\''$uid'\'', '\''mobile'\'', '\''192.168.1.11'\'', '\''Osaka'\'', NOW());"

  token=$(_gen_id_token_with_sid "$uid" "qa-fr2-session@example.com" "$sid_current")
  qa_set_token "$token"

  resp=$(api_delete "/api/v1/users/me/sessions/$sid_revoke")
  assert_http_status "$(resp_status "$resp")" 200 "DELETE /api/v1/users/me/sessions/{id} returns 200"
  assert_db_not_empty "SELECT revoked_at FROM sessions WHERE id = '\''$sid_revoke'\'' AND revoked_at IS NOT NULL;" "revoked session has revoked_at"
  assert_db_not_empty "SELECT id FROM sessions WHERE id = '\''$sid_current'\'' AND revoked_at IS NULL;" "current session remains active"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 3 "Identity provider CRUD via KeycloakFederationBrokerAdapter" '
  local admin_uid token alias resp body
  admin_uid=$(qa_get_admin_id)
  token=$(_tenant_owner_token_for_user "$admin_uid")
  qa_set_token "$token"
  alias="qa-fr2-idp-$$"

  resp=$(api_post /api/v1/identity-providers "{\"alias\":\"$alias\",\"display_name\":\"QA FR2 OIDC\",\"provider_id\":\"oidc\",\"enabled\":true,\"trust_email\":true,\"config\":{\"clientId\":\"qa-fr2-client\",\"clientSecret\":\"qa-fr2-secret\",\"authorizationUrl\":\"https://sso.corp.example.com/oauth2/authorize\",\"tokenUrl\":\"https://sso.corp.example.com/oauth2/token\"}}")
  assert_http_status "$(resp_status "$resp")" 200 "create identity provider returns 200"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.alias" "$alias" "created provider alias matches"

  resp=$(api_get "/api/v1/identity-providers/$alias")
  assert_http_status "$(resp_status "$resp")" 200 "get identity provider returns 200"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.alias" "$alias" "fetched provider alias matches"

  resp=$(api_put "/api/v1/identity-providers/$alias" "{\"display_name\":\"QA FR2 OIDC Updated\"}")
  assert_http_status "$(resp_status "$resp")" 200 "update identity provider returns 200"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.display_name" "QA FR2 OIDC Updated" "updated provider display name matches"

  resp=$(api_delete "/api/v1/identity-providers/$alias")
  assert_http_status "$(resp_status "$resp")" 200 "delete identity provider returns 200"
  qa_set_token ""
'

scenario 4 "Linked identity list and unlink via adapter layer" '
  local admin_uid admin_kc_id token alias identity_id resp body
  admin_uid=$(qa_get_admin_id)
  admin_kc_id=$(db_query "SELECT keycloak_id FROM users WHERE id = '\''$admin_uid'\'' LIMIT 1;" | tr -d "[:space:]")
  token=$(_tenant_owner_token_for_user "$admin_uid")
  alias="qa-fr2-link-$$"

  db_exec "DELETE FROM linked_identities WHERE user_id = '\''$admin_uid'\'' AND provider_alias = '\''$alias'\'';" || true
  qa_set_token "$token"

  resp=$(api_post /api/v1/identity-providers "{\"alias\":\"$alias\",\"display_name\":\"QA FR2 Link\",\"provider_id\":\"oidc\",\"enabled\":true,\"trust_email\":true,\"config\":{\"clientId\":\"qa-link-client\",\"clientSecret\":\"qa-link-secret\",\"authorizationUrl\":\"https://sso.corp.example.com/oauth2/authorize\",\"tokenUrl\":\"https://sso.corp.example.com/oauth2/token\"}}")
  assert_http_status "$(resp_status "$resp")" 200 "create linked-identity provider returns 200"

  cat > /tmp/qa-fr2-fed.json <<JSON
{
  "identityProvider": "$alias",
  "userId": "ext-$alias",
  "userName": "admin@auth9.local"
}
JSON
  docker cp /tmp/qa-fr2-fed.json auth9-keycloak:/tmp/qa-fr2-fed.json
  _kc_admin create "users/$admin_kc_id/federated-identity/$alias" -r auth9 -f /tmp/qa-fr2-fed.json >/dev/null

  db_exec "INSERT INTO linked_identities (id, user_id, provider_type, provider_alias, external_user_id, external_email) VALUES (LOWER(UUID()), '\''$admin_uid'\'', '\''$alias'\'', '\''$alias'\'', '\''ext-$alias'\'', '\''admin@auth9.local'\'');"

  resp=$(api_get /api/v1/users/me/linked-identities)
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/users/me/linked-identities returns 200"
  body=$(resp_body "$resp")
  assert_contains "$body" "$alias" "linked identities response contains adapter-backed alias"

  identity_id=$(db_query "SELECT id FROM linked_identities WHERE user_id = '\''$admin_uid'\'' AND provider_alias = '\''$alias'\'' LIMIT 1;" | tr -d "[:space:]")
  resp=$(api_delete "/api/v1/users/me/linked-identities/$identity_id")
  assert_http_status "$(resp_status "$resp")" 200 "DELETE /api/v1/users/me/linked-identities/{id} returns 200"
  assert_db "SELECT COUNT(*) FROM linked_identities WHERE id = '\''$identity_id'\'';" "0" "linked identity row deleted"

  local kc_after
  kc_after=$(_kc_admin get "users/$admin_kc_id/federated-identity" -r auth9)
  assert_not_contains "$kc_after" "$alias" "Keycloak federated identity removed"

  api_delete "/api/v1/identity-providers/$alias" >/dev/null || true
  db_exec "DELETE FROM linked_identities WHERE user_id = '\''$admin_uid'\'' AND provider_alias = '\''$alias'\'';" || true
  qa_set_token ""
'

run_all
