#!/usr/bin/env bash
# Security Auto Test: security/advanced-attacks/06-http-smuggling
# Doc: docs/security/advanced-attacks/06-http-smuggling.md
# Scenarios: 2
# ASVS: M-ADV-06 | V4.3, V12.2, V13.3
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

# ── Scenario 1: CL-TE / TE-CL smuggling attack ──────────────────────────
scenario 1 "CL-TE / TE-CL smuggling attack" '
  resp_clte=$(api_raw POST "/api/v1/auth/token" \
    -H "Content-Length: 13" \
    -H "Transfer-Encoding: chunked" \
    -d "0

G")
  status_clte=$(resp_status "$resp_clte")
  assert_match "$status_clte" "^(400|411|415|422)$" "CL-TE conflict request rejected or handled safely"

  resp_tecl=$(api_raw POST "/api/v1/auth/token" \
    -H "Content-Length: 3" \
    -H "Transfer-Encoding: chunked" \
    -d "SMUGGLED")
  status_tecl=$(resp_status "$resp_tecl")
  assert_match "$status_tecl" "^(400|411|415|422)$" "TE-CL conflict request rejected or handled safely"

  resp_double_cl=$(api_raw POST "/api/v1/auth/token" \
    -H "Content-Length: 5" \
    -H "Content-Length: 100" \
    -d "test=1")
  status_double_cl=$(resp_status "$resp_double_cl")
  assert_match "$status_double_cl" "^(400|411|415|422)$" "double Content-Length rejected"

  resp_obfuscated=$(api_raw POST "/api/v1/auth/token" \
    -H "Transfer-Encoding: chunked" \
    -H "Transfer-Encoding: x" \
    -d "0

")
  status_obfuscated=$(resp_status "$resp_obfuscated")
  assert_match "$status_obfuscated" "^(400|411|415|422|200)$" "obfuscated TE handled safely"

  resp_health=$(api_get "/health")
  status_health=$(resp_status "$resp_health")
  assert_eq "$status_health" "200" "service healthy after smuggling attempts"
'

# ── Scenario 2: HTTP/2 downgrade attack ───────────────────────────────────
scenario 2 "HTTP/2 downgrade attack" '
  if ! curl --http2 -s -o /dev/null -w "%{http_code}" "${API_BASE}/health" 2>/dev/null | grep -q "200\|301\|302"; then
    assert_eq "skip" "skip" "HTTP/2 not supported or curl lacks --http2"
    return 0
  fi

  resp_h2te=$(curl -s -o /dev/null -w "%{http_code}" --http2 \
    -X POST \
    -H "Transfer-Encoding: chunked" \
    -H "Content-Type: application/json" \
    -d "{}" \
    "${API_BASE}/api/v1/auth/token" 2>/dev/null) || resp_h2te="000"
  assert_match "$resp_h2te" "^(400|415|422|200)$" "HTTP/2 with Transfer-Encoding handled safely"

  resp_traversal=$(curl -s -o /dev/null -w "%{http_code}" --http2 \
    "${API_BASE}/api/v1/../internal/debug" 2>/dev/null) || resp_traversal="000"
  assert_match "$resp_traversal" "^(400|403|404)$" "HTTP/2 path traversal normalized to 404"

  resp_long_hdr=$(curl -s -o /dev/null -w "%{http_code}" --http2 \
    -H "X-Long-Header: $(python3 -c "print('\''A'\'' * 65536)" 2>/dev/null || printf "A%.0s" $(seq 1 10000))" \
    "${API_BASE}/health" 2>/dev/null) || resp_long_hdr="000"
  assert_match "$resp_long_hdr" "^(200|400|413|431)$" "HTTP/2 oversized header handled"

  resp_health=$(api_get "/health")
  status_health=$(resp_status "$resp_health")
  assert_eq "$status_health" "200" "service healthy after HTTP/2 downgrade tests"
'

run_all
