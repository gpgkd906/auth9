#!/usr/bin/env bash
# QA Auto Test: security/input-validation/01-injection
# Doc: docs/security/input-validation/01-injection.md
# Scenarios: 5
# ASVS: M-INPUT-01 | V1.2, V2.1, V4.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: SQL injection - authentication bypass ──────────────────────
scenario 1 "SQL injection - authentication bypass via forgot-password" '
  payloads=(
    "admin'\''--@example.com"
    "'\'' OR '\''1'\''='\''1@example.com"
    "admin'\''; DROP TABLE users;--@example.com"
    "%27%20OR%20%271%27%3D%271@example.com"
  )

  for p in "${payloads[@]}"; do
    resp=$(api_post "/api/v1/auth/forgot-password" "{\"email\":\"${p}\"}")
    status=$(resp_status "$resp")
    body=$(resp_body "$resp")
    assert_match "$status" "^(400|404|422|200|429)$" "SQLi auth bypass payload rejected or handled safely: ${p:0:30}"
    assert_not_contains "$body" "syntax error" "no SQL syntax error in response"
    assert_not_contains "$body" "mysql" "no mysql reference in response"
    assert_not_contains "$body" "SQL" "no SQL reference in response"
  done
'

# ── Scenario 2: SQL injection - data extraction via search ─────────────────
scenario 2 "SQL injection - data extraction via search endpoints" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  sqli_payloads=(
    "test'\'' UNION SELECT password FROM users--"
    "test'\'' AND '\''1'\''='\''1"
    "test'\'' AND '\''1'\''='\''2"
    "test'\'' OR 1=1--"
    "1; DROP TABLE users;--"
  )

  for p in "${sqli_payloads[@]}"; do
    encoded=$(python3 -c "import urllib.parse; print(urllib.parse.quote(\"${p}\"))" 2>/dev/null || echo "$p")
    resp=$(api_get "/api/v1/users?search=${encoded}")
    status=$(resp_status "$resp")
    body=$(resp_body "$resp")
    assert_match "$status" "^(200|400|422)$" "SQLi search payload handled: ${p:0:30}"
    assert_not_contains "$body" "password" "no password field leaked via SQLi"
    assert_not_contains "$body" "syntax error" "no SQL syntax error exposed"
  done

  resp1=$(api_get "/api/v1/users?search=test%27%20AND%20%271%27%3D%271")
  resp2=$(api_get "/api/v1/users?search=test%27%20AND%20%271%27%3D%272")
  body1=$(resp_body "$resp1")
  body2=$(resp_body "$resp2")
  status1=$(resp_status "$resp1")
  status2=$(resp_status "$resp2")
  assert_eq "$status1" "$status2" "boolean blind SQLi: same status for 1=1 vs 1=2"

  qa_set_token ""
'

# ── Scenario 3: NoSQL / Redis injection ────────────────────────────────────
scenario 3 "NoSQL / Redis command injection via user-controlled params" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  redis_payloads=(
    "test%0D%0AKEYS%20*%0D%0A"
    "test%0D%0AFLUSHALL%0D%0A"
    "test%0D%0ACONFIG%20SET%20dir%20/tmp%0D%0A"
  )

  for p in "${redis_payloads[@]}"; do
    resp=$(api_get "/api/v1/users?search=${p}")
    status=$(resp_status "$resp")
    body=$(resp_body "$resp")
    assert_match "$status" "^(200|400|422)$" "Redis injection payload handled: ${p:0:30}"
    assert_not_contains "$body" "FLUSHALL" "no Redis FLUSHALL in response"
    assert_not_contains "$body" "CONFIG" "no Redis CONFIG in response"
  done

  resp=$(api_raw GET "/api/v1/auth/authorize" \
    -H "X-Session-ID: test\r\nFLUSHALL\r\n")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|302|400|404)$" "session header injection handled safely"

  qa_set_token ""
'

# ── Scenario 4: LDAP / Keycloak injection ──────────────────────────────────
scenario 4 "LDAP / Keycloak search injection" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  ldap_payloads=(
    "*)(%26"
    "admin)(|(password=*)"
    "*)(uid=*"
    "test)(cn=*))(|(cn=*"
  )

  for p in "${ldap_payloads[@]}"; do
    encoded=$(python3 -c "import urllib.parse; print(urllib.parse.quote(\"${p}\"))" 2>/dev/null || echo "$p")
    resp=$(api_get "/api/v1/users?search=${encoded}")
    status=$(resp_status "$resp")
    body=$(resp_body "$resp")
    assert_match "$status" "^(200|400|422)$" "LDAP injection payload handled: ${p:0:30}"
    assert_not_contains "$body" "password" "no password leaked via LDAP injection"
    assert_not_contains "$body" "LDAP" "no LDAP error exposed"
    assert_not_contains "$body" "javax.naming" "no Java LDAP trace"
  done

  qa_set_token ""
'

# ── Scenario 5: Command injection ─────────────────────────────────────────
scenario 5 "OS command injection via various input fields" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  cmd_payloads=(
    "; cat /etc/passwd"
    "| whoami"
    "\`whoami\`"
    "\$(cat /etc/passwd)"
    "& ping -c 1 127.0.0.1 &"
  )

  for p in "${cmd_payloads[@]}"; do
    resp=$(api_post "/api/v1/auth/forgot-password" "{\"email\":\"test${p}@example.com\"}")
    status=$(resp_status "$resp")
    body=$(resp_body "$resp")
    assert_match "$status" "^(400|404|422|200|429)$" "cmd injection handled: ${p:0:20}"
    assert_not_contains "$body" "root:" "no /etc/passwd content"
    assert_not_contains "$body" "bin/bash" "no shell path leaked"
  done

  resp=$(api_post "/api/v1/auth/forgot-password" \
    "{\"email\":\"test@example.com; cat /etc/passwd |\"}")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(400|422|200)$" "email cmd injection rejected or safe"
  assert_not_contains "$body" "root:" "no /etc/passwd via email field"

  qa_set_token ""
'

run_all
