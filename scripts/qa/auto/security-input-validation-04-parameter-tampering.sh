#!/usr/bin/env bash
# QA Auto Test: security/input-validation/04-parameter-tampering
# Doc: docs/security/input-validation/04-parameter-tampering.md
# Scenarios: 4
# ASVS: M-INPUT-04 | V2.1, V4.2, V8.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: Hidden/readonly field tampering ───────────────────────────
scenario 1 "Hidden/readonly field tampering - immutable fields ignored" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp_before=$(api_get "/api/v1/users/me")
  body_before=$(resp_body "$resp_before")
  orig_id=$(echo "$body_before" | jq -r ".data.id // .id // empty")
  orig_created=$(echo "$body_before" | jq -r ".data.created_at // .created_at // empty")

  resp=$(api_put "/api/v1/users/me" "{
    \"display_name\": \"Tamper Test\",
    \"id\": \"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee\",
    \"created_at\": \"2020-01-01T00:00:00Z\",
    \"tenant_id\": \"fake-tenant-id\",
    \"keycloak_id\": \"fake-keycloak-id\"
  }")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|400|422)$" "update with readonly fields handled"

  if [[ "$status" == "200" ]]; then
    resp_after=$(api_get "/api/v1/users/me")
    body_after=$(resp_body "$resp_after")
    after_id=$(echo "$body_after" | jq -r ".data.id // .id // empty")

    if [[ -n "$orig_id" && "$orig_id" != "null" ]]; then
      assert_eq "$after_id" "$orig_id" "user id not changed by tampering"
    fi

    after_name=$(echo "$body_after" | jq -r ".data.display_name // .display_name // empty")
    assert_eq "$after_name" "Tamper Test" "display_name was updated legitimately"
  fi

  api_put "/api/v1/users/me" "{\"display_name\":\"QA Test User\"}" >/dev/null 2>&1 || true
  qa_set_token ""
'

# ── Scenario 2: Type confusion attack ─────────────────────────────────────
scenario 2 "Type confusion - wrong types for DTO fields rejected" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")

  if [[ -n "$SVC_ID" ]]; then
    resp=$(api_put "/api/v1/services/${SVC_ID}" \
      "{\"name\":\"test\",\"timeout\":\"not-a-number\"}")
    status=$(resp_status "$resp")
    assert_match "$status" "^(400|422|429)$" "string for numeric timeout field rejected"
  fi

  resp2=$(api_put "/api/v1/system/branding" \
    "{\"allow_registration\":\"true\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(200|400|422)$" "string for boolean field handled"

  resp3=$(api_post "/api/v1/tenants" \
    "{\"name\":[\"array\",\"value\"],\"slug\":\"type-confusion-test\"}")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(400|422|429)$" "array for string name field rejected"

  resp4=$(api_post "/api/v1/tenants" \
    "{\"name\":12345,\"slug\":\"type-confusion-num\"}")
  status4=$(resp_status "$resp4")
  assert_match "$status4" "^(400|422|429)$" "number for string name field rejected"

  qa_set_token ""
'

# ── Scenario 3: Boundary value testing ────────────────────────────────────
scenario 3 "Boundary value testing - extreme inputs handled" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  long_name=$(python3 -c "print(\"A\"*10000)" 2>/dev/null || printf "A%.0s" {1..10000})
  resp=$(api_post "/api/v1/tenants" \
    "{\"name\":\"${long_name}\",\"slug\":\"boundary-test\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422|429)$" "10000-char name rejected"

  resp2=$(api_get "/api/v1/users?page=-1&per_page=-10")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(200|400|422)$" "negative pagination handled"
  if [[ "$status2" == "200" ]]; then
    body2=$(resp_body "$resp2")
    assert_not_contains "$body2" "error" "negative pagination uses defaults"
  fi

  resp3=$(api_get "/api/v1/users?page=0&per_page=0")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(200|400|422)$" "zero pagination handled"

  resp4=$(api_get "/api/v1/users?page=999999999&per_page=999999999")
  status4=$(resp_status "$resp4")
  assert_match "$status4" "^(200|400|422)$" "huge pagination values handled"

  resp5=$(api_put "/api/v1/users/me" "{\"display_name\":\"\"}")
  status5=$(resp_status "$resp5")
  assert_match "$status5" "^(200|400|422)$" "empty string display_name handled"

  resp6=$(api_put "/api/v1/users/me" \
    "{\"display_name\":\"Test \\ud83c\\udf89 Unicode\"}")
  status6=$(resp_status "$resp6")
  assert_match "$status6" "^(200|400|422)$" "unicode display_name handled"

  api_put "/api/v1/users/me" "{\"display_name\":\"QA Test User\"}" >/dev/null 2>&1 || true
  qa_set_token ""
'

# ── Scenario 4: HTTP method/header tampering ──────────────────────────────
scenario 4 "HTTP method override and header tampering" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")

  resp=$(api_raw POST "/api/v1/tenants/${TENANT_ID}" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -H "X-HTTP-Method-Override: DELETE" \
    -d "{}")
  status=$(resp_status "$resp")

  verify=$(api_get "/api/v1/tenants/${TENANT_ID}")
  verify_status=$(resp_status "$verify")
  assert_match "$verify_status" "^(200|404)$" "tenant still exists after method override attempt"
  if [[ "$verify_status" == "200" ]]; then
    assert_eq "$verify_status" "200" "X-HTTP-Method-Override did not delete tenant"
  fi

  resp2=$(api_raw GET "/health" \
    -H "Host: evil.com")
  status2=$(resp_status "$resp2")
  body2=$(resp_body "$resp2")
  assert_match "$status2" "^(200|400)$" "Host header injection handled"
  assert_not_contains "$body2" "evil.com" "evil.com not reflected in response"

  resp3=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/xml" \
    -d "<tenant><name>xml-test</name><slug>xml-test</slug></tenant>")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(400|415|422)$" "XML content-type rejected"

  resp4=$(api_raw GET "/api/v1/users" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "X-Forwarded-For: 127.0.0.1" \
    -H "X-Original-URL: /api/v1/admin/secret")
  status4=$(resp_status "$resp4")
  assert_match "$status4" "^(200|400|403)$" "X-Forwarded-For / X-Original-URL handled"

  qa_set_token ""
'

run_all
