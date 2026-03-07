#!/usr/bin/env bash
# QA Auto Test: identity-provider/03-tenant-enterprise-sso-connectors
# Doc: docs/qa/identity-provider/03-tenant-enterprise-sso-connectors.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_TENANT_ID=""
_SAML_CONNECTOR_ID=""
_OIDC_CONNECTOR_ID=""
_TS=""

_setup() {
  if [[ -n "$_TENANT_ID" ]]; then return 0; fi
  _TS=$(date +%s)
  _TENANT_ID=$(qa_get_tenant_id)
  if [[ -z "$_TENANT_ID" ]]; then
    echo "No active tenant found" >&2; return 1
  fi

  local admin_id
  admin_id=$(db_query "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;")
  TOKEN=$(gen_tenant_token "$admin_id" "$_TENANT_ID")
  qa_set_token "$TOKEN"

  db_exec "DELETE FROM enterprise_sso_domains WHERE domain IN ('qa-saml-${_TS}.example.com','qa-oidc-${_TS}.example.com','qa-saml-${_TS}-new.example.com');" || true
  db_exec "DELETE FROM enterprise_sso_connectors WHERE alias IN ('qa-saml-${_TS}','qa-oidc-${_TS}');" || true
}

scenario 1 "Create SAML connector" '
  _setup

  local alias="qa-saml-${_TS}"
  local domain="qa-saml-${_TS}.example.com"

  resp=$(api_post "/api/v1/tenants/${_TENANT_ID}/sso/connectors" \
    "{\"alias\":\"${alias}\",\"provider_type\":\"saml\",\"enabled\":true,\"domains\":[\"${domain}\"],\"config\":{\"entityId\":\"https://idp.example.com/saml/metadata\",\"ssoUrl\":\"https://idp.example.com/saml/sso\",\"certificate\":\"MIIDpDCCAoygAwIBAgIGAX...\"}}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|201)$" "create SAML connector returns 200/201"

  body=$(resp_body "$resp")
  _SAML_CONNECTOR_ID=$(echo "$body" | jq -r ".data.id // .id // empty")
  assert_ne "$_SAML_CONNECTOR_ID" "" "response returns connector id"

  provider=$(echo "$body" | jq -r ".data.provider_type // .provider_type // empty")
  assert_eq "$provider" "saml" "provider_type is saml"

  assert_db_not_empty \
    "SELECT id FROM enterprise_sso_connectors WHERE tenant_id = '\''${_TENANT_ID}'\'' AND alias = '\''${alias}'\'';" \
    "SAML connector exists in DB"

  assert_db_not_empty \
    "SELECT domain FROM enterprise_sso_domains WHERE domain = '\''${domain}'\'';" \
    "domain binding exists in DB"
'

scenario 2 "Create OIDC connector" '
  _setup

  local alias="qa-oidc-${_TS}"
  local domain="qa-oidc-${_TS}.example.com"

  resp=$(api_post "/api/v1/tenants/${_TENANT_ID}/sso/connectors" \
    "{\"alias\":\"${alias}\",\"provider_type\":\"oidc\",\"enabled\":true,\"priority\":100,\"domains\":[\"${domain}\"],\"config\":{\"clientId\":\"qa-oidc-client\",\"clientSecret\":\"qa-oidc-secret\",\"authorizationUrl\":\"https://idp.example.com/authorize\",\"tokenUrl\":\"https://idp.example.com/token\"}}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|201)$" "create OIDC connector returns 200/201"

  body=$(resp_body "$resp")
  _OIDC_CONNECTOR_ID=$(echo "$body" | jq -r ".data.id // .id // empty")
  assert_ne "$_OIDC_CONNECTOR_ID" "" "response returns connector id"

  provider=$(echo "$body" | jq -r ".data.provider_type // .provider_type // empty")
  assert_eq "$provider" "oidc" "provider_type is oidc"

  assert_db_not_empty \
    "SELECT id FROM enterprise_sso_connectors WHERE tenant_id = '\''${_TENANT_ID}'\'' AND alias = '\''${alias}'\'';" \
    "OIDC connector exists in DB"

  resp_list=$(api_get "/api/v1/tenants/${_TENANT_ID}/sso/connectors")
  assert_http_status "$(resp_status "$resp_list")" 200 "list connectors returns 200"
  list_body=$(resp_body "$resp_list")
  assert_contains "$list_body" "${alias}" "OIDC connector appears in list"
'

scenario 3 "Domain conflict on create" '
  _setup

  if [[ -z "$_SAML_CONNECTOR_ID" ]]; then
    echo "No SAML connector from scenario 1" >&2; return 1
  fi

  local conflict_domain="qa-saml-${_TS}.example.com"
  resp=$(api_post "/api/v1/tenants/${_TENANT_ID}/sso/connectors" \
    "{\"alias\":\"qa-conflict-${_TS}\",\"provider_type\":\"saml\",\"enabled\":true,\"domains\":[\"${conflict_domain}\"],\"config\":{\"entityId\":\"https://other.example.com/saml\",\"ssoUrl\":\"https://other.example.com/sso\",\"certificate\":\"MIIDpDCCAoy...\"}}")
  assert_http_status "$(resp_status "$resp")" 409 "duplicate domain returns 409"

  assert_db "SELECT COUNT(*) FROM enterprise_sso_domains WHERE domain = '\''${conflict_domain}'\'';" \
    "1" "only one domain binding exists"
'

scenario 4 "Update connector enabled status and domains" '
  _setup

  if [[ -z "$_SAML_CONNECTOR_ID" ]]; then
    echo "No SAML connector from scenario 1" >&2; return 1
  fi

  local new_domain="qa-saml-${_TS}-new.example.com"
  resp=$(api_put "/api/v1/tenants/${_TENANT_ID}/sso/connectors/${_SAML_CONNECTOR_ID}" \
    "{\"enabled\":false,\"domains\":[\"${new_domain}\"]}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|204)$" "update connector returns 200/204"

  enabled_val=$(db_query "SELECT enabled FROM enterprise_sso_connectors WHERE id = '\''${_SAML_CONNECTOR_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$enabled_val" "0" "connector disabled in DB"

  domain_val=$(db_query "SELECT domain FROM enterprise_sso_domains WHERE connector_id = '\''${_SAML_CONNECTOR_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$domain_val" "${new_domain}" "domain updated in DB"
'

scenario 5 "Test connection and delete connector" '
  _setup

  if [[ -z "$_OIDC_CONNECTOR_ID" ]]; then
    echo "No OIDC connector from scenario 2" >&2; return 1
  fi

  resp_test=$(api_post "/api/v1/tenants/${_TENANT_ID}/sso/connectors/${_OIDC_CONNECTOR_ID}/test" "{}")
  status_test=$(resp_status "$resp_test")
  assert_match "$status_test" "^(200|400|502)$" "test connection returns structured result"

  resp_del=$(api_delete "/api/v1/tenants/${_TENANT_ID}/sso/connectors/${_OIDC_CONNECTOR_ID}")
  status_del=$(resp_status "$resp_del")
  assert_match "$status_del" "^(200|204)$" "delete connector returns 200/204"

  assert_db "SELECT COUNT(*) FROM enterprise_sso_connectors WHERE id = '\''${_OIDC_CONNECTOR_ID}'\'';" \
    "0" "connector deleted from DB"
  assert_db "SELECT COUNT(*) FROM enterprise_sso_domains WHERE connector_id = '\''${_OIDC_CONNECTOR_ID}'\'';" \
    "0" "domain bindings deleted from DB"

  if [[ -n "$_SAML_CONNECTOR_ID" ]]; then
    api_delete "/api/v1/tenants/${_TENANT_ID}/sso/connectors/${_SAML_CONNECTOR_ID}" >/dev/null 2>&1 || true
  fi
  db_exec "DELETE FROM enterprise_sso_domains WHERE domain LIKE '\''qa-%-${_TS}%'\'';" || true
  db_exec "DELETE FROM enterprise_sso_connectors WHERE alias LIKE '\''qa-%-${_TS}%'\'';" || true
'

run_all
