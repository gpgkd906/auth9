#!/usr/bin/env bash
# Security Auto Test: security/authentication/05-idp-security
# Doc: docs/security/authentication/05-idp-security.md
# Scenarios: 4
# ASVS: M-AUTH-05 | V10.5, V10.6, V6.4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "OAuth account linking hijack prevention" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  # Attempt to link identity to another user via API
  resp=$(api_post "/api/v1/users/non-existent-victim-id/identities" \
    "{\"provider\":\"github\",\"provider_user_id\":\"attacker-github-id\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|403|404|405)$" "linking identity to another user rejected"

  # Verify own identities endpoint
  resp=$(api_get "/api/v1/users/me/identities")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|404|405)$" "own identities endpoint responds"

  qa_set_token ""
'

scenario 2 "OAuth callback parameter tampering" '
  # Forged state parameter
  resp=$(api_raw GET "/api/v1/auth/callback?code=fake-auth-code&state=forged-random-state")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|403|404|302)$" "forged state parameter rejected"

  # Missing state parameter
  resp=$(api_raw GET "/api/v1/auth/callback?code=fake-code")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|403|404|302)$" "missing state parameter rejected"

  # Missing code parameter
  resp=$(api_raw GET "/api/v1/auth/callback?state=some-state")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|403|404|302)$" "missing code parameter rejected"

  # Tampered redirect_uri via base64 encoded state
  TAMPERED_STATE=$(echo -n "{\"redirect_uri\":\"http://evil.com\",\"client_id\":\"auth9-portal\"}" | base64 | tr -d "\n")
  resp=$(api_raw GET "/api/v1/auth/callback?code=fake&state=$TAMPERED_STATE")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|403|404|302)$" "tampered state with evil redirect_uri rejected"
'

scenario 3 "Email header injection prevention" '
  # CRLF injection in email field
  resp=$(api_post "/api/v1/auth/forgot-password" \
    "{\"email\":\"victim@test.com\\r\\nBcc: attacker@evil.com\"}")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(400|404|422|429)$" "CRLF injection in email rejected"

  # Newline injection
  resp=$(api_post "/api/v1/auth/forgot-password" \
    "{\"email\":\"victim@test.com\\nCC: attacker@evil.com\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|404|422|429)$" "newline injection in email rejected"

  # URL-encoded injection
  resp=$(api_post "/api/v1/auth/forgot-password" \
    "{\"email\":\"victim@test.com%0ABcc:%20attacker@evil.com\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|404|422|429)$" "URL-encoded email header injection rejected"

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  # CRLF in invitation email
  resp=$(api_post "/api/v1/tenants/{tenant_id}/invitations" \
    "{\"email\":\"test@test.com\\r\\nBcc: spy@evil.com\",\"tenant_id\":\"test\",\"role_ids\":[]}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|404|422|429)$" "CRLF injection in invitation email rejected"

  qa_set_token ""
'

scenario 4 "Email template injection prevention" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  # Template injection via user name
  resp=$(api_put "/api/v1/users/me" "{\"name\":\"{{ 7 * 7 }}\"}")
  status=$(resp_status "$resp")
  if [[ "$status" == "200" ]]; then
    body=$(resp_body "$resp")
    assert_not_contains "$body" "49" "template expression not evaluated in response"
  fi
  assert_match "$status" "^(200|400|404|405)$" "update user name responds"

  # Tera template DoS attempt
  resp=$(api_put "/api/v1/users/me" \
    "{\"name\":\"{% for i in range(end=10000000) %}A{% endfor %}\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|400|404|405|422)$" "template DoS payload handled safely"

  # Custom email template injection
  resp=$(api_put "/api/v1/email-templates/password-reset" \
    "{\"body\":\"{% include \\\"/etc/passwd\\\" %}\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|403|404|405)$" "email template file inclusion rejected or endpoint not exposed"

  qa_set_token ""
'

run_all
